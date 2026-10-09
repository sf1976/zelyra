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
    let (header_text, body) = raw.split_once("\r\n\r\n").unwrap_or((raw, ""));
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
        if content_length > MAX_REQUEST_BODY_BYTES {
            return Err(HttpError {
                message: format!(
                    "request body exceeds the {} byte limit",
                    MAX_REQUEST_BODY_BYTES
                ),
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
    if body.len() > MAX_REQUEST_BODY_BYTES {
        return Err(HttpError {
            message: format!(
                "request body exceeds the {} byte limit",
                MAX_REQUEST_BODY_BYTES
            ),
        });
    }
    let path = target.split_once('?').map_or(target, |(path, _)| path);
    Ok(Request {
        method: method.into(),
        target: target.into(),
        path: path.into(),
        headers,
        body: body.into(),
        remote_addr: None,
    })
}

pub const MAX_REQUEST_BODY_BYTES: usize = 1_048_576;
pub(super) const MAX_REQUEST_HEADER_BYTES: usize = 64 * 1024;
pub(super) const HTTP_EXCHANGE_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug)]
pub(super) enum RequestReadError {
    Io(io::Error),
    Http(HttpError),
    PayloadTooLarge,
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

pub(super) fn read_http_request_from<R: Read>(reader: &mut R) -> Result<String, RequestReadError> {
    let mut buffer = Vec::new();
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
        if content_length > MAX_REQUEST_BODY_BYTES {
            return Err(RequestReadError::PayloadTooLarge);
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
    String::from_utf8(buffer).map_err(|error| {
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

pub(super) fn read_http_request(
    stream: &mut TcpStream,
    deadline: Instant,
) -> Result<String, RequestReadError> {
    read_http_request_from(&mut DeadlineReader { stream, deadline })
}
