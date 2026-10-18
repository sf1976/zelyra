# Mutable array backend benchmark

This benchmark compares the same CPU-bound Zelyra function on the AST
reference backend, MIR, and Bytecode. It is a reproducible focused benchmark,
not a general performance claim.

## Reproduction

```sh
cargo bench -p zelyra-mir --bench backend_mutable_arrays --locked
```

The release benchmark parses, type-checks, and lowers the program once before
measurement. It performs one warm-up and then nine measurements per backend.
Each round uses AST, MIR, Bytecode order. It reports minimum, median, and
maximum. Every run must return the same value, `20009`. Compiler/lowering time
and process startup are excluded.

## Measurement

- Date: 2026-10-18
- Platform: Linux x86_64, kernel 6.12.111+deb13-amd64
- Environment: virtual AMD EPYC-Milan CPU, 6 visible CPUs
- Rust: `rustc 1.98.1`, Cargo `1.98.1`
- Optimized Cargo bench build, three separate benchmark process runs
- Workload per execution: 20,000 `Int` values, 100,000 indexed `+=` updates
- Nine measurements per backend per run after one warm-up

| Backend | Run 1 median | Run 2 median | Run 3 median | Range across all samples |
| --- | ---: | ---: | ---: | ---: |
| AST | 28.57 ms | 28.10 ms | 29.09 ms | 27.47–34.67 ms |
| MIR | 24.42 ms | 23.21 ms | 23.54 ms | 22.17–32.31 ms |
| Bytecode | 27.91 ms | 27.90 ms | 27.59 ms | 25.64–37.00 ms |

All backends returned `Int(20009)`. The sample maxima show noise in the shared
virtual machine. This benchmark does not measure memory use, cold startup, I/O,
real web requests, or other programs. It therefore does not justify changing
the default backend.
