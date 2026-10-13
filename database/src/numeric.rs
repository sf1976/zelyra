//! Exact PostgreSQL NUMERIC wire decoding and type-independent SQL NULL.
//! PostgreSQL's numeric_send uses big-endian base-10000 digit groups.

use bytes::BytesMut;
use postgres::types::{FromSql, IsNull, ToSql, Type};
use std::{error::Error, fmt, fmt::Write, io};

type CodecError = Box<dyn Error + Send + Sync>;

#[derive(Debug)]
pub(crate) struct NumericText(String);

impl fmt::Display for NumericText {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl<'a> FromSql<'a> for NumericText {
    fn from_sql(_: &Type, raw: &'a [u8]) -> Result<Self, CodecError> {
        decode(raw).map(Self)
    }

    fn accepts(ty: &Type) -> bool {
        *ty == Type::NUMERIC
    }
}

#[derive(Debug)]
pub(crate) struct SqlNull;

impl ToSql for SqlNull {
    fn to_sql(&self, _: &Type, _: &mut BytesMut) -> Result<IsNull, CodecError> {
        Ok(IsNull::Yes)
    }

    fn accepts(_: &Type) -> bool {
        true
    }

    postgres::types::to_sql_checked!();
}

fn invalid() -> CodecError {
    io::Error::new(
        io::ErrorKind::InvalidData,
        "invalid or nonfinite PostgreSQL NUMERIC result",
    )
    .into()
}

fn decode(raw: &[u8]) -> Result<String, CodecError> {
    if raw.len() < 8 {
        return Err(invalid());
    }
    let word = |offset| u16::from_be_bytes([raw[offset], raw[offset + 1]]);
    let count = usize::from(word(0));
    let weight = i32::from(word(2) as i16);
    let sign = word(4);
    let scale = usize::from(word(6));
    if raw.len() != 8 + count * 2 || !matches!(sign, 0 | 0x4000) || scale > 16383 {
        return Err(invalid());
    }
    let digits = (0..count)
        .map(|index| word(8 + index * 2))
        .collect::<Vec<_>>();
    if digits.iter().any(|digit| *digit >= 10000) {
        return Err(invalid());
    }
    let fraction_groups = scale.div_ceil(4) as i32;
    // Never hide nonzero digits beyond the declared display scale.
    for (index, digit) in digits.iter().enumerate() {
        let exponent = weight - index as i32;
        if exponent < -fraction_groups && *digit != 0
            || scale % 4 != 0
                && exponent == -fraction_groups
                && digit % 10_u16.pow((4 - scale % 4) as u32) != 0
        {
            return Err(invalid());
        }
    }
    let group = |exponent: i32| {
        usize::try_from(weight - exponent)
            .ok()
            .and_then(|index| digits.get(index))
            .copied()
            .unwrap_or(0)
    };
    // Header limits bound the representation to < 148 KB, even with sparse digits.
    let mut result = String::new();
    if sign == 0x4000 && digits.iter().any(|digit| *digit != 0) {
        result.push('-');
    }
    if weight < 0 {
        result.push('0');
    } else {
        let mut started = false;
        for exponent in (0..=weight).rev() {
            let digit = group(exponent);
            if started {
                write!(&mut result, "{digit:04}")?;
            } else if digit != 0 {
                write!(&mut result, "{digit}")?;
                started = true;
            }
        }
        if !started {
            result.push('0');
        }
    }
    if scale > 0 {
        result.push('.');
        let fraction_start = result.len();
        for index in 1..=fraction_groups {
            write!(&mut result, "{:04}", group(-index))?;
        }
        result.truncate(fraction_start + scale);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wire(weight: i16, sign: u16, scale: u16, digits: &[u16]) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&(digits.len() as u16).to_be_bytes());
        bytes.extend_from_slice(&weight.to_be_bytes());
        bytes.extend_from_slice(&sign.to_be_bytes());
        bytes.extend_from_slice(&scale.to_be_bytes());
        for digit in digits {
            bytes.extend_from_slice(&digit.to_be_bytes());
        }
        bytes
    }

    #[test]
    fn numeric_wire_preserves_digits_sign_and_scale() {
        assert_eq!(decode(&wire(0, 0, 2, &[12, 3400])).unwrap(), "12.34");
        assert_eq!(
            decode(&wire(0, 0x4000, 4, &[12, 3400])).unwrap(),
            "-12.3400"
        );
        assert_eq!(decode(&wire(-3, 0, 9, &[1000])).unwrap(), "0.000000001");
        assert_eq!(
            decode(&wire(4, 0, 0, &[1844, 6744, 737, 955, 1615])).unwrap(),
            "18446744073709551615"
        );
        assert_eq!(decode(&wire(0, 0, 9, &[])).unwrap(), "0.000000000");
        assert_eq!(decode(&wire(2, 0, 2, &[12])).unwrap(), "1200000000.00");
    }

    #[test]
    fn malformed_and_nonfinite_numeric_results_are_controlled_errors() {
        for raw in [
            vec![],
            vec![0; 7],
            wire(0, 0xc000, 0, &[]),
            wire(0, 0xd000, 0, &[]),
            wire(0, 0xf000, 0, &[]),
            wire(0, 0, 0xffff, &[]),
            wire(0, 0, 2, &[10000]),
            wire(0, 1, 2, &[1]),
            wire(-1, 0, 2, &[1234]),
        ] {
            assert!(decode(&raw).is_err());
        }
        let mut truncated = wire(0, 0, 2, &[12, 3400]);
        truncated.pop();
        assert!(decode(&truncated).is_err());
    }

    #[test]
    fn null_can_bind_to_numeric_and_other_nullable_columns() {
        for ty in [Type::NUMERIC, Type::INT8, Type::BOOL, Type::TEXT] {
            let mut bytes = BytesMut::new();
            assert!(matches!(
                SqlNull.to_sql_checked(&ty, &mut bytes).unwrap(),
                IsNull::Yes
            ));
            assert!(bytes.is_empty());
        }
    }
}
