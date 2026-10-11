//! Strict delimited-text support used by the language's CSV built-ins.

/// Parses one CSV document. Quoted fields support doubled quotes and embedded
/// LF or CRLF newlines. A UTF-8 BOM is ignored only at the start of a file.
pub(crate) fn decode(input: &str, delimiter: char) -> Result<Vec<Vec<String>>, String> {
    if delimiter == '"' || delimiter == '\r' || delimiter == '\n' {
        return Err("CSV delimiter cannot be a quote or newline".into());
    }
    if input.is_empty() {
        return Ok(Vec::new());
    }

    let mut chars = input.trim_start_matches('\u{feff}').chars().peekable();
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut field = String::new();
    let mut quoted = false;
    let mut after_quote = false;
    let mut at_field_start = true;
    let mut row_started = false;
    let mut line = 1usize;

    while let Some(character) = chars.next() {
        if quoted {
            match character {
                '"' if chars.peek() == Some(&'"') => {
                    chars.next();
                    field.push('"');
                }
                '"' => {
                    quoted = false;
                    after_quote = true;
                }
                '\r' if chars.peek() == Some(&'\n') => {
                    chars.next();
                    field.push_str("\r\n");
                    line += 1;
                }
                '\n' => {
                    field.push('\n');
                    line += 1;
                }
                '\r' => {
                    field.push('\r');
                }
                other => field.push(other),
            }
            continue;
        }

        if after_quote {
            match character {
                value if value == delimiter => {
                    row.push(std::mem::take(&mut field));
                    at_field_start = true;
                    after_quote = false;
                    row_started = true;
                }
                '\n' => {
                    row.push(std::mem::take(&mut field));
                    rows.push(std::mem::take(&mut row));
                    at_field_start = true;
                    after_quote = false;
                    row_started = false;
                    line += 1;
                }
                '\r' if chars.peek() == Some(&'\n') => {
                    chars.next();
                    row.push(std::mem::take(&mut field));
                    rows.push(std::mem::take(&mut row));
                    at_field_start = true;
                    after_quote = false;
                    row_started = false;
                    line += 1;
                }
                other => {
                    return Err(format!(
                        "unexpected character {other:?} after closing quote at line {line}"
                    ));
                }
            }
            continue;
        }

        if at_field_start && character == '"' {
            quoted = true;
            at_field_start = false;
            row_started = true;
            continue;
        }

        if character == delimiter {
            row.push(std::mem::take(&mut field));
            at_field_start = true;
            row_started = true;
        } else if character == '\n' {
            row.push(std::mem::take(&mut field));
            rows.push(std::mem::take(&mut row));
            at_field_start = true;
            row_started = false;
            line += 1;
        } else if character == '\r' && chars.peek() == Some(&'\n') {
            chars.next();
            row.push(std::mem::take(&mut field));
            rows.push(std::mem::take(&mut row));
            at_field_start = true;
            row_started = false;
            line += 1;
        } else if character == '\r' {
            return Err(format!(
                "bare carriage return outside a quoted field at line {line}"
            ));
        } else if character == '"' {
            return Err(format!(
                "quote inside an unquoted field at line {line}; quote the complete field"
            ));
        } else {
            field.push(character);
            at_field_start = false;
            row_started = true;
        }
    }

    if quoted {
        return Err(format!(
            "unterminated quoted field at end of input near line {line}"
        ));
    }
    if row_started || !row.is_empty() || !field.is_empty() || after_quote {
        row.push(field);
        rows.push(row);
    }
    Ok(rows)
}

/// Encodes fields using CSV quoting and doubled quotes where needed.
pub(crate) fn encode(rows: &[Vec<String>], delimiter: char) -> Result<String, String> {
    if delimiter == '"' || delimiter == '\r' || delimiter == '\n' {
        return Err("CSV delimiter cannot be a quote or newline".into());
    }
    if rows.iter().any(Vec::is_empty) {
        return Err("CSV rows must contain at least one field".into());
    }
    let mut output = String::new();
    for (row_index, row) in rows.iter().enumerate() {
        if row_index > 0 {
            output.push('\n');
        }
        for (field_index, value) in row.iter().enumerate() {
            if field_index > 0 {
                output.push(delimiter);
            }
            let needs_quotes = (value.is_empty() && row.len() == 1)
                || value.contains(delimiter)
                || value.contains('"')
                || value.contains('\r')
                || value.contains('\n');
            if needs_quotes {
                output.push('"');
                for character in value.chars() {
                    if character == '"' {
                        output.push_str("\"\"");
                    } else {
                        output.push(character);
                    }
                }
                output.push('"');
            } else {
                output.push_str(value);
            }
        }
    }
    Ok(output)
}

/// Converts a decimal string to an integer scaled by `scale` decimal places.
/// No binary floating point is used. Comma and dot are accepted as decimal
/// separators, but they cannot be mixed and excess fractional digits fail.
pub(crate) fn decimal_to_scaled(input: &str, scale: i64) -> Result<i64, String> {
    if !(0..=9).contains(&scale) {
        return Err("decimal scale must be between 0 and 9".into());
    }
    let input = input.trim();
    if input.is_empty() {
        return Err("value is empty".into());
    }
    let (negative, unsigned) = match input.as_bytes()[0] {
        b'-' => (true, &input[1..]),
        b'+' => (false, &input[1..]),
        _ => (false, input),
    };
    if unsigned.is_empty() {
        return Err("value has no digits".into());
    }
    let mut split = unsigned.split(['.', ',']);
    let whole = split.next().unwrap_or_default();
    let fraction = split.next();
    if split.next().is_some() {
        return Err("use one decimal separator only".into());
    }
    if fraction.is_some() && fraction == Some("") {
        return Err("fractional digits must follow the decimal separator".into());
    }
    if whole.is_empty() || !whole.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err("whole part must contain digits only".into());
    }
    let fraction = fraction.unwrap_or_default();
    if !fraction.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err("fractional part must contain digits only".into());
    }
    if fraction.len() > scale as usize {
        return Err(format!("at most {scale} fractional digits are allowed"));
    }

    let factor = 10_i128.pow(scale as u32);
    let whole = whole
        .parse::<i128>()
        .map_err(|_| "whole part is outside the supported range".to_string())?;
    let mut fractional = if fraction.is_empty() {
        0
    } else {
        fraction
            .parse::<i128>()
            .map_err(|_| "fractional part is outside the supported range".to_string())?
    };
    for _ in fraction.len()..scale as usize {
        fractional *= 10;
    }
    let magnitude = whole
        .checked_mul(factor)
        .and_then(|value| value.checked_add(fractional))
        .ok_or_else(|| "value is outside the supported range".to_string())?;
    let value = if negative { -magnitude } else { magnitude };
    i64::try_from(value).map_err(|_| "value is outside the supported Int range".into())
}

/// Formats a scaled integer as a fixed-point decimal string.
pub(crate) fn format_scaled(value: i64, scale: i64) -> Result<String, String> {
    if !(0..=9).contains(&scale) {
        return Err("decimal scale must be between 0 and 9".into());
    }
    if scale == 0 {
        return Ok(value.to_string());
    }
    let factor = 10_i64.pow(scale as u32);
    let magnitude = i128::from(value).abs();
    let factor = i128::from(factor);
    let sign = if value < 0 { "-" } else { "" };
    Ok(format!(
        "{sign}{}.{:0width$}",
        magnitude / factor,
        magnitude % factor,
        width = scale as usize
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv_round_trip_preserves_quoted_fields_and_newlines() {
        let rows = vec![
            vec!["number".into(), "name".into(), "note".into()],
            vec![
                "42".into(),
                "Äpfel; \"".into(),
                "erste Zeile\nzweite Zeile".into(),
            ],
        ];
        let encoded = encode(&rows, ';').unwrap();
        assert_eq!(decode(&encoded, ';').unwrap(), rows);
        assert_eq!(
            decode("left;right\r\n1;2\r\n", ';').unwrap(),
            [["left", "right"], ["1", "2"]]
        );
        assert_eq!(
            decode(&encode(&[vec![String::new()]], ';').unwrap(), ';').unwrap(),
            [[""]]
        );
        assert!(encode(&[Vec::new()], ';').is_err());
    }

    #[test]
    fn csv_reports_malformed_quote_structures_and_ignores_a_bom() {
        assert!(decode("a;b\n\"broken", ';')
            .unwrap_err()
            .contains("unterminated quoted field"));
        assert!(decode("a;b\nplain\"quote;x", ';')
            .unwrap_err()
            .contains("quote inside an unquoted field"));
        assert!(decode("a\rb", ';')
            .unwrap_err()
            .contains("bare carriage return"));
        assert_eq!(decode("\u{feff}a;b\n1;2", ';').unwrap()[0], ["a", "b"]);
    }

    #[test]
    fn fixed_point_conversion_is_exact_and_rejects_excess_precision() {
        assert_eq!(decimal_to_scaled("12.34", 2), Ok(1234));
        assert_eq!(decimal_to_scaled("12,3", 2), Ok(1230));
        assert_eq!(decimal_to_scaled("-0.05", 2), Ok(-5));
        assert!(decimal_to_scaled("1.001", 2).is_err());
        assert!(decimal_to_scaled("1.", 2).is_err());
        assert!(decimal_to_scaled("1.2,3", 2).is_err());
        assert_eq!(format_scaled(1234, 2), Ok("12.34".into()));
        assert_eq!(format_scaled(-5, 2), Ok("-0.05".into()));
    }
}
