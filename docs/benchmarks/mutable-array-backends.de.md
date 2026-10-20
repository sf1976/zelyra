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

## Vergleich mit Quellschrittmarkierungen v26 — 20.10.2026

Der Vergleich derselben optimierten Benchmarkversion verwendete exakt den
Tag v1.2.28 (`fa54c045`) und den noch unveröffentlichten
Quellschritt-Branch (`de82b6a0`). Beide Läufe nutzten sechs sichtbare
AMD-EPYC-Milan-vCPUs unter KVM, Debian-Kernel `6.12.113+deb13-amd64`, Rust
`1.98.1` und jeweils drei getrennte Prozesse. Jeder Prozess führte einen
Warm-up und neun Messungen je Backend aus.

| Backend | Laufmediane Baseline | Laufmediane Quellschritt-Branch | Median der Laufmediane |
| --- | --- | --- | --- |
| AST | 28,20; 29,38; 30,10 ms | 29,38; 28,42; 28,97 ms | 29,38 → 28,97 ms |
| MIR | 23,36; 24,97; 25,16 ms | 27,01; 23,07; 23,12 ms | 24,97 → 23,12 ms |
| Bytecode | 26,52; 27,68; 26,71 ms | 33,58; 33,78; 32,58 ms | 26,71 → 33,58 ms |

Der Median des Bytecodes ist im Quellschritt-Build bei dieser
CPU-Berechnung um 25,8 % langsamer. Das ist ein gemessenes Regressionssignal,
keine allgemeine Leistungsbehauptung und kein Beweis, dass allein die
Step-Markierungen die Ursache sind. Der Branch bleibt experimentell; dieser
Pfad muss optimiert und erneut gemessen werden, bevor der Benchmark eine
Entscheidung zum Standard-Backend stützen kann. Alle Läufe ergaben den
Sollwert `Int(20009)`.

## Optimierung der Quellschritte in v27 — 20.10.2026

Der v27-Kandidat bettet die häufige Quellschrittmarkierung in die folgende
Instruktion ein und zählt serielle Schritte lokal; parallele Zweige behalten
einen gemeinsamen atomaren Zähler. Verglichen wurde mit dem exakten Tag
v1.2.28 (`fa54c045`) auf demselben AMD-EPYC-Milan-KVM-Host mit sechs vCPUs,
Debian-Kernel `6.12.113+deb13-amd64` und Rust `1.98.1`. Jeder von drei
Prozessläufen nutzte einen Warm-up und neun Messungen je Backend.

| Backend | Laufmediane Baseline | Laufmediane v27 | Median der Laufmediane |
| --- | --- | --- | --- |
| AST | 28,20; 29,38; 30,10 ms | 29,06; 28,47; 27,78 ms | 29,38 → 28,47 ms |
| MIR | 23,36; 24,97; 25,16 ms | 23,23; 23,33; 22,51 ms | 24,97 → 23,23 ms |
| Bytecode | 26,52; 27,68; 26,71 ms | 30,87; 29,94; 30,10 ms | 26,71 → 30,10 ms |

Der Median-Rückstand des Bytecodes gegenüber v1.2.28 liegt bei dieser einen
Arbeitslast nun bei 12,7 %, nach 25,8 % im früheren v26-Kandidaten. v27 hat
diesen Messfall erkennbar verbessert, den Unterschied aber nicht beseitigt.
Das Ergebnis bleibt ein fokussierter CPU-Benchmark: Es belegt weder die
allgemeine Runtime-Leistung noch rechtfertigt es einen Wechsel des
Standard-Backends. Alle Läufe ergaben `Int(20009)`.
