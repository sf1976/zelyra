/// Produces a reproducible, size-bounded mix of grammar-seed mutations and
/// arbitrary byte strings. The fixed seed keeps normal CI deterministic.
pub fn corpus(seeds: &[&str], cases: usize, max_mutations: usize) -> Vec<String> {
    let mut state = 0x5a17_2026_d3c4_b2e1_u64;
    let mut output = Vec::with_capacity(cases * 2);
    let alphabet = [
        b' ', b'\n', b'\r', b'\t', b'_', b':', b';', b'(', b')', b'{', b'}', b'[', b']', b'<',
        b'>', b'=', b'!', b'"', b'\'', b'/', b'*', b'?', b'%', b'&', b'|', b'0', b'1', b'a', b'z',
        b'A', b'Z', b'\0', 0x7f, 0x80, 0xff,
    ];

    for case in 0..cases {
        let seed = seeds[case % seeds.len()];
        let mut bytes = seed.as_bytes().to_vec();
        let mutations = (next(&mut state) as usize) % (max_mutations + 1);
        for _ in 0..mutations {
            let operation = next(&mut state) % 3;
            let index = if bytes.is_empty() {
                0
            } else {
                next(&mut state) as usize % bytes.len()
            };
            match operation {
                0 if bytes.len() < 256 => {
                    let byte = alphabet[next(&mut state) as usize % alphabet.len()];
                    bytes.insert(index, byte);
                }
                1 if !bytes.is_empty() => {
                    bytes.remove(index);
                }
                _ if !bytes.is_empty() => {
                    bytes[index] = alphabet[next(&mut state) as usize % alphabet.len()];
                }
                _ => {}
            }
        }
        output.push(String::from_utf8_lossy(&bytes).into_owned());

        let raw_len = next(&mut state) as usize % 193;
        let raw = (0..raw_len)
            .map(|_| alphabet[next(&mut state) as usize % alphabet.len()])
            .collect::<Vec<_>>();
        output.push(String::from_utf8_lossy(&raw).into_owned());
    }
    output
}

fn next(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}
