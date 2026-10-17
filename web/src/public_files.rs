use super::{Request, Response};
use std::fs::{self, File};
use std::io::Read;
use std::path::{Component, Path, PathBuf};

pub(super) const MAX_PUBLIC_FILE_BYTES: u64 = 4 * 1024 * 1024;

#[derive(Clone, Debug)]
pub(super) struct PublicDirectory {
    canonical_root: PathBuf,
}

impl PublicDirectory {
    pub(super) fn new(path: &Path) -> Result<Self, String> {
        let metadata = fs::symlink_metadata(path).map_err(|error| {
            format!(
                "cannot inspect public directory {}: {error}",
                path.display()
            )
        })?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err("public path must be a real directory, not a symlink".into());
        }
        let canonical_root = fs::canonicalize(path)
            .map_err(|error| format!("cannot resolve public directory: {error}"))?;
        Ok(Self { canonical_root })
    }

    pub(super) fn response(&self, request: &Request) -> Option<Response> {
        if !matches!(request.method.as_str(), "GET" | "HEAD")
            || request.path.starts_with("/__zelyra/")
            || !request.path.starts_with('/')
        {
            return None;
        }

        let mut candidate = self.canonical_root.clone();
        for component in request.path.split('/').skip(1) {
            if component.is_empty() {
                continue;
            }
            let component = decode_segment(component).ok()?;
            if component.starts_with('.') && component != ".well-known" {
                return None;
            }
            match Path::new(&component).components().next() {
                Some(Component::Normal(_)) => candidate.push(component),
                _ => return None,
            }
            if fs::symlink_metadata(&candidate)
                .ok()?
                .file_type()
                .is_symlink()
            {
                return None;
            }
        }
        if candidate == self.canonical_root {
            return None;
        }

        let canonical_file = fs::canonicalize(&candidate).ok()?;
        if !canonical_file.starts_with(&self.canonical_root) {
            return None;
        }
        let file = File::open(&canonical_file).ok()?;
        let metadata = file.metadata().ok()?;
        if !metadata.is_file() {
            return None;
        }
        if metadata.len() > MAX_PUBLIC_FILE_BYTES {
            return Some(Response::empty(413).with_header("Cache-Control", "no-store"));
        }
        let mut bytes = Vec::with_capacity(metadata.len() as usize);
        file.take(MAX_PUBLIC_FILE_BYTES + 1)
            .read_to_end(&mut bytes)
            .ok()?;
        if bytes.len() as u64 > MAX_PUBLIC_FILE_BYTES {
            return Some(Response::empty(413).with_header("Cache-Control", "no-store"));
        }

        let content_type = content_type(&canonical_file);
        let mut response = Response::bytes(200, content_type, bytes)
            .with_header("Cache-Control", "public, max-age=3600");
        if request.method == "HEAD" {
            response.suppress_body = true;
        }
        Some(response)
    }
}

fn decode_segment(segment: &str) -> Result<String, ()> {
    let bytes = segment.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let high = *bytes.get(index + 1).ok_or(())?;
            let low = *bytes.get(index + 2).ok_or(())?;
            let high = hex(high).ok_or(())?;
            let low = hex(low).ok_or(())?;
            let value = (high << 4) | low;
            if matches!(value, b'/' | b'\\' | 0) {
                return Err(());
            }
            decoded.push(value);
            index += 3;
        } else {
            if matches!(bytes[index], b'\\' | 0) {
                return Err(());
            }
            decoded.push(bytes[index]);
            index += 1;
        }
    }
    let decoded = String::from_utf8(decoded).map_err(|_| ())?;
    if matches!(decoded.as_str(), "." | "..") {
        return Err(());
    }
    Ok(decoded)
}

fn hex(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn content_type(path: &Path) -> &'static str {
    match path.extension().and_then(|extension| extension.to_str()) {
        Some(extension) if extension.eq_ignore_ascii_case("html") => "text/html; charset=utf-8",
        Some(extension) if extension.eq_ignore_ascii_case("css") => "text/css; charset=utf-8",
        Some(extension) if extension.eq_ignore_ascii_case("js") => "text/javascript; charset=utf-8",
        Some(extension) if extension.eq_ignore_ascii_case("json") => {
            "application/json; charset=utf-8"
        }
        Some(extension) if extension.eq_ignore_ascii_case("txt") => "text/plain; charset=utf-8",
        Some(extension) if extension.eq_ignore_ascii_case("svg") => "image/svg+xml",
        Some(extension) if extension.eq_ignore_ascii_case("png") => "image/png",
        Some(extension)
            if extension.eq_ignore_ascii_case("jpg") || extension.eq_ignore_ascii_case("jpeg") =>
        {
            "image/jpeg"
        }
        Some(extension) if extension.eq_ignore_ascii_case("gif") => "image/gif",
        Some(extension) if extension.eq_ignore_ascii_case("webp") => "image/webp",
        Some(extension) if extension.eq_ignore_ascii_case("ico") => "image/x-icon",
        Some(extension) if extension.eq_ignore_ascii_case("woff") => "font/woff",
        Some(extension) if extension.eq_ignore_ascii_case("woff2") => "font/woff2",
        Some(extension) if extension.eq_ignore_ascii_case("ttf") => "font/ttf",
        Some(extension) if extension.eq_ignore_ascii_case("pdf") => "application/pdf",
        Some(extension) if extension.eq_ignore_ascii_case("wasm") => "application/wasm",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn request(method: &str, path: &str) -> Request {
        Request {
            method: method.into(),
            target: path.into(),
            path: path.into(),
            headers: HashMap::new(),
            body: String::new(),
            remote_addr: None,
        }
    }

    fn test_directory() -> (PathBuf, PathBuf) {
        let root = std::env::temp_dir().join(format!(
            "zelyra-public-test-{}-{}",
            std::process::id(),
            super::super::new_request_id()
        ));
        let public = root.join("public");
        fs::create_dir_all(public.join("assets")).unwrap();
        (root, public)
    }

    #[test]
    fn serves_binary_assets_with_mime_type_and_exact_bytes() {
        let (root, public) = test_directory();
        let expected = [0_u8, 255, 1, 2, 127];
        fs::write(public.join("assets/logo.png"), expected).unwrap();
        let directory = PublicDirectory::new(&public).unwrap();
        let response = directory
            .response(&request("GET", "/assets/logo.png"))
            .unwrap();
        assert_eq!(response.status, 200);
        assert_eq!(response.content_type, "image/png");
        assert_eq!(response.body_bytes.as_deref(), Some(expected.as_slice()));
        assert!(response.to_http_bytes().ends_with(&expected));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rejects_traversal_encoded_separators_and_hidden_files() {
        let (root, public) = test_directory();
        fs::write(root.join("secret.txt"), b"secret").unwrap();
        fs::write(public.join(".env"), b"secret").unwrap();
        fs::write(public.join("safe.txt"), b"safe").unwrap();
        let directory = PublicDirectory::new(&public).unwrap();
        assert!(directory
            .response(&request("GET", "/%2e%2e/secret.txt"))
            .is_none());
        assert!(directory
            .response(&request("GET", "/%2e%2e%2fsecret.txt"))
            .is_none());
        assert!(directory.response(&request("GET", "/.env")).is_none());
        assert!(directory.response(&request("GET", "/safe.txt")).is_some());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn head_preserves_content_length_and_suppresses_asset_bytes() {
        let (root, public) = test_directory();
        fs::write(public.join("assets/app.css"), b"a{color:red}").unwrap();
        let directory = PublicDirectory::new(&public).unwrap();
        let response = directory
            .response(&request("HEAD", "/assets/app.css"))
            .unwrap();
        assert!(response.suppress_body);
        assert_eq!(response.body_bytes.as_deref(), Some(&b"a{color:red}"[..]));
        assert!(String::from_utf8(response.to_http_bytes())
            .unwrap()
            .contains("Content-Length: 12\r\n"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn app_dispatch_serves_public_html_without_framework_rewriting() {
        let (root, public) = test_directory();
        let html = "<!doctype html><title>Static page</title>";
        fs::write(public.join("about.html"), html).unwrap();
        let app = super::super::WebApp::new(Vec::new(), Vec::new())
            .with_ui_settings(
                super::super::UiLanguage::English,
                super::super::UiLevel::Learn,
            )
            .with_public_directory(&public)
            .unwrap();
        let response = app.dispatch(&request("GET", "/about.html"));
        assert_eq!(response.status, 200);
        assert_eq!(response.content_type, "text/html; charset=utf-8");
        assert_eq!(response.body_bytes.as_deref(), Some(html.as_bytes()));
        assert!(response.body.is_empty());
        let _ = fs::remove_dir_all(root);
    }

    #[cfg(unix)]
    #[test]
    fn refuses_symlinked_public_root_and_symlink_escape() {
        use std::os::unix::fs::symlink;
        let (root, public) = test_directory();
        let outside = root.join("outside");
        fs::create_dir(&outside).unwrap();
        fs::write(outside.join("secret.txt"), b"secret").unwrap();
        symlink(outside.join("secret.txt"), public.join("assets/escape.txt")).unwrap();
        assert!(PublicDirectory::new(&public)
            .unwrap()
            .response(&request("GET", "/assets/escape.txt"))
            .is_none());
        let link = root.join("public-link");
        symlink(&public, &link).unwrap();
        assert!(PublicDirectory::new(&link).is_err());
        let _ = fs::remove_dir_all(root);
    }
}
