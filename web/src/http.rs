use super::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HttpError {
    pub message: String,
}

impl fmt::Display for HttpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.message.fmt(f)
    }
}

impl std::error::Error for HttpError {}

pub fn parse_request(raw: &str) -> Result<Request, HttpError> {
    let parsed = parse_request_bytes(raw.as_bytes())?;
    let body = std::str::from_utf8(parsed.body).map_err(|error| HttpError {
        message: error.to_string(),
    })?;
    Ok(Request {
        method: parsed.method,
        target: parsed.target,
        path: parsed.path,
        headers: parsed.headers,
        body: body.into(),
        remote_addr: None,
    })
}

pub(super) struct ParsedRequestBytes<'a> {
    pub method: String,
    pub target: String,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub body: &'a [u8],
}

pub(super) fn parse_request_bytes(raw: &[u8]) -> Result<ParsedRequestBytes<'_>, HttpError> {
    parse_request_bytes_with_limit(raw, MAX_REQUEST_BODY_BYTES)
}

pub(super) fn parse_request_bytes_with_limit(
    raw: &[u8],
    max_body_bytes: usize,
) -> Result<ParsedRequestBytes<'_>, HttpError> {
    let header_end = find_header_end(raw).unwrap_or(raw.len());
    let header_text = std::str::from_utf8(&raw[..header_end]).map_err(|error| HttpError {
        message: error.to_string(),
    })?;
    let body = if header_end == raw.len() {
        &raw[raw.len()..]
    } else {
        &raw[header_end + 4..]
    };
    let mut lines = header_text.split("\r\n");
    let request_line = lines.next().ok_or_else(|| HttpError {
        message: "request is empty".into(),
    })?;
    let mut request_parts = request_line.split_whitespace();
    let method = request_parts.next().ok_or_else(|| HttpError {
        message: "request method is missing".into(),
    })?;
    let target = request_parts.next().ok_or_else(|| HttpError {
        message: "request target is missing".into(),
    })?;
    let version = request_parts.next().ok_or_else(|| HttpError {
        message: "HTTP version is missing".into(),
    })?;
    if version != "HTTP/1.1" && version != "HTTP/1.0" {
        return Err(HttpError {
            message: format!("unsupported HTTP version `{version}`"),
        });
    }
    if request_parts.next().is_some() {
        return Err(HttpError {
            message: "request line contains too many fields".into(),
        });
    }
    let mut headers: HashMap<String, String> = HashMap::new();
    for line in lines {
        if line.is_empty() {
            break;
        }
        let (name, value) = line.split_once(':').ok_or_else(|| HttpError {
            message: "malformed HTTP header".into(),
        })?;
        let name = name.trim().to_ascii_lowercase();
        if matches!(
            name.as_str(),
            "host"
                | "origin"
                | "referer"
                | "x-forwarded-proto"
                | "cookie"
                | "authorization"
                | "x-request-id"
                | "x-zelyra-tenant"
                | "x-zelyra-editor-token"
                | "content-type"
        ) && headers.contains_key(&name)
        {
            return Err(HttpError {
                message: "request contains a duplicate security-sensitive header".into(),
            });
        }
        headers.insert(name, value.trim().into());
    }
    if let Some(content_length) = headers.get("content-length") {
        let content_length = content_length.parse::<usize>().map_err(|_| HttpError {
            message: "content-length must be a non-negative integer".into(),
        })?;
        if content_length > max_body_bytes {
            return Err(HttpError {
                message: format!("request body exceeds the {} byte limit", max_body_bytes),
            });
        }
        if content_length != body.len() {
            return Err(HttpError {
                message: format!(
                    "content-length declares {content_length} bytes, received {}",
                    body.len()
                ),
            });
        }
    }
    if body.len() > max_body_bytes {
        return Err(HttpError {
            message: format!("request body exceeds the {} byte limit", max_body_bytes),
        });
    }
    let path = target.split_once('?').map_or(target, |(path, _)| path);
    Ok(ParsedRequestBytes {
        method: method.into(),
        target: target.into(),
        path: path.into(),
        headers,
        body,
    })
}

pub const MAX_REQUEST_BODY_BYTES: usize = 1_048_576;
pub(super) const MAX_UPLOAD_REQUEST_BODY_BYTES: usize = 256 * 1024 * 1024;
const MAX_IN_FLIGHT_REQUEST_BODY_BYTES: usize = 256 * 1024 * 1024;
pub(super) const MULTIPART_FRAMING_BUDGET_BYTES: usize = 1024 * 1024;
pub(super) const MAX_REQUEST_HEADER_BYTES: usize = 64 * 1024;
pub(super) const HTTP_EXCHANGE_TIMEOUT: Duration = Duration::from_secs(30);
static IN_FLIGHT_REQUEST_BODY_BYTES: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);

pub(super) struct ReadRequestBytes<'a> {
    pub bytes: Vec<u8>,
    pub max_body_bytes: usize,
    _budget: Option<RequestBodyBudget<'a>>,
}

struct RequestBodyBudget<'a> {
    bytes: usize,
    counter: &'a std::sync::atomic::AtomicUsize,
}

impl<'a> RequestBodyBudget<'a> {
    fn reserve(
        counter: &'a std::sync::atomic::AtomicUsize,
        bytes: usize,
        limit: usize,
    ) -> Option<Self> {
        use std::sync::atomic::Ordering;
        if reserve_request_body_bytes(counter, bytes, limit, Ordering::AcqRel, Ordering::Acquire) {
            Some(Self { bytes, counter })
        } else {
            None
        }
    }
}

fn reserve_request_body_bytes(
    counter: &std::sync::atomic::AtomicUsize,
    bytes: usize,
    limit: usize,
    success_order: std::sync::atomic::Ordering,
    failure_order: std::sync::atomic::Ordering,
) -> bool {
    let mut used = counter.load(failure_order);
    loop {
        let Some(total) = used.checked_add(bytes).filter(|total| *total <= limit) else {
            return false;
        };
        match counter.compare_exchange_weak(used, total, success_order, failure_order) {
            Ok(_) => return true,
            Err(observed) => used = observed,
        }
    }
}

impl Drop for RequestBodyBudget<'_> {
    fn drop(&mut self) {
        self.counter
            .fetch_sub(self.bytes, std::sync::atomic::Ordering::Release);
    }
}

#[derive(Debug)]
pub(super) enum RequestReadError {
    Io(io::Error),
    Http(HttpError),
    PayloadTooLarge,
    InFlightLimit,
}

pub(super) fn find_header_end(buffer: &[u8]) -> Option<usize> {
    buffer.windows(4).position(|window| window == b"\r\n\r\n")
}

pub(super) fn declared_content_length(header_text: &str) -> Result<Option<usize>, HttpError> {
    let mut content_length = None;
    for line in header_text.split("\r\n").skip(1) {
        let Some((name, value)) = line.split_once(':') else {
            if !line.is_empty() {
                return Err(HttpError {
                    message: "malformed HTTP header".into(),
                });
            }
            continue;
        };
        if name.trim().eq_ignore_ascii_case("transfer-encoding") {
            return Err(HttpError {
                message: "transfer-encoding is not supported".into(),
            });
        }
        if name.trim().eq_ignore_ascii_case("content-length") {
            let parsed = value.trim().parse::<usize>().map_err(|_| HttpError {
                message: "content-length must be a non-negative integer".into(),
            })?;
            if let Some(previous) = content_length {
                if previous != parsed {
                    return Err(HttpError {
                        message: "conflicting content-length headers".into(),
                    });
                }
            }
            content_length = Some(parsed);
        }
    }
    Ok(content_length)
}

#[cfg(test)]
pub(super) fn read_http_request_bytes_from<R: Read>(
    reader: &mut R,
) -> Result<Vec<u8>, RequestReadError> {
    read_http_request_bytes_from_with_limit(reader, |_| MAX_REQUEST_BODY_BYTES)
        .map(|request| request.bytes)
}

pub(super) fn read_http_request_bytes_from_with_limit<R: Read>(
    reader: &mut R,
    body_limit_for_headers: impl FnOnce(&str) -> usize,
) -> Result<ReadRequestBytes<'static>, RequestReadError> {
    let mut buffer = Vec::new();
    let mut body_limit_for_headers = Some(body_limit_for_headers);
    let mut max_body_bytes = MAX_REQUEST_BODY_BYTES;
    let mut body_budget = None;
    let total_length = loop {
        let mut chunk = [0_u8; 8192];
        let size = reader.read(&mut chunk).map_err(RequestReadError::Io)?;
        if size == 0 {
            break None;
        }
        buffer.extend_from_slice(&chunk[..size]);
        if buffer.len() > MAX_REQUEST_HEADER_BYTES && find_header_end(&buffer).is_none() {
            return Err(RequestReadError::Http(HttpError {
                message: "HTTP headers exceed the configured limit".into(),
            }));
        }
        let Some(header_end) = find_header_end(&buffer) else {
            continue;
        };
        if header_end > MAX_REQUEST_HEADER_BYTES {
            return Err(RequestReadError::Http(HttpError {
                message: "HTTP headers exceed the configured limit".into(),
            }));
        }
        let header_text = std::str::from_utf8(&buffer[..header_end]).map_err(|error| {
            RequestReadError::Http(HttpError {
                message: error.to_string(),
            })
        })?;
        let content_length = declared_content_length(header_text)
            .map_err(RequestReadError::Http)?
            .unwrap_or(0);
        max_body_bytes = body_limit_for_headers
            .take()
            .map(|resolve| resolve(header_text))
            .unwrap_or(MAX_REQUEST_BODY_BYTES)
            .clamp(MAX_REQUEST_BODY_BYTES, MAX_UPLOAD_REQUEST_BODY_BYTES);
        if content_length > max_body_bytes {
            return Err(RequestReadError::PayloadTooLarge);
        }
        body_budget = RequestBodyBudget::reserve(
            &IN_FLIGHT_REQUEST_BODY_BYTES,
            content_length,
            MAX_IN_FLIGHT_REQUEST_BODY_BYTES,
        );
        if content_length > 0 && body_budget.is_none() {
            return Err(RequestReadError::InFlightLimit);
        }
        let total_length = header_end + 4 + content_length;
        if buffer.len() >= total_length {
            break Some(total_length);
        }
        while buffer.len() < total_length {
            let size = reader.read(&mut chunk).map_err(RequestReadError::Io)?;
            if size == 0 {
                break;
            }
            buffer.extend_from_slice(&chunk[..size]);
        }
        break Some(total_length);
    };
    if let Some(total_length) = total_length {
        buffer.truncate(total_length);
    }
    Ok(ReadRequestBytes {
        bytes: buffer,
        max_body_bytes,
        _budget: body_budget,
    })
}

#[cfg(test)]
pub(super) fn read_http_request_from<R: Read>(reader: &mut R) -> Result<String, RequestReadError> {
    String::from_utf8(read_http_request_bytes_from(reader)?).map_err(|error| {
        RequestReadError::Http(HttpError {
            message: error.to_string(),
        })
    })
}

pub(super) struct DeadlineReader<'a> {
    stream: &'a mut TcpStream,
    deadline: Instant,
}

impl Read for DeadlineReader<'_> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let remaining = self.deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "HTTP exchange deadline exceeded",
            ));
        }
        self.stream.set_read_timeout(Some(remaining))?;
        self.stream.read(buffer)
    }
}

pub(super) fn read_http_request_bytes(
    stream: &mut TcpStream,
    deadline: Instant,
    body_limit_for_headers: impl FnOnce(&str) -> usize,
) -> Result<ReadRequestBytes<'static>, RequestReadError> {
    read_http_request_bytes_from_with_limit(
        &mut DeadlineReader { stream, deadline },
        body_limit_for_headers,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn request_body_budget_is_bounded_and_released_by_guard() {
        let counter = AtomicUsize::new(0);
        let guard = RequestBodyBudget::reserve(&counter, 6, 10).unwrap();
        assert!(!reserve_request_body_bytes(
            &counter,
            5,
            10,
            Ordering::AcqRel,
            Ordering::Acquire
        ));
        drop(guard);
        assert_eq!(counter.load(Ordering::Acquire), 0);
        let guard = RequestBodyBudget::reserve(&counter, 5, 10).unwrap();
        assert_eq!(counter.load(Ordering::Acquire), 5);
        drop(guard);
        assert_eq!(counter.load(Ordering::Acquire), 0);
    }

    #[test]
    fn concurrent_request_body_reservations_never_exceed_the_limit() {
        use std::sync::{atomic::AtomicUsize, Arc, Barrier};
        use std::thread;

        let counter = Arc::new(AtomicUsize::new(0));
        let barrier = Arc::new(Barrier::new(32));
        let workers = (0..32)
            .map(|_| {
                let counter = counter.clone();
                let barrier = barrier.clone();
                thread::spawn(move || {
                    barrier.wait();
                    let guard = RequestBodyBudget::reserve(&counter, 1, 8);
                    barrier.wait();
                    guard.is_some()
                })
            })
            .collect::<Vec<_>>();
        let successful_reservations = workers
            .into_iter()
            .map(|worker| worker.join().expect("reservation worker must not panic"))
            .filter(|reserved| *reserved)
            .count();

        assert_eq!(successful_reservations, 8);
        assert_eq!(counter.load(std::sync::atomic::Ordering::Acquire), 0);
    }

    #[test]
    fn route_limit_reader_accepts_declared_large_body_but_default_remains_capped() {
        let body = vec![b'x'; MAX_REQUEST_BODY_BYTES + 1];
        let request = format!(
            "POST /upload HTTP/1.1\r\nHost: localhost\r\nContent-Type: multipart/form-data; boundary=b\r\nContent-Length: {}\r\n\r\n",
            body.len()
        )
        .into_bytes();
        let mut bytes = request.clone();
        bytes.extend_from_slice(&body);
        let accepted =
            read_http_request_bytes_from_with_limit(&mut Cursor::new(bytes.clone()), |_| {
                MAX_REQUEST_BODY_BYTES + 1
            })
            .expect("route-specific body limit should accept its declared size");
        assert_eq!(accepted.bytes.len(), request.len() + body.len());
        assert!(matches!(
            read_http_request_bytes_from(&mut Cursor::new(bytes)),
            Err(RequestReadError::PayloadTooLarge)
        ));
    }

    #[test]
    fn byte_reader_preserves_non_utf8_multipart_payloads() {
        let body = b"--b\r\nContent-Disposition: form-data; name=\"file\"; filename=\"x.bin\"\r\n\r\n\0\xff\r\n--b--\r\n";
        let mut request = format!(
            "POST /upload HTTP/1.1\r\nHost: localhost\r\nContent-Type: multipart/form-data; boundary=b\r\nContent-Length: {}\r\n\r\n",
            body.len()
        )
        .into_bytes();
        request.extend_from_slice(body);
        let read = read_http_request_bytes_from(&mut Cursor::new(request.clone()))
            .expect("read binary request");
        let parsed = parse_request_bytes(&read).expect("parse binary request headers");
        assert_eq!(parsed.body, body);
        assert_eq!(
            multipart::decode(
                parsed.headers.get("content-type").unwrap(),
                parsed.body,
                multipart::MultipartLimits::default(),
            )
            .unwrap()[0]
                .data,
            b"\0\xff"
        );
    }

    #[test]
    fn text_request_parser_retains_its_string_body_contract() {
        let parsed =
            parse_request("POST /save HTTP/1.1\r\nHost: localhost\r\nContent-Length: 3\r\n\r\nabc")
                .unwrap();
        assert_eq!(parsed.body, "abc");
    }

    #[test]
    fn byte_reader_rejects_transfer_encoding_instead_of_truncating_chunked_bodies() {
        let request = b"POST /upload HTTP/1.1\r\nHost: localhost\r\nTransfer-Encoding: chunked\r\n\r\n0\r\n\r\n";
        let error = read_http_request_bytes_from(&mut Cursor::new(request))
            .expect_err("chunked requests are unsupported");
        assert!(
            matches!(error, RequestReadError::Http(HttpError { message }) if message.contains("transfer-encoding is not supported"))
        );
    }
}
