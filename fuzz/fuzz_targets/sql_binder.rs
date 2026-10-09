#![no_main]

use libfuzzer_sys::fuzz_target;
use zelyra_database::fuzz_bind_named_parameters;

fuzz_target!(|input: &[u8]| {
    if input.len() > 16 * 1024 {
        return;
    }
    let sql = String::from_utf8_lossy(input);
    let _ = fuzz_bind_named_parameters(&sql);
});
