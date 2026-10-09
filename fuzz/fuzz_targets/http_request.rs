#![no_main]

use libfuzzer_sys::fuzz_target;
use zelyra_web::parse_request;

fuzz_target!(|input: &[u8]| {
    if input.len() > 1024 * 1024 + 16 * 1024 {
        return;
    }
    let request = String::from_utf8_lossy(input);
    let _ = parse_request(&request);
});
