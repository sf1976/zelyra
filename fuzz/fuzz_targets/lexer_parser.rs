#![no_main]

use libfuzzer_sys::fuzz_target;
use zelyra_lexer::lex;
use zelyra_parser::parse;

fuzz_target!(|input: &[u8]| {
    if input.len() > 16 * 1024 {
        return;
    }
    let source = String::from_utf8_lossy(input);
    if let Ok(tokens) = lex(&source) {
        let _ = parse(&tokens);
    }
});
