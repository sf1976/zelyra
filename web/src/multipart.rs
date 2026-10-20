//! Bounded, byte-preserving decoder for `multipart/form-data` request bodies.
//!
//! This module does not decide whether a part is a valid application upload.
//! Callers must enforce the request-wide byte limit before decoding and then
//! apply the route's field, file-size, content-type, authorization, and tenant
//! policies. Part bodies are borrowed directly from the bounded request bytes.

use std::fmt;

const MAX_BOUNDARY_BYTES: usize = 70;
const DEFAULT_MAX_PARTS: usize = 64;
const DEFAULT_MAX_PART_HEADER_BYTES: usize = 16 * 1024;
const DEFAULT_MAX_HEADERS_PER_PART: usize = 64;
const DEFAULT_MAX_FIELD_NAME_BYTES: usize = 128;
const DEFAULT_MAX_FILENAME_BYTES: usize = 255;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MultipartLimits {
    pub max_body_bytes: usize,
    pub max_parts: usize,
    pub max_part_header_bytes: usize,
    pub max_headers_per_part: usize,
    pub max_field_name_bytes: usize,
    pub max_filename_bytes: usize,
}

impl MultipartLimits {
    pub const fn new(max_body_bytes: usize) -> Self {
        Self {
            max_body_bytes,
            max_parts: DEFAULT_MAX_PARTS,
            max_part_header_bytes: DEFAULT_MAX_PART_HEADER_BYTES,
            max_headers_per_part: DEFAULT_MAX_HEADERS_PER_PART,
            max_field_name_bytes: DEFAULT_MAX_FIELD_NAME_BYTES,
            max_filename_bytes: DEFAULT_MAX_FILENAME_BYTES,
        }
    }
}

impl Default for MultipartLimits {
    fn default() -> Self {
        Self::new(1024 * 1024)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MultipartPart<'a> {
    pub name: String,
    /// The submitted filename is untrusted metadata and must never be used as
    /// a filesystem path.
    pub filename: Option<String>,
    pub content_type: Option<String>,
    pub data: &'a [u8],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MultipartError {
    pub message: &'static str,
}

impl fmt::Display for MultipartError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.message)
    }
}

impl std::error::Error for MultipartError {}

fn error(message: &'static str) -> MultipartError {
    MultipartError { message }
}

/// Decode a bounded `multipart/form-data` body without interpreting part data
/// as text. The total body length is checked before any scanning occurs.
pub fn decode<'a>(
    content_type: &str,
    body: &'a [u8],
    limits: MultipartLimits,
) -> Result<Vec<MultipartPart<'a>>, MultipartError> {
    if body.len() > limits.max_body_bytes {
        return Err(error("multipart request exceeds the configured body limit"));
    }
    let boundary = parse_boundary(content_type)?;
    let mut delimiter = Vec::with_capacity(boundary.len() + 2);
    delimiter.extend_from_slice(b"--");
    delimiter.extend_from_slice(boundary.as_bytes());
    if !body.starts_with(&delimiter) {
        return Err(error("multipart body does not begin with its boundary"));
    }

    let mut cursor = delimiter.len();
    let mut parts = Vec::new();
    loop {
        if body.get(cursor..cursor + 2) == Some(b"--") {
            cursor += 2;
            if body.get(cursor..cursor + 2) == Some(b"\r\n") {
                cursor += 2;
            }
            if cursor != body.len() {
                return Err(error(
                    "multipart body contains data after the closing boundary",
                ));
            }
            return Ok(parts);
        }
        if body.get(cursor..cursor + 2) != Some(b"\r\n") {
            return Err(error("multipart boundary is not followed by a part"));
        }
        cursor += 2;
        if parts.len() >= limits.max_parts {
            return Err(error("multipart body contains too many parts"));
        }

        let header_end = find_bytes(&body[cursor..], b"\r\n\r\n")
            .map(|offset| cursor + offset)
            .ok_or_else(|| error("multipart part headers are incomplete"))?;
        let header_len = header_end - cursor;
        if header_len > limits.max_part_header_bytes {
            return Err(error("multipart part headers exceed the configured limit"));
        }
        let headers = parse_headers(&body[cursor..header_end], limits.max_headers_per_part)?;
        cursor = header_end + 4;

        let mut body_delimiter = Vec::with_capacity(boundary.len() + 4);
        body_delimiter.extend_from_slice(b"\r\n--");
        body_delimiter.extend_from_slice(boundary.as_bytes());
        let (data_end, next_cursor) = find_part_end(body, cursor, &body_delimiter)?;
        let (name, filename) = parse_disposition(
            headers
                .get("content-disposition")
                .ok_or_else(|| error("multipart part is missing Content-Disposition"))?,
            limits,
        )?;
        let content_type = headers.get("content-type").cloned();
        parts.push(MultipartPart {
            name,
            filename,
            content_type,
            data: &body[cursor..data_end],
        });
        cursor = next_cursor;
    }
}

fn parse_boundary(content_type: &str) -> Result<String, MultipartError> {
    let mut pieces = split_parameters(content_type)?;
    if !pieces.next().is_some_and(|media_type| {
        media_type
            .trim()
            .eq_ignore_ascii_case("multipart/form-data")
    }) {
        return Err(error("content type is not multipart/form-data"));
    }
    let mut boundary = None;
    for parameter in pieces {
        let Some((name, raw_value)) = parameter.split_once('=') else {
            return Err(error(
                "multipart content type contains an invalid parameter",
            ));
        };
        if name.trim().eq_ignore_ascii_case("boundary") {
            if boundary.is_some() {
                return Err(error(
                    "multipart content type contains duplicate boundaries",
                ));
            }
            boundary = Some(parse_parameter_value(raw_value.trim())?);
        }
    }
    let boundary = boundary.ok_or_else(|| error("multipart content type is missing a boundary"))?;
    if boundary.is_empty()
        || boundary.len() > MAX_BOUNDARY_BYTES
        || !boundary.bytes().all(is_boundary_byte)
        || boundary.ends_with(' ')
    {
        return Err(error("multipart boundary is invalid or too long"));
    }
    Ok(boundary)
}

fn is_boundary_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || b"'()+_,-./:=? ".contains(&byte)
}

fn split_parameters(value: &str) -> Result<impl Iterator<Item = &str>, MultipartError> {
    let mut pieces = Vec::new();
    let mut quoted = false;
    let mut escaped = false;
    let mut start = 0;
    for (index, character) in value.char_indices() {
        if escaped {
            escaped = false;
        } else if quoted && character == '\\' {
            escaped = true;
        } else if character == '"' {
            quoted = !quoted;
        } else if character == ';' && !quoted {
            pieces.push(&value[start..index]);
            start = index + character.len_utf8();
        }
    }
    if quoted || escaped {
        return Err(error(
            "multipart header contains an unterminated quoted value",
        ));
    }
    pieces.push(&value[start..]);
    Ok(pieces.into_iter())
}

fn parse_parameter_value(value: &str) -> Result<String, MultipartError> {
    if let Some(inner) = value.strip_prefix('"').and_then(|v| v.strip_suffix('"')) {
        unquote(inner)
    } else if value.is_empty()
        || value
            .bytes()
            .any(|byte| byte.is_ascii_whitespace() || byte == b'"')
    {
        Err(error("multipart header contains an invalid token value"))
    } else {
        Ok(value.to_owned())
    }
}

fn unquote(value: &str) -> Result<String, MultipartError> {
    let mut output = String::with_capacity(value.len());
    let mut characters = value.chars();
    while let Some(character) = characters.next() {
        if character == '\\' {
            let escaped = characters
                .next()
                .ok_or_else(|| error("multipart header contains an incomplete escape"))?;
            if escaped.is_control() {
                return Err(error("multipart header contains a control character"));
            }
            output.push(escaped);
        } else if character == '"' || character.is_control() {
            return Err(error("multipart header contains an invalid quoted value"));
        } else {
            output.push(character);
        }
    }
    Ok(output)
}

fn parse_headers(
    raw: &[u8],
    max_headers: usize,
) -> Result<std::collections::BTreeMap<String, String>, MultipartError> {
    let text =
        std::str::from_utf8(raw).map_err(|_| error("multipart part headers are not UTF-8"))?;
    let mut headers = std::collections::BTreeMap::new();
    for line in text.split("\r\n") {
        if headers.len() >= max_headers {
            return Err(error("multipart part contains too many headers"));
        }
        if line.starts_with([' ', '\t']) {
            return Err(error("folded multipart headers are not supported"));
        }
        if line
            .chars()
            .any(|character| character.is_control() && character != '\t')
        {
            return Err(error("multipart part header contains a control character"));
        }
        let (name, value) = line
            .split_once(':')
            .ok_or_else(|| error("multipart part contains a malformed header"))?;
        let name = name.trim().to_ascii_lowercase();
        if name.is_empty()
            || !name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&byte))
        {
            return Err(error("multipart part contains an invalid header name"));
        }
        if headers.insert(name, value.trim().to_owned()).is_some() {
            return Err(error("multipart part contains a duplicate header"));
        }
    }
    if headers
        .get("content-transfer-encoding")
        .is_some_and(|encoding| !encoding.eq_ignore_ascii_case("binary"))
    {
        return Err(error("encoded multipart transfer bodies are not supported"));
    }
    Ok(headers)
}

fn parse_disposition(
    header: &str,
    limits: MultipartLimits,
) -> Result<(String, Option<String>), MultipartError> {
    let mut pieces = split_parameters(header)?;
    if !pieces
        .next()
        .is_some_and(|kind| kind.trim().eq_ignore_ascii_case("form-data"))
    {
        return Err(error("multipart part disposition is not form-data"));
    }
    let mut name = None;
    let mut filename = None;
    for parameter in pieces {
        let (key, raw_value) = parameter
            .split_once('=')
            .ok_or_else(|| error("multipart disposition contains an invalid parameter"))?;
        let key = key.trim().to_ascii_lowercase();
        let value = parse_parameter_value(raw_value.trim())?;
        match key.as_str() {
            "name" => {
                if name.replace(value).is_some() {
                    return Err(error(
                        "multipart disposition contains duplicate field names",
                    ));
                }
            }
            "filename" => {
                if filename.replace(value).is_some() {
                    return Err(error("multipart disposition contains duplicate filenames"));
                }
            }
            "filename*" => {
                return Err(error("extended multipart filenames are not supported"));
            }
            _ => {}
        }
    }
    let name = name.ok_or_else(|| error("multipart part is missing its field name"))?;
    if name.is_empty()
        || name.len() > limits.max_field_name_bytes
        || name.chars().any(char::is_control)
    {
        return Err(error("multipart field name is invalid or too long"));
    }
    if filename.as_ref().is_some_and(|value| {
        value.len() > limits.max_filename_bytes || value.chars().any(char::is_control)
    }) {
        return Err(error("multipart filename is invalid or too long"));
    }
    Ok((name, filename))
}

fn find_part_end(
    body: &[u8],
    start: usize,
    delimiter: &[u8],
) -> Result<(usize, usize), MultipartError> {
    let mut search = start;
    while let Some(offset) = find_bytes(&body[search..], delimiter) {
        let delimiter_start = search + offset;
        let suffix_start = delimiter_start + delimiter.len();
        match body.get(suffix_start..suffix_start + 2) {
            Some(b"\r\n") => return Ok((delimiter_start, suffix_start)),
            Some(b"--") => return Ok((delimiter_start, suffix_start)),
            _ => search = delimiter_start + 1,
        }
    }
    Err(error("multipart part has no terminating boundary"))
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limits() -> MultipartLimits {
        MultipartLimits {
            max_body_bytes: 4096,
            max_parts: 4,
            max_part_header_bytes: 512,
            max_headers_per_part: 8,
            max_field_name_bytes: 32,
            max_filename_bytes: 64,
        }
    }

    #[test]
    fn preserves_binary_payload_bytes_and_quoted_boundary() {
        let body = b"--a?b\r\nContent-Disposition: form-data; name=\"upload\"; filename=\"photo.png\"\r\nContent-Type: image/png\r\n\r\n\x89PNG\x00\xff\r\n--a?b--\r\n";
        let parts = decode(
            "multipart/form-data; charset=utf-8; boundary=\"a?b\"",
            body,
            limits(),
        )
        .unwrap();
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0].name, "upload");
        assert_eq!(parts[0].filename.as_deref(), Some("photo.png"));
        assert_eq!(parts[0].content_type.as_deref(), Some("image/png"));
        assert_eq!(parts[0].data, b"\x89PNG\x00\xff");
    }

    #[test]
    fn parses_multiple_fields_in_order_and_empty_file_selection() {
        let body = b"--BOUNDARY\r\nContent-Disposition: form-data; name=csrf\r\n\r\ntoken\r\n--BOUNDARY\r\nContent-Disposition: form-data; name=optional; filename=\"\"\r\nContent-Type: application/octet-stream\r\n\r\n\r\n--BOUNDARY--";
        let parts = decode("Multipart/Form-Data; boundary=BOUNDARY", body, limits()).unwrap();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].data, b"token");
        assert_eq!(parts[1].filename.as_deref(), Some(""));
        assert!(parts[1].data.is_empty());
    }

    #[test]
    fn rejects_unbounded_or_malformed_requests() {
        let body = b"--b--";
        assert_eq!(
            decode(
                "multipart/form-data; boundary=b",
                body,
                MultipartLimits::new(1)
            )
            .unwrap_err()
            .message,
            "multipart request exceeds the configured body limit"
        );
        for (content_type, malformed) in [
            ("application/octet-stream", body.as_slice()),
            ("multipart/form-data", body.as_slice()),
            (
                "multipart/form-data; boundary=b; boundary=c",
                body.as_slice(),
            ),
            ("multipart/form-data; boundary=bad\r\nname", body.as_slice()),
            (
                "multipart/form-data; boundary=b",
                b"--b\r\nno headers\r\n\r\nx".as_slice(),
            ),
            (
                "multipart/form-data; boundary=b",
                b"--b\r\nContent-Disposition: form-data; name=x\r\n\r\ndata".as_slice(),
            ),
            (
                "multipart/form-data; boundary=b",
                b"--b\r\nContent-Disposition: form-data; name=x\nInjected: yes\r\n\r\ndata\r\n--b--".as_slice(),
            ),
        ] {
            assert!(decode(content_type, malformed, limits()).is_err());
        }
    }

    #[test]
    fn rejects_header_and_part_count_overflow() {
        let header_limited = MultipartLimits {
            max_part_header_bytes: 8,
            ..limits()
        };
        let too_large_header = b"--b\r\nContent-Disposition: form-data; name=x\r\n\r\ny\r\n--b--";
        assert_eq!(
            decode(
                "multipart/form-data; boundary=b",
                too_large_header,
                header_limited
            )
            .unwrap_err()
            .message,
            "multipart part headers exceed the configured limit"
        );

        let part_limited = MultipartLimits {
            max_parts: 1,
            ..limits()
        };
        let two_parts = b"--b\r\nContent-Disposition: form-data; name=a\r\n\r\n1\r\n--b\r\nContent-Disposition: form-data; name=b\r\n\r\n2\r\n--b--";
        assert_eq!(
            decode("multipart/form-data; boundary=b", two_parts, part_limited)
                .unwrap_err()
                .message,
            "multipart body contains too many parts"
        );

        let header_count_limited = MultipartLimits {
            max_headers_per_part: 1,
            ..limits()
        };
        let too_many_headers =
            b"--b\r\nContent-Disposition: form-data; name=x\r\nX-Extra: y\r\n\r\nz\r\n--b--";
        assert_eq!(
            decode(
                "multipart/form-data; boundary=b",
                too_many_headers,
                header_count_limited
            )
            .unwrap_err()
            .message,
            "multipart part contains too many headers"
        );
    }

    #[test]
    fn rejects_duplicate_security_sensitive_headers_and_extended_filenames() {
        let duplicate = b"--b\r\nContent-Disposition: form-data; name=x\r\nContent-Type: text/plain\r\nContent-Type: text/html\r\n\r\nx\r\n--b--";
        assert_eq!(
            decode("multipart/form-data; boundary=b", duplicate, limits())
                .unwrap_err()
                .message,
            "multipart part contains a duplicate header"
        );
        let extended = b"--b\r\nContent-Disposition: form-data; name=x; filename*=utf-8''a.txt\r\n\r\nx\r\n--b--";
        assert_eq!(
            decode("multipart/form-data; boundary=b", extended, limits())
                .unwrap_err()
                .message,
            "extended multipart filenames are not supported"
        );
    }
}
