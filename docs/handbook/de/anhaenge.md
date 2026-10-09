# ANHÄNGE

---

## 24. Verbindliche Quellen und Compiler-Prüfung (Source Authority)

> **Grundsatz:** Zelyra ist eine eigenständige Sprache. Parser und geprüfte Tests entscheiden, was existiert.

Bei Widersprüchen gilt immer folgende verbindliche Reihenfolge:

1. **[Formale Sprachspezifikation](docs/specification.de.md) und Phasendokumente**
2. **Compiler-Code:** Lexer-, AST-, Parser-, Namensauflösungs-, Typprüfungs- und semantischer Code in den Crates `lexer`, `parser`, `ast`, `hir`, `cli`, `runtime`, `database`, `web` und `forms`
3. **Offizielle automatisierte Sprach- und Integrationstests:** Workspace-Tests (`cargo test --workspace`), Machine-Interface-Tests und E2E-Shellskripte in `tests/`
4. **Die offizielle Standardbibliothek:** (sobald eigenständig strukturiert)
5. **Offizielle Zelyra-Beispiele:** `.zyl`-Dateien in `examples/`, die mit dem aktuellen Compiler erfolgreich verifiziert wurden
6. **Dokumentation und Handbuch**

### Wichtige Invarianten für Entwickler und KI-Assistenten

- **Roadmap ist Planung, keine Syntax:** Zukünftige Phasenvorschläge dürfen erst nach Implementierung in Lexer/Parser als verfügbare Syntax dargestellt werden.
- **Compiler-Implementierungssprache Rust ist kein Zelyra:** Zelyra wird in Rust entwickelt, aber Rust-Syntax in einer `.zyl`-Datei ist ungültig, es sei denn, die Zelyra-Grammatik definiert sie ausdrücklich.
- **Keine erfundenen Befehle:** Alle CLI-Befehle müssen in `cli/src/main.rs` existieren.
- **Prüfzyklus:** Jede Erweiterung durchläuft Formatierung (`cargo fmt`), Typ- und Crate-Prüfung (`cargo check`), Linter (`cargo clippy`) und Tests (`cargo test`).

---

## Anhang A: Schnelleinstieg / Spickzettel (Syntax-Cheat-Sheet)

### Grundlegende Syntax
```zelyra
// Funktionen mit Vertraegen
fn summe(a: Int, b: Int) -> Int
    requires { a >= 0 && b >= 0 }
    ensures { result >= 0 }
{
    return a + b
}

// Einstiegspunkt und Variablen
fn main() {
    x = 10                  // Typableitung (unveraenderlich)
    mutable zaehler = 0     // Veraenderlich
    name: String = "Zelyra" // Expliziter Typ

    print(summe(3, 7))
}
```

### Typen
- Zahlen: `Int` (64-Bit vorzeichenbehaftet), `UInt` (vorzeichenlos), `Float`, `Decimal` (Festkomma)
- Text & Zeichen: `String`, `Char`
- Wahrheitswerte: `Bool` (`true`, `false`)
- Sammlungen: `Int[]`, `String[]`
- Abwesenheit: `Option<T>` (`Some(x)`, `None`), Kurzform `T?`
- Fehler: `Result<T, E>` (`Ok(x)`, `Err(e)`)
- System & Zeit: `Timestamp`, `Date`, `Time`, `Duration`

### Kontrollstrukturen
```zelyra
fn kontrolle(x: Int) {
    if x > 10 {
        print("Gross")
    } else {
        print("Klein")
    }

    match x {
        1 => { print("Eins") }
        2 => { print("Zwei") }
        _ => { print("Andere") }
    }

    mutable i = 0
    while i < 3 invariant { i >= 0 } {
        i = i + 1
    }

    for n in [1, 2, 3] {
        print(n)
    }
}

fn main() {
    kontrolle(1)
}
```

### Datenbank & Web
```zelyra
database main {
    engine: mariadb
    database: "app"
}

table items {
    id: Id primary auto
    bezeichnung: String required
}

page "/items" {
    html {
        <h1>Artikelliste</h1>
    }
}
```

---

## Anhang B: Alle Fehlermeldungen von Zelyra auf einen Blick

| Fehlercode | Kategorie | Beschreibung | Typische Behebung |
| :--- | :--- | :--- | :--- |
| `E-LEX-001` | Lexer | Unerwartetes Zeichen / Lexikalischer Fehler | Tippfehler oder unzulässiges Sonderzeichen entfernen |
| `E-PARSE-001` | Parser | Syntaxfehler (z. B. fehlende Klammer, falsches Token) | Syntax gemäß Zelyra-Grammatik korrigieren |
| `E-NAME-001` | Auflösung | Unbekannter Name / Variable nicht gefunden | Deklaration prüfen oder Tippfehler korrigieren |
| `E-TYPE-001` | Typprüfung | Typkonflikt (z. B. String zugewiesen an Int) | Typen anpassen oder Konvertierung vornehmen |
| `E-FEATURE-001` | Feature-Schalter | Zugriff auf eine deaktivierte Sprachoberfläche (`web`, `api`, `crud`, `auth`, `audit`) | Feature in `zelyra.toml` oder `.env` aktivieren |
| `E-CAP-001` / `E-CAP-002` | Capabilities | Fehlende Capability-Berechtigung (z. B. `database`, `network`) | In `zelyra.toml` unter `[capabilities]` freigeben |
| `E-POLICY-001` / `002` | Richtlinien | Verstoß gegen Sicherheits- oder Audit-Richtlinien | Sicherheitsdeklaration prüfen |
| `E-DB-001` - `E-DB-005` | Datenbank | Datenbankverbindungs- oder Treiberfehler | `DATABASE_URL` prüfen, MariaDB-Dienst starten |
| `E-SQL-001` - `E-SQL-004` | SQL | Ungültiges SQL / Schema-Misfit / Spalte nicht existent | SQL-Anweisung gegen Tabellendefinition prüfen |
| `E-VIEW-001` - `E-VIEW-009` | Views & Pages | Fehler in View-Interpolation, Slots oder Datenbindung | Slot-Namen und Datentypen der Page-Bindung prüfen |
| `E-VIEW-010` - `E-VIEW-015` | Query-Controls | Ungültige Such-, Sortier-, Paginierungs- oder Filterfelder | Deklarierte Whitelist (`search`, `sort`, `filter`) prüfen |
| `E-FORM-001` - `E-FORM-004` | Formulare | Validierungsfehler oder ungültige Feldtypen | Formular-Deklaration und Eingabedaten anpassen |
| `E-CRUD-001` - `E-CRUD-006` | CRUD | Ungültige CRUD-Ressource, Schema-Konflikt oder Layout-Fehler | Tabellenverknüpfung und Layout-Slots prüfen |
| `E-AUTH-001` - `E-AUTH-028` | Authentifizierung | Session-, Passwort- oder Berechtigungskonflikt | Rollen (`permits`), `requires auth` und Hashes prüfen |
| `E-AUDIT-001` - `E-AUDIT-010` | Audit-Trail | Fehler in der kryptografischen Hash-Kette des Audit-Logs | Prüfsummen und Audit-Tabelle validieren |
| `E-SETUP-001` - `E-SETUP-006` | Setup-Flow | Portkonflikt, Socket-Fehler oder Compose-Problem | Freie Ports wählen, Docker-Berechtigungen prüfen |
| `E-SETUP-WEB-001` | Web-Setup | Ungültiges oder abgelaufenes Setup-Token | Setup-Assistenten neu starten und Token-URL nutzen |
| `E-IMPACT-001` | Impact-Analyse | Zyklische oder ungültige Abhängigkeiten | Quellcode-Abhängigkeiten entflechten |
| `E-MOD-019` | Modul-/Tabellenabhängigkeit | Tabellenmodul liegt nicht im Importabschluss des Verbrauchers | Tabellenbesitzermodul direkt oder transitiv importieren |
| `E-MOD-020` | Modul-/UI-Abhängigkeit | Referenzierte öffentliche View oder Komponente liegt außerhalb des Importgraphs | UI-Besitzermodul direkt oder transitiv importieren |
| `E-MOD-021` | Modul-/Tabellenfreigabe | Erkannter Lese-/Schreibzugriff hat keinen passenden Grant des Tabellenbesitzers | `access`-Liste der Tabelle und exakten Modulpfad prüfen |
| `E-MOD-022` | Modul-/Datenbankabhängigkeit | Ein datenbanknutzendes Modul importiert den Provider nicht | Datenbankmodul direkt oder transitiv importieren; Konfiguration aus `main.zyl` auslagern |
| `E-RUNTIME-001` | Laufzeit | Unbehandelter Laufzeitfehler | Verträge (`requires`, `ensures`) oder Fehlerwerte prüfen |

---

## Anhang C: Zelyra-CLI-Referenz

| Befehl | Option / Flag | Beschreibung |
| :--- | :--- | :--- |
| `zelyra --version` | | Gibt den vollständigen Compiler- und Paketversionsstand aus |
| `zelyra new <dir>` | `--template minimal\|mariadb-crud\|...` | Erstellt ein neues Zelyra-Projekt mit Vorlage |
| | `--mariadb` | Erzeugt MariaDB-Projekt mit Compose, Dockerfile und `.env` |
| | `--web-port <p> --host-port <p> --db-host-port <p>` | Konfiguriert Container- und Host-Ports |
| `zelyra init` | `[--mariadb]` | Initialisiert das aktuelle Verzeichnis als Zelyra-Projekt |
| `zelyra check <file.zyl>` | `[--format json]` | Prüft Syntax, Typen, Verträge und Capabilities statisch |
| `zelyra run <file.zyl>` | | Kompiliert und führt ein Zelyra-Programm aus |
| `zelyra serve <file.zyl>` | `[host:port]` | Startet den integrierten HTTP-Webserver |
| `zelyra module plan <entry> <module-or-resource>` | | Experimentelle, schreibgeschützte Vorschau bekannter Modulabhängigkeiten |
| `zelyra module bundle <entry> <module-or-resource>` | `--output <dir> [--dry-run] [--docker --compiler-ref <commit>]` | Exportiert ein geprüftes experimentelles Paket oder zeigt vorher einen JSON-Dateiplan; kein Vollständigkeitsnachweis |
| | `--docker --compiler-ref <40-stelliger-Commit>` | Erzeugt Dockerfile, Compose-App und `.env.example`; Abhängigkeitsschluss bleibt unvollständig |
| `zelyra setup` | `[--database]` | Startet Docker Compose / MariaDB |
| | `[--schema]` | Startet Umgebung und wendet Datenbankschema an |
| | `[--all]` | Führt Konfiguration, Start und Migration in einem Schritt aus |
| | `[--host-port <p>] [--db-host-port <p>]` | Setzt verbindliche Host-Ports für die neue `.env` |
| `zelyra setup --web` | `[--port <p>]` | Startet den lokalen, token-geschützten Browser-Setup-Assistenten |
| `zelyra config <file.zyl>` | `[--format json]` | Zeigt die wirksame Konfiguration und Feature-Schalter geheimnisfrei an |
| `zelyra doctor <file.zyl>` | `[--port <p>] [--json]` | Prüft Toolchain, MariaDB, Docker und Ports ohne DB-Änderung |
| | `[--env-file <file>]` | Liest gezielt `DATABASE_URL` aus der angegebenen Datei |
| `zelyra fmt <file.zyl>` | `[--check]` | Formatiert Quellcode nach dem offiziellen Standard |
| `zelyra verify <file.zyl>` | | Führt formale Vertragsverifikation durch |
| `zelyra doc <file.zyl>` | `--openapi` | Generiert OpenAPI-3.0-Spezifikationen |
| | `--typescript` | Generiert typisierten, abhängigkeitsfreien TypeScript-Client |
| `zelyra db init <file.zyl>` | | Initialisiert Datenbank und Basistabellen |
| `zelyra db setup <file.zyl>` | | Richtet die MariaDB-Datenbank initial ein |
| `zelyra db apply <file.zyl>` | | Wendet Schema-Migrationen sicher an |
| `zelyra db history <file.zyl> [--format=json]` | | Zeigt den MariaDB-Migrationsfortschritt |
| `zelyra auth hash-password` | `[--stdin]` | Erzeugt sichere Argon2-Passworthashes |
| `zelyra form validate <file> <Form>` | | Prüft Formulare mit Testwerten auf der Konsole |
| `zelyra context <file.zyl>` | `[--format json]` | Gibt den semantischen Quellcode-Kontext für Tools aus |
| `zelyra module plan <entry> <module-or-resource-id>` | | Zeigt bekannte Abhängigkeiten ab Quelldatei oder Anwendungsressource; kein Deploymentexport |

---

## Anhang D: Die Standardbibliothek im Überblick

### Grundfunktionen (ohne Capabilities)
- `print(wert)`: Gibt einen beliebigen Wert auf der Standardausgabe aus.
- `len(array)`: Liefert die Anzahl der Elemente in einem Array als `Int`.
- `append(array, element)`: Erzeugt ein neues Array mit angehängtem Wert.
- `contains(array, element)` -> `Bool`: Prüft, ob ein Wert im Array enthalten ist.
- `first(array)` -> `Option<T>`: Liefert das erste Element oder `None`.
- `last(array)` -> `Option<T>`: Liefert das letzte Element oder `None`.
- `get(map, key)` -> `Option<V>`: Schlägt einen Schlüssel in einer `Map<K, V>` nach.
- `put(map, key, value)` -> `Map<K, V>`: Fügt ein Schlüssel-Wert-Paar hinzu oder aktualisiert es funktional.
- `keys(map)` -> `K[]`: Liefert alle Schlüssel einer Map als Array.
- `values(map)` -> `V[]`: Liefert alle Werte einer Map als Array.
- `Some(wert)` / `None`: Konstruktoren für den Typ `Option<T>`.
- `Ok(wert)` / `Err(fehler)`: Konstruktoren für den Typ `Result<T, E>`.
- `json_encode(wert)` -> `String`: Wandelt Daten in JSON um.
- `json_decode<T>(text)` -> `T`: Parst typisiertes JSON; ungültige Daten werden als Laufzeitfehler gemeldet.

### Funktionen mit Capabilities
- `uses Console`:
  - `read_console(prompt: String)` -> `String?`: Zeigt den Prompt an und liest eine Zeile; `None` bedeutet EOF.
- `uses Clock`:
  - `now()` -> `Timestamp`: Aktueller Systemzeitstempel.
- `uses Random`:
  - `random_int(min: Int, max: Int)` -> `Int`: Zufallszahl im Intervall.
- `uses Environment`:
  - `env(name: String)` -> `Option<String>`: Liest eine Umgebungsvariable.
- `uses FileSystem`:
  - `read_text(pfad: String)` -> `String`: Liest Dateiinhalt als Text.
  - `write_text(pfad: String, inhalt: String)`: Schreibt Inhalt in Datei.
  - `delete_file(pfad: String)`: Löscht eine Datei.
  - `list_dir(ordner: String)` -> `String[]`: Listet Dateinamen auf.
- `uses Database`:
  - `sql<T[]> { SELECT ... }`: Führt typisierte SQL-Abfragen aus.
  - `transaction { ... }`: Fasst Abfragen transaktional zusammen.

---

## Anhang E: SQL-Spickzettel für Zelyra-Entwickler

In Zelyra eingebettetes SQL wird mit `sql<T[]>` oder `sql` ausgeführt:

```zelyra
database main {
    engine: mariadb
    database: "app"
}

table tasks {
    id: Id primary auto
    name: String required
    erledigt: Bool default false
}

fn sql_beispiele() uses Database {
    // 1. SELECT mit typisiertem Rueckgabetyp und sicherem Parameter
    status = false
    gefiltert = sql<Task[]> {
        SELECT id, name, erledigt
        FROM tasks
        WHERE erledigt = :status
    }

    // 2. INSERT in einer Transaktion
    text = "Neue Aufgabe"
    transaction {
        sql {
            INSERT INTO tasks (name, erledigt)
            VALUES (:text, false)
        }
    }

    // 3. UPDATE
    ziel_id = 1
    transaction {
        sql {
            UPDATE tasks
            SET erledigt = true
            WHERE id = :ziel_id
        }
    }
}

fn main() uses Database {
    print("SQL Spickzettel validiert.")
}
```

---

## Anhang F: HTML- und Web-Referenz in Zelyra

### Web-Strukturen und Deklarationen

| Element | Deklaration | Zweck |
| :--- | :--- | :--- |
| **Page** | `page "/pfad/{param}" { ... }` | Definiert eine HTTP-GET-Route mit Pfadparametern und HTML-Antwort |
| **View-Layout** | `view LayoutName { html { ... <slot /> ... } }` | Wiederverwendbares Layout mit Standard- und benannten Slots |
| **Component** | `component Name { props { ... } html { ... } }` | Wiederverwendbare HTML-Komponente mit typisierten Eigenschaften |
| **Named Slot** | `<slot name="header">Fallback</slot>` | Platzhalter im Layout/Komponente mit optionalem Standardinhalt |
| **Slot Injection** | `<slot name="header">Inhalt</slot>` | Übergabe von Kindinhalten an den passenden Slot |
| **Data Loading** | `load item = sql<Item> { SELECT ... }` | Typisiertes Laden eines Einzeldatensatzes mit Feldzugriff `{item.field}` |
| **Collection Loop**| `for item in items { <li>{item.name}</li> }` | Typisierte serverseitige Iteration über geladene Datensätze |
| **Search Control** | `search { col1 col2 }` | Whitelist-geprüfte URL-Suche mit parametrisierter `LIKE`-Abfrage |
| **Sort Control** | `sort { col1 col2 }` | Typisierte Sortierung über `?sort=col&order=asc\|desc` |
| **Pagination** | `paginated 25` | Paginierung mit `LIMIT`/`OFFSET`, `page`, `pages` und `total` |
| **Filter Control** | `filter { col1 col2 }` | Typisierte Filteroperatoren (`eq`, `contains`, `starts_with`, `gt`, `lte` etc.) |
| **CRUD Layout** | `crud Res { table tbl layout: LayoutName }` | Bindet generierte CRUD-Ansichten in Slots `title`, `nav`, `content`, `actions` ein |

---

## Anhang G: Glossar der Fachbegriffe

- **AST (Abstract Syntax Tree):** Die hierarchische Baumstruktur, in die der Compiler deinen Quellcode übersetzt.
- **Capability (Fähigkeit):** Ausdrückliche Berechtigung (`uses FileSystem`, etc.), ohne die eine Funktion keine geschützten Ressourcen berühren darf.
- **Design by Contract:** Entwurfsmethode, bei der Funktionen über Vorbedingungen (`requires`) und Nachbedingungen (`ensures`) vertraglich abgesichert werden.
- **Immutable (Unveränderlich):** Variablen können nach der ersten Zuweisung nicht mehr verändert werden. In Zelyra Standard, es sei denn, sie werden mit `mutable` deklariert.
- **Invariant:** Eine Bedingung (z. B. in einer Schleife), die vor und nach jedem Durchlauf garantiert wahr sein muss.
- **Option:** Typ (`Some(v)` oder `None`), der das mögliche Fehlen eines Wertes darstellt – Zelyras Antwort auf gefürchtete `null`-Pointer-Crashes.
- **Result:** Typ (`Ok(v)` oder `Err(e)`), der das Scheitern einer Operation als sicheren Wert zurückgibt, statt unkontrollierte Abstürze auszulösen.
- **Typed Hole (`_`):** Platzhalter im Code, der dem Compiler und KI-Werkzeugen signalisiert, an dieser Stelle eine passende Implementierung zu erwarten.

---

## Anhang H: Lösungen zu den Übungsaufgaben der Kapitel

### Kapitel 1: Begrüßung
```zelyra
fn main() {
    print("Hallo Welt aus Zelyra!")
}
```

### Kapitel 5: Rabattpreis berechnen
```zelyra
fn berechne_rabatt(original: Float, prozent: Float) -> Float {
    return original * (1.0 - (prozent / 100.0))
}

fn main() {
    print(berechne_rabatt(100.0, 20.0))
}
```

### Kapitel 11: Zahlen verdoppeln
```zelyra
fn verdopple(zahl: Int) -> Int {
    return zahl * 2
}

fn main() {
    print(verdopple(21))
}
```

### Kapitel 12: Vor- und Nachbedingungen
```zelyra
fn begrenze(wert: Int, min_w: Int, max_w: Int) -> Int
    requires { min_w <= max_w }
    ensures { result >= min_w && result <= max_w }
{
    if wert < min_w { return min_w }
    if wert > max_w { return max_w }
    return wert
}

fn main() {
    print(begrenze(120, 0, 100))
}
```

### Kapitel 13: Array summieren
```zelyra
fn summe_array(zahlen: Int[]) -> Int {
    mutable gesamt = 0
    for z in zahlen {
        gesamt = gesamt + z
    }
    return gesamt
}

fn main() {
    liste: Int[] = [1, 2, 3, 4, 5]
    print(summe_array(liste))
}
```

---

## Anhang I: Häufige Fragen und Antworten (FAQ)

**Frage: Warum gibt es in Zelyra 0.3.0 keine `import`-Anweisung?**
*Antwort:* Das veröffentlichte 0.3.0-Binary enthält keine Modulimporte. Ein
CLI-Aufruf prüft die ausdrücklich angegebene `.zyl`-Datei. Im aktuellen,
unveröffentlichten Entwicklungszweig gibt es experimentelle Imports für
Funktionen, Typen, Records, Tabellen, Views, Komponenten und eine projektweite
Datenbankkonfiguration. Unter anderem `serve` kann den verknüpften
Projektgraphen verwenden; ein vollständiges, stabiles Modul- und Paketmodell
bleibt geplante Arbeit.

**Frage: Kann ich mit Zelyra auch reine Konsolenprogramme schreiben?**
*Antwort:* Ja. `print()` gibt Werte aus. `read_console("Prompt: ")` liest eine Zeile und liefert `String?`; dafür braucht die Funktion `uses Console` und das Projekt gegebenenfalls `console = true`.

**Frage: Warum unterstützt Zelyra MariaDB als bevorzugte Engine?**
*Antwort:* MariaDB bietet herausragende Performance, Open-Source-Freiheit, Stabilität und breite Cloud-Unterstützung für professionelle Webanwendungen.

---

## Anhang J: Weiterführende Ressourcen und Community

- **Offizielles GitHub-Repository:** [https://github.com/sf1976/zelyra](https://github.com/sf1976/zelyra)
- **Dokumentation & Online-Handbuch:** [https://siedelmann.com/handbuch](https://siedelmann.com/handbuch) / [https://siedelmann.com/handbook](https://siedelmann.com/handbook)
- **Beispiele & Vorlagen:** Im Verzeichnis `examples/` des Repositories findest du lauffähige Vorlagen für Authentifizierung, CRUD-Ansichten, APIs und Datenbanken.
