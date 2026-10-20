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

## Source-step v26 comparison — 2026-10-20

A same-host comparison used the exact v1.2.28 tag (`fa54c045`) and the
unreleased source-step branch (`de82b6a0`). Both used the same optimized
benchmark, six visible AMD EPYC-Milan vCPUs under KVM, Debian kernel
`6.12.113+deb13-amd64`, Rust `1.98.1`, and three separate process runs.
Each run contained one warm-up and nine measurements per backend.

| Backend | Baseline run medians | Source-step run medians | Median of run medians |
| --- | --- | --- | --- |
| AST | 28.20, 29.38, 30.10 ms | 29.38, 28.42, 28.97 ms | 29.38 → 28.97 ms |
| MIR | 23.36, 24.97, 25.16 ms | 27.01, 23.07, 23.12 ms | 24.97 → 23.12 ms |
| Bytecode | 26.52, 27.68, 26.71 ms | 33.58, 33.78, 32.58 ms | 26.71 → 33.58 ms |

The median Bytecode result is 25.8% slower in the source-step build on this
CPU-bound workload. This is a measured regression signal, not a general
performance claim or proof that Step markers alone caused it. The source-step
branch remains experimental; optimize and remeasure this path before using
this benchmark to support any default-backend decision. All runs returned the
expected `Int(20009)`.
