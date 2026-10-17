#![no_main]

use libfuzzer_sys::fuzz_target;
use zelyra_mir::bytecode::{decode_with_limits, encode_with_limits, BytecodeFormatLimits};

const MAX_FUZZ_INPUT: usize = 1024 * 1024;

fuzz_target!(|input: &[u8]| {
    if input.len() > MAX_FUZZ_INPUT {
        return;
    }

    let limits = BytecodeFormatLimits {
        max_bytes: MAX_FUZZ_INPUT,
        max_functions: 1024,
        max_instructions: 100_000,
        max_string_bytes: 64 * 1024,
        max_type_depth: 64,
        max_type_nodes: 100_000,
    };

    if let Ok(program) = decode_with_limits(input, limits) {
        // Accepted untrusted artifacts must remain safe to re-encode and
        // decode under the same bounds. libFuzzer treats a panic in any of
        // these steps as a finding.
        if let Ok(encoded) = encode_with_limits(&program, limits) {
            let _ = decode_with_limits(&encoded, limits);
        }
    }
});
