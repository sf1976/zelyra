//! Validated native temporal parameters and exact PostgreSQL binary results.
use bytes::BytesMut;
use postgres::types::{FromSql, IsNull, ToSql, Type};
use std::{error::Error, fmt, io};
use zelyra_ast::temporal::*;

type CodecError = Box<dyn Error + Send + Sync>;
const PG_EPOCH_DAYS: i64 = 10957;
const PG_EPOCH_MILLIS: i64 = 946684800000;

#[derive(Debug)]
pub(crate) enum Parameter {
    Date(String),
    Time(String),
    Timestamp(i64),
}

#[derive(Debug)]
pub(crate) struct TemporalText(String);
impl fmt::Display for TemporalText {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        out.write_str(&self.0)
    }
}

fn invalid() -> CodecError {
    io::Error::new(
        io::ErrorKind::InvalidData,
        "invalid, out-of-range, or unsupported precision in SQL temporal value",
    )
    .into()
}

impl ToSql for Parameter {
    fn to_sql(&self, ty: &Type, out: &mut BytesMut) -> Result<IsNull, CodecError> {
        match (self, ty) {
            (Self::Date(value), &Type::DATE) => {
                let days = date_epoch_days(value).ok_or_else(invalid)? - PG_EPOCH_DAYS;
                out.extend_from_slice(&i32::try_from(days)?.to_be_bytes());
            }
            (Self::Time(value), &Type::TIME) => {
                out.extend_from_slice(&time_micros(value).ok_or_else(invalid)?.to_be_bytes());
            }
            (Self::Timestamp(value), &Type::TIMESTAMPTZ) if valid_timestamp_millis(*value) => {
                let micros = value
                    .checked_sub(PG_EPOCH_MILLIS)
                    .and_then(|value| value.checked_mul(1000))
                    .ok_or_else(invalid)?;
                out.extend_from_slice(&micros.to_be_bytes());
            }
            _ => return Err(invalid()),
        }
        Ok(IsNull::No)
    }
    fn accepts(ty: &Type) -> bool {
        matches!(*ty, Type::DATE | Type::TIME | Type::TIMESTAMPTZ)
    }
    postgres::types::to_sql_checked!();
}
impl<'a> FromSql<'a> for TemporalText {
    fn from_sql(ty: &Type, raw: &'a [u8]) -> Result<Self, CodecError> {
        let text = match *ty {
            Type::DATE => {
                let days = i32::from_be_bytes(raw.try_into().map_err(|_| invalid())?);
                epoch_days_date(i64::from(days) + PG_EPOCH_DAYS).ok_or_else(invalid)?
            }
            Type::TIME => {
                let micros = i64::from_be_bytes(raw.try_into().map_err(|_| invalid())?);
                if !(0..86_400_000_000).contains(&micros) || micros % 1_000_000 != 0 {
                    return Err(invalid());
                }
                let seconds = micros / 1_000_000;
                format!(
                    "{:02}:{:02}:{:02}",
                    seconds / 3600,
                    seconds / 60 % 60,
                    seconds % 60
                )
            }
            Type::TIMESTAMPTZ => {
                let micros = i64::from_be_bytes(raw.try_into().map_err(|_| invalid())?);
                format_postgres_timestamp(micros, true)?
            }
            Type::TIMESTAMP => {
                let micros = i64::from_be_bytes(raw.try_into().map_err(|_| invalid())?);
                format_postgres_timestamp(micros, false)?
            }
            _ => return Err(invalid()),
        };
        Ok(Self(text))
    }
    fn accepts(ty: &Type) -> bool {
        matches!(
            *ty,
            Type::DATE | Type::TIME | Type::TIMESTAMP | Type::TIMESTAMPTZ
        )
    }
}

fn format_postgres_timestamp(micros: i64, with_timezone: bool) -> Result<String, CodecError> {
    let millis = micros
        .div_euclid(1000)
        .checked_add(PG_EPOCH_MILLIS)
        .filter(|value| valid_timestamp_millis(*value))
        .ok_or_else(invalid)?;
    let formatted = format_timestamp_utc(millis);
    let without_zone = formatted.strip_suffix('Z').ok_or_else(invalid)?;
    if without_zone.len() < 4 {
        return Err(invalid());
    }
    // `format_timestamp_utc` always emits milliseconds. Replace that fraction
    // with the exact six-digit fraction from PostgreSQL's binary value.
    let timestamp = &without_zone[..without_zone.len() - 4];
    let fraction = micros.rem_euclid(1_000_000);
    let fraction = if fraction % 1000 == 0 {
        format!("{:03}", fraction / 1000)
    } else {
        format!("{fraction:06}")
    };
    Ok(if with_timezone {
        format!("{timestamp}.{fraction}Z")
    } else {
        format!("{timestamp}.{fraction}")
    })
}

fn time_micros(value: &str) -> Option<i64> {
    if !valid_time(value) {
        return None;
    }
    let parts = value
        .split(':')
        .map(str::parse::<i64>)
        .collect::<Result<Vec<_>, _>>()
        .ok()?;
    Some((parts[0] * 3600 + parts[1] * 60 + parts[2]) * 1_000_000)
}

pub(crate) fn mysql_parameter(value: &Parameter) -> Result<mysql::Value, CodecError> {
    Ok(match value {
        Parameter::Date(value) if valid_date(value) && value.as_str() >= "1000-01-01" => {
            mysql::Value::Date(
                value[..4].parse()?,
                value[5..7].parse()?,
                value[8..10].parse()?,
                0,
                0,
                0,
                0,
            )
        }
        Parameter::Time(value) => {
            let micros = time_micros(value).ok_or_else(invalid)?;
            let seconds = micros / 1_000_000;
            mysql::Value::Time(
                false,
                0,
                (seconds / 3600) as u8,
                (seconds / 60 % 60) as u8,
                (seconds % 60) as u8,
                0,
            )
        }
        Parameter::Timestamp(value) if valid_timestamp_millis(*value) => {
            let formatted = format_timestamp_utc(*value);
            if formatted.as_str() < "1000-01-01" {
                return Err(invalid());
            }
            mysql::Value::Date(
                formatted[..4].parse()?,
                formatted[5..7].parse()?,
                formatted[8..10].parse()?,
                formatted[11..13].parse()?,
                formatted[14..16].parse()?,
                formatted[17..19].parse()?,
                formatted[20..23].parse::<u32>()? * 1000,
            )
        }
        _ => return Err(invalid()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_postgres_temporal_values_round_trip_without_precision_loss() {
        for (param, ty, expected) in [
            (
                Parameter::Date("2000-01-01".into()),
                Type::DATE,
                "2000-01-01",
            ),
            (
                Parameter::Date("0001-01-01".into()),
                Type::DATE,
                "0001-01-01",
            ),
            (Parameter::Time("23:59:59".into()), Type::TIME, "23:59:59"),
            (Parameter::Time("00:00:00".into()), Type::TIME, "00:00:00"),
            (
                Parameter::Timestamp(-62135596800000),
                Type::TIMESTAMPTZ,
                "0001-01-01T00:00:00.000Z",
            ),
            (
                Parameter::Timestamp(253402300799999),
                Type::TIMESTAMPTZ,
                "9999-12-31T23:59:59.999Z",
            ),
            (
                Parameter::Timestamp(-1),
                Type::TIMESTAMPTZ,
                "1969-12-31T23:59:59.999Z",
            ),
            (
                Parameter::Timestamp(946684800001),
                Type::TIMESTAMPTZ,
                "2000-01-01T00:00:00.001Z",
            ),
        ] {
            let mut buffer = BytesMut::new();
            assert!(matches!(
                param.to_sql_checked(&ty, &mut buffer).unwrap(),
                IsNull::No
            ));
            assert_eq!(
                TemporalText::from_sql(&ty, &buffer).unwrap().to_string(),
                expected
            );
        }
        let one_microsecond = 1_i64.to_be_bytes();
        assert_eq!(
            TemporalText::from_sql(&Type::TIMESTAMPTZ, &one_microsecond)
                .unwrap()
                .to_string(),
            "2000-01-01T00:00:00.000001Z"
        );
        assert_eq!(
            TemporalText::from_sql(&Type::TIMESTAMP, &one_microsecond)
                .unwrap()
                .to_string(),
            "2000-01-01T00:00:00.000001"
        );
        for (ty, bytes) in [
            (Type::DATE, i32::MAX.to_be_bytes().to_vec()),
            (Type::TIME, 86_400_000_000_i64.to_be_bytes().to_vec()),
            (Type::TIME, 1_i64.to_be_bytes().to_vec()),
            (Type::TIMESTAMPTZ, i64::MAX.to_be_bytes().to_vec()),
            (Type::DATE, vec![]),
        ] {
            assert!(TemporalText::from_sql(&ty, &bytes).is_err());
        }
    }

    #[test]
    fn native_parameters_reject_invalid_values_and_backend_range_mismatches() {
        for (parameter, ty) in [
            (Parameter::Date("1900-02-29".into()), Type::DATE),
            (Parameter::Time("24:00:00".into()), Type::TIME),
            (Parameter::Time("12:34:56.001".into()), Type::TIME),
            (Parameter::Timestamp(-62135596800001), Type::TIMESTAMPTZ),
            (Parameter::Timestamp(253402300800000), Type::TIMESTAMPTZ),
            (Parameter::Date("2000-02-29".into()), Type::TIME),
        ] {
            assert!(parameter.to_sql_checked(&ty, &mut BytesMut::new()).is_err());
        }
        for parameter in [
            Parameter::Date("0999-12-31".into()),
            Parameter::Date("2000-02-30".into()),
            Parameter::Time("-01:00:00".into()),
            Parameter::Timestamp(-62135596800000),
            Parameter::Timestamp(253402300800000),
        ] {
            assert!(mysql_parameter(&parameter).is_err());
        }
        for parameter in [
            Parameter::Date("1000-01-01".into()),
            Parameter::Date("9999-12-31".into()),
            Parameter::Time("00:00:00".into()),
            Parameter::Time("23:59:59".into()),
            Parameter::Timestamp(-1),
            Parameter::Timestamp(253402300799999),
        ] {
            assert!(mysql_parameter(&parameter).is_ok());
        }
    }
}
