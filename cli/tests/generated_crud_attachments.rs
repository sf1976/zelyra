use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

struct TestProject {
    root: PathBuf,
    server: Option<Child>,
}

impl Drop for TestProject {
    fn drop(&mut self) {
        if let Some(server) = &mut self.server {
            let _ = server.kill();
            let _ = server.wait();
        }
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn request(address: &str, bytes: &[u8]) -> Vec<u8> {
    let mut stream = TcpStream::connect(address).expect("connect to generated CRUD server");
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("set response timeout");
    stream.write_all(bytes).expect("write HTTP request");
    let mut response = Vec::new();
    stream
        .read_to_end(&mut response)
        .expect("read HTTP response");
    response
}

fn get_until_response(address: &str, target: &str) -> Vec<u8> {
    let deadline = Instant::now() + Duration::from_secs(10);
    let request = format!("GET {target} HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\n\r\n");
    loop {
        if let Ok(mut stream) = TcpStream::connect(address) {
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .expect("set readiness timeout");
            if stream.write_all(request.as_bytes()).is_ok() {
                let mut response = Vec::new();
                if stream.read_to_end(&mut response).is_ok() && response.starts_with(b"HTTP/1.1 ") {
                    return response;
                }
            }
        }
        assert!(Instant::now() < deadline, "server did not answer {target}");
        thread::sleep(Duration::from_millis(50));
    }
}

fn response_body(response: &[u8]) -> &[u8] {
    let split = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .expect("HTTP response has a header terminator");
    &response[split + 4..]
}

#[test]
#[ignore = "requires ZELYRA_TEST_DATABASE_URL pointing to a disposable MariaDB database"]
fn generated_crud_uploads_and_serves_file_ref_attachments() {
    let database_url = std::env::var("ZELYRA_TEST_DATABASE_URL")
        .expect("ignored integration test needs a disposable database URL");
    let id = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is after Unix epoch")
        .as_nanos();
    let table = format!("zelyra_crud_attachment_{}_{}", std::process::id(), id);
    let source = format!(
        r#"
        database main {{ engine: mariadb }}
        table {table} {{
            id: Id primary auto
            title: String(80) required
            document: FileRef? attachment {{
                max_size: "1KB"
                allowed_types: ["application/pdf"]
            }}
        }}
        crud Contract -> {table} {{ list {{ id title document }} }}
        fn main() {{}}
        "#
    );
    let program = zelyra_parser::parse(&zelyra_lexer::lex(&source).expect("lex test project"))
        .expect("parse test project");
    let schema = zelyra_database::build_schema(&program).expect("build test schema");
    zelyra_database::execute_query(&database_url, &schema.create_sql(), Vec::new())
        .expect("create isolated CRUD table");

    let root = std::env::temp_dir().join(format!("zelyra-generated-attachment-{id}"));
    fs::create_dir_all(&root).expect("create isolated project");
    fs::write(root.join("main.zyl"), &source).expect("write test source");
    fs::write(root.join(".env"), format!("DATABASE_URL={database_url}\n"))
        .expect("write test database configuration");
    let mut project = TestProject { root, server: None };

    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("reserve an available port");
    let port = listener.local_addr().expect("read test port").port();
    drop(listener);
    let address = format!("127.0.0.1:{port}");
    let server = Command::new(env!("CARGO_BIN_EXE_zelyra"))
        .current_dir(&project.root)
        .args(["serve", "main.zyl", &address])
        .env("ZELYRA_AUTH_TOKEN", "attachment-test-token")
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("start Zelyra server");
    project.server = Some(server);

    let new_page = get_until_response(&address, &format!("/{table}/new"));
    assert!(new_page.starts_with(b"HTTP/1.1 200 OK\r\n"), "{new_page:?}");
    let new_page = String::from_utf8(response_body(&new_page).to_vec()).unwrap();
    assert!(new_page.contains("type=\"file\""), "{new_page}");
    assert!(
        new_page.contains("accept=\"application/pdf\""),
        "{new_page}"
    );
    let csrf = new_page
        .split("name=\"_zelyra_csrf\" value=\"")
        .nth(1)
        .and_then(|value| value.split('"').next())
        .expect("generated form includes a CSRF token");

    let boundary = "zelyra-generated-crud-attachment-boundary";
    let pdf = b"%PDF-1.7\ngenerated CRUD attachment";
    let mut body = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"_zelyra_csrf\"\r\n\r\n{csrf}\r\n--{boundary}\r\nContent-Disposition: form-data; name=\"title\"\r\n\r\nSigned contract\r\n--{boundary}\r\nContent-Disposition: form-data; name=\"document\"; filename=\"contract.pdf\"\r\nContent-Type: application/pdf\r\n\r\n"
    )
    .into_bytes();
    body.extend_from_slice(pdf);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    let wire = format!(
        "POST /{table}/new HTTP/1.1\r\nHost: {address}\r\nOrigin: http://{address}\r\nContent-Type: multipart/form-data; boundary={boundary}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let mut upload_request = wire.into_bytes();
    upload_request.extend_from_slice(&body);
    let upload = request(&address, &upload_request);
    assert!(
        upload.starts_with(b"HTTP/1.1 303 See Other\r\n"),
        "{}",
        String::from_utf8_lossy(&upload)
    );

    let rows = zelyra_database::execute_read_only_query(
        &database_url,
        &format!("SELECT id, document FROM {table}"),
        Vec::new(),
    )
    .expect("read generated CRUD row");
    assert_eq!(rows.rows.len(), 1);
    let record_id = &rows.rows[0][0];
    let file_ref = &rows.rows[0][1];
    assert!(!file_ref.is_empty());

    let list = get_until_response(&address, &format!("/{table}"));
    assert!(list.starts_with(b"HTTP/1.1 200 OK\r\n"), "{list:?}");
    let list = String::from_utf8(response_body(&list).to_vec()).unwrap();
    assert!(
        list.contains(&format!("/{table}/{record_id}/files/document")),
        "generated CRUD list exposes its protected download link"
    );
    assert!(!list.contains(file_ref), "raw FileRef must not be rendered");

    let download_request = format!(
        "GET /{table}/{record_id}/files/document HTTP/1.1\r\nHost: {address}\r\nAuthorization: Bearer attachment-test-token\r\nConnection: close\r\n\r\n"
    );
    let unauthenticated_request = format!(
        "GET /{table}/{record_id}/files/document HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\n\r\n"
    );
    let unauthenticated = request(&address, unauthenticated_request.as_bytes());
    assert!(
        !unauthenticated.starts_with(b"HTTP/1.1 200 OK\r\n"),
        "generated attachment downloads require authentication: {}",
        String::from_utf8_lossy(&unauthenticated)
    );
    let download = request(&address, download_request.as_bytes());
    assert!(
        download.starts_with(b"HTTP/1.1 200 OK\r\n"),
        "{}",
        String::from_utf8_lossy(&download)
    );
    assert!(
        download
            .windows(b"Content-Type: application/pdf".len())
            .any(|window| window.eq_ignore_ascii_case(b"Content-Type: application/pdf")),
        "generated file route returns the declared media type"
    );
    assert_eq!(response_body(&download), pdf);

    let _ = project.server.as_mut().unwrap().kill();
    let server_output = project.server.take().unwrap().wait_with_output().unwrap();
    if !server_output.status.success() {
        let stderr = String::from_utf8_lossy(&server_output.stderr);
        assert!(
            stderr.contains("Broken pipe") || stderr.is_empty(),
            "{stderr}"
        );
    }
    zelyra_database::execute_query(&database_url, &format!("DROP TABLE {table}"), Vec::new())
        .expect("drop isolated CRUD table");
}
