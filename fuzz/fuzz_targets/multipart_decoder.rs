#![no_main]

use libfuzzer_sys::fuzz_target;
use zelyra_web::multipart::{decode, MultipartLimits};

fuzz_target!(|input: &[u8]| {
    let limits = MultipartLimits {
        max_body_bytes: 64 * 1024,
        max_parts: 32,
        max_part_header_bytes: 4096,
        max_headers_per_part: 32,
        max_field_name_bytes: 128,
        max_filename_bytes: 255,
    };
    let _ = decode("multipart/form-data; boundary=FuzzBoundary", input, limits);
});
