//! Shared runtime and effect policy values used by the interpreter backends.

use super::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileSystemPolicy {
    pub base_dir: PathBuf,
    pub read_roots: Vec<PathBuf>,
    pub write_roots: Vec<PathBuf>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NetworkPolicy {
    pub allowed_hosts: Vec<String>,
    pub timeout_ms: u64,
    pub max_response_bytes: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProcessPolicy {
    pub allowed_commands: Vec<String>,
    pub timeout_ms: u64,
    pub max_output_bytes: usize,
}

#[derive(Clone, Default, PartialEq, Eq)]
pub struct MailPolicy {
    pub host: Option<String>,
    pub port: Option<u16>,
    pub security: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub ca_certificate_pem: Option<Vec<u8>>,
    pub from: Option<String>,
}

impl fmt::Debug for MailPolicy {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MailPolicy")
            .field("host", &self.host)
            .field("port", &self.port)
            .field("security", &self.security)
            .field("username", &self.username.as_ref().map(|_| "<redacted>"))
            .field("password", &self.password.as_ref().map(|_| "<redacted>"))
            .field(
                "ca_certificate_pem",
                &self.ca_certificate_pem.as_ref().map(|_| "<configured>"),
            )
            .field("from", &self.from)
            .finish()
    }
}

impl Drop for MailPolicy {
    fn drop(&mut self) {
        if let Some(password) = &mut self.password {
            zeroize::Zeroize::zeroize(password);
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct RuntimePolicy {
    pub filesystem: Option<FileSystemPolicy>,
    pub network: Option<NetworkPolicy>,
    pub process: Option<ProcessPolicy>,
    pub mail: Option<MailPolicy>,
    /// Optional statement and expression evaluation budget for CLI, sandbox,
    /// and untrusted-code runs.
    /// `None` leaves long-running server handlers unrestricted.
    pub step_budget: Option<usize>,
    /// Maximum language function-call depth. All runtimes hard-cap this at 500,
    /// including callers that construct policies through the Rust embedding API.
    pub max_stack_depth: Option<usize>,
    /// Validated package settings, scoped to the source file that owns them.
    /// Values are redacted in Debug output and zeroized when released.
    pub package_settings: Option<PackageSettings>,
}
