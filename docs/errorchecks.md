# Datenbank- und Schema-Prüfbefund / Database and Schema Findings

## Deutsch

### Ergebnis

Der eingereichte Befund war teilweise zutreffend. Die tatsächlichen Fehler bei optionalen Relationen, mandantengebundenen Fremdschlüsseln und Decimal-Defaults wurden behoben. Zwei weitere Forderungen beschrieben dagegen bereits vorhandene Funktionen oder hätten eine inkompatible Änderung verlangt.

### Behauptete Punkte und Prüfung

| Punkt | Prüfung und Ergebnis |
| --- | --- |
| Optionale Relation, etwa `customer: Customer?` | **Fehler bestätigt und behoben.** Ein optionaler benannter Typ, der einer Tabelle entspricht, wird als nullable Relation gespeichert: `<relation>_id`, Integer-Spaltentyp und Fremdschlüssel. |
| Mandantenübergreifende Relationen | **Fehler bestätigt und behoben.** Wenn Quell- und Zieltabelle dieselbe nominale Mandanten-ID verwenden, plant das Schema einen eindeutigen Zielschlüssel `(tenant_id, id)` und einen zusammengesetzten Fremdschlüssel `(tenant_id, <relation>_id)`. Abweichende nominale Mandantentypen werden nicht gleichgesetzt. |
| Reservierte Feldnamen und SQL-Namensmapping | **Keine fehlende Funktion.** Feldnamen wie `type`, `version`, `order`, `group` und `status` sind im Feldkontext bereits zulässig. Für einen abweichenden physischen SQL-Spaltennamen gibt es `storage_name`. Eine zusätzliche `r#`- oder `@column(...)`-Syntax wäre redundant und ist nicht eingeführt worden. |
| `Decimal(precision, scale)` | **Bereits vorhanden.** Die Tabellensyntax und SQL-Abbildung unterstützen Präzision und Skala. Zusätzlich wurde die Ausgabe eines ganzzahligen Defaults an die deklarierte Skala angepasst, etwa `Decimal(12, 2) default 0` zu `DEFAULT 0.00`. |
| `Money` auf `DECIMAL(12, 2)` setzen | **Nicht übernommen.** Zelyras bestehender `Money`-Standard ist `DECIMAL(19, 4)`. Eine Absenkung würde Wertebereich und Nachkommastellengenauigkeit reduzieren und könnte gespeicherte Daten beziehungsweise bestehende Anwendungen beeinträchtigen. Sie ist keine Fehlerbehebung für `Decimal(p, s)`. |
| Einargumentiges `Decimal(scale)` | **Nicht ergänzt.** Ohne festgelegte Präzision wäre die resultierende SQL-Spalte mehrdeutig. Die explizite Form `Decimal(precision, scale)` bleibt eindeutig. |

### Nullable-Philosophie

`NULL` ist hier die Datenbankabbildung eines ausdrücklich optionalen Zelyra-Typs wie `Customer?`. Die Optionalität bleibt im Zelyra-Quelltext sichtbar und wird vom Typsystem geprüft. Ein nicht optionales Feld wird durch diese Änderung nicht automatisch nullable.

### Nachweise

- Regressionstests decken optionale Relationen, zusammengesetzte Mandanten-Fremdschlüssel, unvereinbare nominale Mandantentypen, Feldnamen und Decimal-Defaults ab.
- Ein MariaDB-Ende-zu-Ende-Test prüft, dass eine gültige Relation innerhalb desselben Mandanten funktioniert und MariaDB eine mandantenübergreifende Relation tatsächlich mit einem Fremdschlüssel-Fehler zurückweist.
- Lokal erfolgreich ausgeführt: `cargo test --all --locked`, `cargo clippy --workspace --locked -- -D warnings`, `cargo fmt --all -- --check`, `cargo check --workspace --locked`, Release-Metadatenprüfung und MariaDB-Ende-zu-Ende-Test.
- Die Implementierung liegt in PR [#115](https://github.com/sf1976/zelyra/pull/115), Branch `fix/1.2.17-relational-schema-safety`, für Version `1.2.17`.
- Der GitHub-CI-Status ist veränderlich und muss vor Merge beziehungsweise Veröffentlichung direkt in PR #115 geprüft werden. Diese Datei behauptet nicht, dass ein noch laufender Check bestanden sei.

## English

### Finding

The submitted report was partly correct. The real defects in optional relations, tenant-scoped foreign keys, and Decimal defaults have been fixed. The remaining requests either described existing functionality or would have introduced an incompatible change.

### Findings by claim

| Claim | Review and result |
| --- | --- |
| Optional relation such as `customer: Customer?` | **Confirmed and fixed.** An optional named type that resolves to a table is stored as a nullable relation using `<relation>_id`, an integer column, and a foreign key. |
| Cross-tenant relations | **Confirmed and fixed.** When source and target tables use the same nominal tenant ID type, the schema planner ensures a unique target key `(tenant_id, id)` and plans a composite foreign key `(tenant_id, <relation>_id)`. Distinct nominal tenant types are not treated as interchangeable. |
| Reserved field names and SQL column mapping | **Not a missing feature.** Names such as `type`, `version`, `order`, `group`, and `status` are already accepted in field position. `storage_name` already maps a Zelyra field to a different physical SQL column name. Adding `r#` or `@column(...)` would be redundant, so neither syntax was introduced. |
| `Decimal(precision, scale)` | **Already supported.** Table syntax and SQL mapping support precision and scale. Integer defaults are now rendered using the declared scale, for example `Decimal(12, 2) default 0` becomes `DEFAULT 0.00`. |
| Change `Money` to `DECIMAL(12, 2)` | **Not adopted.** Zelyra's existing `Money` default is `DECIMAL(19, 4)`. Reducing it would narrow the value range and fractional precision and could affect stored data and existing applications. It is not required to fix `Decimal(p, s)`. |
| One-argument `Decimal(scale)` | **Not added.** Without a defined precision, the resulting SQL column would be ambiguous. The explicit `Decimal(precision, scale)` form remains unambiguous. |

### Nullability model

`NULL` is the database representation of an explicitly optional Zelyra type such as `Customer?`. Optionality remains visible in Zelyra source and is checked by the type system. This change does not make non-optional fields nullable by default.

### Evidence

- Regression tests cover optional relations, composite tenant foreign keys, incompatible nominal tenant types, field names, and Decimal defaults.
- A MariaDB end-to-end test verifies a valid same-tenant relation and confirms that MariaDB rejects a cross-tenant relation with a foreign-key error.
- Passed locally: `cargo test --all --locked`, `cargo clippy --workspace --locked -- -D warnings`, `cargo fmt --all -- --check`, `cargo check --workspace --locked`, release metadata validation, and the MariaDB end-to-end test.
- Implementation review: PR [#115](https://github.com/sf1976/zelyra/pull/115), branch `fix/1.2.17-relational-schema-safety`, targeting version `1.2.17`.
- GitHub CI status changes over time and must be checked directly on PR #115 before merging or releasing. This document does not claim that a still-running check has passed.
