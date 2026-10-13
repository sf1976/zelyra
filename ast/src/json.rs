//! Lossless JSON values, including object keys reserved by serde's numeric bridge.

use serde_json::{value::RawValue, Error, Map, Number, Value};
use std::{collections::BTreeMap, io, str::FromStr};

pub fn parse(input: &str) -> Result<Value, Error> {
    let raw: Box<RawValue> = serde_json::from_str(input)?;
    decode(&raw, 0)
}

pub fn parse_slice(input: &[u8]) -> Result<Value, Error> {
    let raw: Box<RawValue> = serde_json::from_slice(input)?;
    decode(&raw, 0)
}

fn decode(raw: &RawValue, depth: usize) -> Result<Value, Error> {
    if depth >= 128 {
        return Err(Error::io(io::Error::new(
            io::ErrorKind::InvalidData,
            "JSON nesting limit exceeded",
        )));
    }
    let text = raw.get().trim();
    match text.as_bytes().first() {
        Some(b'{') => {
            let fields: BTreeMap<String, Box<RawValue>> = serde_json::from_str(text)?;
            let mut object = Map::new();
            for (key, value) in fields {
                object.insert(key, decode(&value, depth + 1)?);
            }
            Ok(Value::Object(object))
        }
        Some(b'[') => {
            let items: Vec<Box<RawValue>> = serde_json::from_str(text)?;
            items
                .iter()
                .map(|item| decode(item, depth + 1))
                .collect::<Result<Vec<_>, _>>()
                .map(Value::Array)
        }
        Some(b'"') => serde_json::from_str(text).map(Value::String),
        Some(b't' | b'f') => serde_json::from_str(text).map(Value::Bool),
        Some(b'n') => Ok(Value::Null),
        _ => Number::from_str(text).map(Value::Number),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numeric_lexemes_are_preserved_without_changing_objects_to_numbers() {
        let value = parse(r#"{"amount":9999999999999999.99,"small":0.000000001,"$serde_json::private::Number":"12.34"}"#).unwrap();
        assert_eq!(value["amount"].to_string(), "9999999999999999.99");
        assert_eq!(value["small"].to_string(), "0.000000001");
        assert_eq!(value["$serde_json::private::Number"], "12.34");
        let object = parse(r#"{"$serde_json::private::Number":"12.34"}"#).unwrap();
        assert!(object.is_object());
        assert_eq!(
            parse(&serde_json::to_string(&object).unwrap()).unwrap(),
            object
        );
    }

    #[test]
    fn malformed_and_excessively_nested_inputs_fail_in_a_controlled_way() {
        assert!(parse("{\"value\": NaN}").is_err());
        assert!(parse("1 2").is_err());
        assert!(parse(&("[".repeat(129) + "0" + &"]".repeat(129))).is_err());
        assert_eq!(
            parse("[null,false,0,\"\"]").unwrap(),
            serde_json::json!([null, false, 0, ""])
        );
    }
}
