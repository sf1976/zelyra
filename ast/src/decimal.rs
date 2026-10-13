//! Shared exact parsing for the language and its form/HTTP/SQL boundaries.

pub const SCALE_DIGITS: u32 = 9;

/// Parse a signed fixed-point decimal without a floating-point intermediate.
/// Exponents, whitespace, a leading plus, and excess fractional digits fail.
pub fn parse_scaled(value: &str, scale: u32) -> Option<i128> {
    if scale > SCALE_DIGITS {
        return None;
    }
    let (negative, unsigned) = value
        .strip_prefix('-')
        .map_or((false, value), |v| (true, v));
    let (whole, fraction) = unsigned.split_once('.').unwrap_or((unsigned, ""));
    if whole.is_empty()
        || !whole.bytes().all(|byte| byte.is_ascii_digit())
        || fraction.len() > scale as usize
        || !fraction.bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    let factor = 10_u128.checked_pow(scale)?;
    let whole = whole.parse::<u128>().ok()?.checked_mul(factor)?;
    let fractional = if fraction.is_empty() {
        0
    } else {
        fraction
            .parse::<u128>()
            .ok()?
            .checked_mul(10_u128.checked_pow(scale - fraction.len() as u32)?)?
    };
    let magnitude = whole.checked_add(fractional)?;
    if negative && magnitude == (i128::MAX as u128) + 1 {
        Some(i128::MIN)
    } else {
        let signed = i128::try_from(magnitude).ok()?;
        if negative {
            signed.checked_neg()
        } else {
            Some(signed)
        }
    }
}

/// Money columns use DECIMAL/NUMERIC(19,4); return exact ten-thousandths.
pub fn parse_money(value: &str) -> Option<i128> {
    parse_scaled(value, 4).filter(|amount| amount.unsigned_abs() <= 9_999_999_999_999_999_999)
}

/// JSON numbers may use exponents or insignificant fractional zeroes. Expand
/// them exactly and reject any value requiring rounding at the requested scale.
pub fn parse_json_number(value: &str, scale: u32) -> Option<i128> {
    if scale > SCALE_DIGITS {
        return None;
    }
    // Validate JSON grammar before interpreting mantissa/exponent, including zero.
    value.parse::<serde_json::Number>().ok()?;
    let (negative, value) = value
        .strip_prefix('-')
        .map_or((false, value), |v| (true, v));
    let (mantissa, exponent) = value.split_once(['e', 'E']).unwrap_or((value, "0"));
    let (whole, fraction) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    if whole.is_empty()
        || !whole.bytes().all(|byte| byte.is_ascii_digit())
        || !fraction.bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    let digits = format!("{whole}{fraction}");
    let digits = digits.trim_start_matches('0');
    if digits.is_empty() {
        return Some(0);
    }
    let significant = digits.trim_end_matches('0');
    let trailing = i64::try_from(digits.len() - significant.len()).ok()?;
    let power = i64::from(scale)
        .checked_add(exponent.parse::<i64>().ok()?)?
        .checked_sub(i64::try_from(fraction.len()).ok()?)?
        .checked_add(trailing)?;
    if !(0..=38).contains(&power) {
        return None;
    }
    let magnitude = significant
        .parse::<u128>()
        .ok()?
        .checked_mul(10_u128.checked_pow(power as u32)?)?;
    if negative && magnitude == (i128::MAX as u128) + 1 {
        Some(i128::MIN)
    } else {
        let value = i128::try_from(magnitude).ok()?;
        if negative {
            value.checked_neg()
        } else {
            Some(value)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decimal_boundaries_cover_both_signed_extremes() {
        assert_eq!(
            parse_scaled("170141183460469231731687303715.884105727", 9),
            Some(i128::MAX)
        );
        assert_eq!(
            parse_scaled("-170141183460469231731687303715.884105728", 9),
            Some(i128::MIN)
        );
        assert_eq!(
            parse_scaled("170141183460469231731687303715.884105728", 9),
            None
        );
        assert_eq!(
            parse_scaled("-170141183460469231731687303715.884105729", 9),
            None
        );
        assert_eq!(parse_scaled("0.000000001", 9), Some(1));
    }

    #[test]
    fn money_preserves_its_four_digit_storage_contract() {
        assert_eq!(
            parse_money("999999999999999.9999"),
            Some(9_999_999_999_999_999_999)
        );
        assert_eq!(parse_money("-12.34"), Some(-123400));
        assert_eq!(parse_money("0001.2"), Some(12000));
        for value in [
            "",
            "-",
            ".5",
            "+1",
            " 1",
            "1e3",
            "NaN",
            "inf",
            "1.00001",
            "1000000000000000",
        ] {
            assert_eq!(parse_money(value), None, "accepted {value}");
        }
    }

    #[test]
    fn json_exponents_and_zeroes_do_not_require_binary_rounding() {
        assert_eq!(parse_json_number("1e3", 9), Some(1_000_000_000_000));
        assert_eq!(parse_json_number("1e-9", 9), Some(1));
        assert_eq!(parse_json_number("1.23000000000", 9), Some(1_230_000_000));
        assert_eq!(
            parse_json_number("100000000000000000000000000000000000000000000000000e-50", 9),
            Some(1_000_000_000)
        );
        assert_eq!(parse_json_number("1e-10", 9), None);
        assert_eq!(parse_json_number("1e999", 9), None);
        for invalid in ["0e", "00", "1.", "+1", ".5", "NaN"] {
            assert_eq!(parse_json_number(invalid, 9), None);
        }
    }
}
