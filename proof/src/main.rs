use std::{env, path::Path, process::ExitCode};

fn main() -> ExitCode {
    let args = env::args_os().skip(1).collect::<Vec<_>>();
    let result = match args.as_slice() {
        [flag] if flag == "--help" || flag == "-h" => {
            println!("zelyra-proof-check <zelyra-proof.json> <source-root>\nzelyra-proof-check --bundle <directory> <trusted-public-key-hex>");
            return ExitCode::SUCCESS;
        }
        [proof, root] => zelyra_proof::read_bounded(Path::new(proof)).and_then(|bytes| {
            let proof: zelyra_proof::Proof =
                serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
            proof.check(Path::new(root))
        }),
        [flag, bundle, key] if flag == "--bundle" => key
            .to_str()
            .ok_or_else(|| "public key must be lowercase hexadecimal".into())
            .and_then(|key| zelyra_proof::deployment::check(Path::new(bundle), key)),
        _ => {
            eprintln!("Usage: zelyra-proof-check <zelyra-proof.json> <source-root> | --bundle <directory> <trusted-public-key-hex>");
            return ExitCode::from(2);
        }
    };
    match result {
        Ok(()) => {
            println!("OK: evidence structure and bound files verified");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error[E-PROOF-001]: {error}");
            ExitCode::FAILURE
        }
    }
}
