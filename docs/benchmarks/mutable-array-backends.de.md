# Mutable-Array-Backendbenchmark

Diese Messung vergleicht dieselbe CPU-lastige Zelyra-Funktion auf dem AST-
Referenzbackend, MIR und Bytecode. Sie ist ein reproduzierbarer Einzelbenchmark,
keine allgemeine Leistungszusage.

## Wiederholen

```sh
cargo bench -p zelyra-mir --bench backend_mutable_arrays --locked
```

Der Release-Benchmark parst, typprüft und senkt das Programm einmal vor der
Messung. Er führt eine Aufwärmrunde und danach neun Messungen je Backend aus.
Die Backend-Reihenfolge ist pro Runde AST, MIR, Bytecode. Gemeldet werden
Minimum, Median und Maximum. Alle Läufe müssen denselben Rückgabewert `20009`
liefern. Compiler- und Lowering-Zeit sowie Prozessstart sind nicht enthalten.

## Messung

- Datum: 18.10.2026
- Plattform: Linux x86_64, Kernel 6.12.111+deb13-amd64
- Umgebung: virtuelle AMD EPYC-Milan-CPU, 6 sichtbare CPUs
- Rust: `rustc 1.98.1`, Cargo `1.98.1`
- Optimierter Cargo-Bench-Build, drei getrennte Benchmark-Prozessläufe
- Arbeitslast je Ausführung: 20.000 `Int`-Werte, 100.000 indizierte `+=`-Updates
- Je Lauf neun Messungen pro Backend nach einer Aufwärmrunde

| Backend | Median Lauf 1 | Median Lauf 2 | Median Lauf 3 | Gesamtbereich aller Samples |
| --- | ---: | ---: | ---: | ---: |
| AST | 28,57 ms | 28,10 ms | 29,09 ms | 27,47–34,67 ms |
| MIR | 24,42 ms | 23,21 ms | 23,54 ms | 22,17–32,31 ms |
| Bytecode | 27,91 ms | 27,90 ms | 27,59 ms | 25,64–37,00 ms |

Alle Backends lieferten `Int(20009)`. Die Sample-Spitzen zeigen Schwankungen der
gemeinsam genutzten virtuellen Maschine. Der Benchmark misst weder Speicher-
verbrauch noch kalten Start, I/O, reale Webanfragen oder andere Programme.
Die Messung begründet daher keine Backend-Standardänderung.
