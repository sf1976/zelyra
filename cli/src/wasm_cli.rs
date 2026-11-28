use super::*;
use std::io::Read;

pub(super) fn command(arguments: &[String]) -> ExitCode {
    let [subcommand, path, export, args @ ..] = arguments else {
        usage();
        return ExitCode::from(2);
    };
    if subcommand != "run" {
        usage();
        return ExitCode::from(2);
    }
    match execute(Path::new(path), export, args) {
        Ok(value) => {
            print_machine_document(&json!({"schema_version": 1, "value": value}));
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error[E-WASM-001]: {error}");
            ExitCode::FAILURE
        }
    }
}

fn execute(path: &Path, export: &str, arguments: &[String]) -> Result<i32, String> {
    let arguments = arguments
        .iter()
        .map(|value| value.parse::<i32>())
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| "WebAssembly extension arguments must be signed 32-bit integers")?;
    let metadata = fs::symlink_metadata(path).map_err(|error| error.to_string())?;
    if !metadata.is_file() || metadata.len() > zelyra_runtime::MAX_WASM_EXTENSION_BYTES as u64 {
        return Err("expected a regular, size-bounded WebAssembly extension file".into());
    }
    let mut bytes = Vec::new();
    fs::File::open(path)
        .and_then(|file| {
            file.take((zelyra_runtime::MAX_WASM_EXTENSION_BYTES + 1) as u64)
                .read_to_end(&mut bytes)
        })
        .map_err(|error| error.to_string())?;
    zelyra_runtime::run_wasm_extension(&bytes, export, &arguments).map_err(str::to_owned)
}
