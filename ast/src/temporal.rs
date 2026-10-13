//! Canonical Gregorian dates, clock times, and UTC millisecond timestamps.

pub fn date_epoch_days(value: &str) -> Option<i64> {
    if !valid_date(value) {
        return None;
    }
    let (year, month_day) = value.split_once('-')?;
    let (month, day) = month_day.split_once('-')?;
    let mut year = year.parse::<i64>().ok()?;
    let month = month.parse::<i64>().ok()?;
    let day = day.parse::<i64>().ok()?;
    year -= i64::from(month <= 2);
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let shifted_month = month + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * shifted_month + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    Some(era * 146_097 + day_of_era - 719_468)
}

pub fn epoch_days_date(days: i64) -> Option<String> {
    let shifted = days.checked_add(719_468)?;
    let era = shifted.div_euclid(146_097);
    let day_of_era = shifted - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    if !(1..=9_999).contains(&year) {
        return None;
    }
    Some(format!("{year:04}-{month:02}-{day:02}"))
}

pub fn timestamp_bounds_millis() -> Option<(i64, i64)> {
    let first = date_epoch_days("0001-01-01")?.checked_mul(86_400_000)?;
    let last = date_epoch_days("9999-12-31")?
        .checked_add(1)?
        .checked_mul(86_400_000)?
        .checked_sub(1)?;
    Some((first, last))
}

pub fn valid_date(value: &str) -> bool {
    let Some((year, month_day)) = value.split_once('-') else {
        return false;
    };
    let Some((month, day)) = month_day.split_once('-') else {
        return false;
    };
    if year.len() != 4
        || month.len() != 2
        || day.len() != 2
        || !year.bytes().all(|byte| byte.is_ascii_digit())
        || !month.bytes().all(|byte| byte.is_ascii_digit())
        || !day.bytes().all(|byte| byte.is_ascii_digit())
    {
        return false;
    }
    let (Ok(year), Ok(month), Ok(day)) = (
        year.parse::<u32>(),
        month.parse::<u32>(),
        day.parse::<u32>(),
    ) else {
        return false;
    };
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return false,
    };
    year > 0 && (1..=days).contains(&day)
}

pub fn valid_time(value: &str) -> bool {
    let mut parts = value.split(':');
    let (Some(hour), Some(minute), Some(second), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return false;
    };
    [hour, minute, second]
        .iter()
        .all(|part| part.len() == 2 && part.bytes().all(|byte| byte.is_ascii_digit()))
        && hour.parse::<u32>().is_ok_and(|value| value <= 23)
        && minute.parse::<u32>().is_ok_and(|value| value <= 59)
        && second.parse::<u32>().is_ok_and(|value| value <= 59)
}

pub fn format_timestamp_utc(timestamp_millis: i64) -> String {
    let seconds = timestamp_millis.div_euclid(1_000);
    let millis = timestamp_millis.rem_euclid(1_000);
    let days = seconds.div_euclid(86_400);
    let seconds_in_day = seconds.rem_euclid(86_400);

    let shifted_days = days + 719_468;
    let era = shifted_days.div_euclid(146_097);
    let day_of_era = shifted_days - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_part = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_part + 2) / 5 + 1;
    let month = if month_part < 10 {
        month_part + 3
    } else {
        month_part - 9
    };
    if month <= 2 {
        year += 1;
    }

    let hour = seconds_in_day / 3_600;
    let minute = (seconds_in_day % 3_600) / 60;
    let second = seconds_in_day % 60;
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{millis:03}Z")
}

pub fn valid_timestamp_millis(value: i64) -> bool {
    timestamp_bounds_millis().is_some_and(|(min, max)| (min..=max).contains(&value))
}

/// HTTP/form timestamp input: exact Unix milliseconds or an explicit UTC ISO
/// timestamp. Offset zones and local wall-clock timestamps are not inferred.
pub fn parse_timestamp_input(value: &str) -> Option<i64> {
    if let Ok(millis) = value.parse::<i64>() {
        return valid_timestamp_millis(millis).then_some(millis);
    }
    let text = value.strip_suffix('Z')?;
    let (date, clock) = text.split_once('T')?;
    let (time, fraction) = clock.split_once('.').unwrap_or((clock, ""));
    if !valid_date(date)
        || !valid_time(time)
        || fraction.len() > 3
        || !fraction.bytes().all(|byte| byte.is_ascii_digit())
        || clock.contains('.') && fraction.is_empty()
    {
        return None;
    }
    let milliseconds = if fraction.is_empty() {
        0
    } else {
        fraction.parse::<i64>().ok()? * 10_i64.pow(3 - fraction.len() as u32)
    };
    let parts = time
        .split(':')
        .map(str::parse::<i64>)
        .collect::<Result<Vec<_>, _>>()
        .ok()?;
    let day_millis = (parts[0] * 3600 + parts[1] * 60 + parts[2]) * 1000 + milliseconds;
    let epoch = date_epoch_days(date)?
        .checked_mul(86_400_000)?
        .checked_add(day_millis)?;
    valid_timestamp_millis(epoch).then_some(epoch)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utc_input_preserves_milliseconds_and_rejects_invalid_or_ambiguous_times() {
        assert_eq!(parse_timestamp_input("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(parse_timestamp_input("1970-01-01T00:00:00.1Z"), Some(100));
        assert_eq!(parse_timestamp_input("1969-12-31T23:59:59.999Z"), Some(-1));
        assert_eq!(
            parse_timestamp_input("0001-01-01T00:00:00.000Z"),
            Some(-62135596800000)
        );
        assert_eq!(
            parse_timestamp_input("9999-12-31T23:59:59.999Z"),
            Some(253402300799999)
        );
        for value in [
            "1900-02-29T00:00:00Z",
            "2026-01-01T24:00:00Z",
            "2026-01-01T23:59:60Z",
            "1970-01-01T00:00:00.0001Z",
            "1970-01-01T00:00:00.Z",
            "2026-01-01T12:00:00",
            "2026-01-01T12:00:00+00:00",
            "9223372036854775807",
        ] {
            assert_eq!(parse_timestamp_input(value), None, "accepted {value}");
        }
    }
}
