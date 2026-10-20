# ANHÄNGE

## Anhang A: Schnelleinstieg / Spickzettel (Syntax-Cheat-Sheet)

### Grundlegende Syntax

`# Kommentar` gilt bis zum Zeilenende. Für einen Block stehen öffnendes und schließendes `##` jeweils allein auf einer Zeile; dazwischen steht der Kommentartext.
```zelyra
# Funktionen mit Vertraegen
fn summe(a: Int, b: Int) -> Int
    requires { a >= 0 && b >= 0 }
    ensures { result >= 0 }
{
    return a + b
}

# Einstiegspunkt und Variablen
fn main() {
    x = 10                  # Typableitung (unveraenderlich)
    mutable zaehler = 0     # Veraenderlich
    name: String = "Zelyra" # Expliziter Typ

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
| `zelyra console [entry.zyl]` | `[--sandbox|--persist] [--tenant <id>]` | Startet die interaktive AST-Sprachkonsole; MariaDB/MySQL-Schreibvorgänge werden standardmäßig zurückgerollt |
| `zelyra files backup [entry.zyl]` | `--output <neues-verzeichnis>` | Sichert privaten Zelyra-Dateispeicher samt Manifesten; enthält keine Datenbank, Konfiguration oder Geheimnisse |
| `zelyra files restore [entry.zyl]` | `--from <verzeichnis> --apply` | Prüft Prüfsummen vor dem Wiederherstellen; Datenbank vorher separat wiederherstellen und Zelyra-Server anhalten |
| `zelyra db console [file.zyl]` | | Startet den nativen Datenbankclient mit Projektverbindung |
| `zelyra db query <file.zyl> <SQL>` | `[--format table|json|csv]` | Führt eine geprüfte Abfrage standardmäßig schreibgeschützt aus |
| `zelyra serve <file.zyl>` | `[host:port]` | Startet den integrierten HTTP-Webserver |
| `zelyra routes <entry.zyl>` | `[--format human\|json]` | Listet erkannte Routen mit HTTP-Methode und Quellfundstelle; Kollisionen derselben Methode melden `E-ROUTE-001` |
| `zelyra setup` | `[--database]` | Startet Docker Compose |
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
| `zelyra db create <file.zyl>` | | Erzeugt SQL für die Datenbankeinrichtung |
| `zelyra db setup <file.zyl>` | | Richtet die MariaDB-Datenbank initial ein |
| `zelyra db map <file.zyl>` | `[--format=text\|json]` | Vergleicht schreibgeschützt deklarierte Tabellen mit Live-Tabellen, Spalten und Fremdschlüsseln |
| `zelyra db plan <file.zyl>` | | Zeigt die geplanten Schemaänderungen; prüfe den Plan vor der Migration |
| `zelyra db apply <file.zyl>` | | Wendet Schema-Migrationen sicher an |
| `zelyra auth hash-password` | `[--stdin]` | Erzeugt sichere Argon2-Passworthashes |
| `zelyra form validate <file> <Form>` | | Prüft Formulare mit Testwerten auf der Konsole |
| `zelyra context <file.zyl>` | `[--format json]` | Gibt den semantischen Quellcode-Kontext für Tools aus |
| `zelyra editor [directory]` | `[--port <p>] [--no-open]` | Startet Studio |
| `zelyra --tutorial invoice [directory]` | `[--no-start]` | Erzeugt das Rechnungstutorial |
| `zelyra outbox setup <file.zyl>` | | Richtet die MariaDB-Outbox ein |
| `zelyra outbox list <file.zyl>` | `[--limit <1..200>] [--format text\|json]` | Zeigt Einträge |
| `zelyra outbox run <file.zyl>` | `--handler <event.type=function> [--once]` | Verarbeitet fällige Ereignisse |
| `zelyra outbox requeue <event-id> <file.zyl>` | | Reiht erneut ein |
| `zelyra task <list|run> <file.zyl>` | `[command] [options]` | Listet deklarierte Tasks auf und führt sie compilergeprüft aus |
| `zelyra service <install|status|restart|change|remove>` | `[--port <port>]` | Installiert und verwaltet Linux-systemd-Dienste |
| `zelyra schedule <list|export> <file.zyl>` | `[--format cron\|json]` | Listet residente Zeitpläne auf oder exportiert sie als Cron-Vorschau |
| `zelyra module init <dir> --name <n>` | `[--version <semver>]` | Erstellt ein offline nutzbares lokales Zelyra-Modulpaket |
| `zelyra module list <file.zyl>` | `[--format human\|json]` | Zeigt Quellmodule, Exporte, Prüfsummen und Einstellungs-Schemas |
| `zelyra module settings check <file.zyl>` | `[--format json]` | Validiert typisierte Moduleinstellungen und Geheimnisse |
| `zelyra module plan <entry.zyl> <target>` | | Ermittelt den statischen Modulabschluss und Abhängigkeitsgraphen |
| `zelyra module bundle <entry> <target>` | `--output <dir> [--docker]` | Schnürt ein portables Quellbundle mit isolierten Paketabhängigkeiten |
| `zelyra package lock [file.zyl]` | `[--check\|--update]` | Prüft oder aktualisiert die SHA-256-Integrität in `zelyra.lock` |
| `zelyra package keygen\|sign\|verify` | `[options]` | Erstellt Ed25519-Schlüssel und signiert/prüft Quellarchive (`.zpkg`) |
| `zelyra package trust <add\|list\|remove>` | `[publisher] [key-hex]` | Verwaltet autorisierte Herausgeberschlüssel in `zelyra.trust.toml` |
| `zelyra package registry build <dir>` | | Erzeugt einen statischen Index (`index.json`) aus `.zpkg`-Dateien |
| `zelyra package search <url> [filter]` | | Durchsucht einen statischen Paketindex nach Namen |
| `zelyra package add <url> <pkg> <ver>` | `--as <alias>` | Installiert eine exakte, vertrauensgeprüfte Paketversion |
| `zelyra package update <url> <pkg> <ver>` | `--as <alias> [--plan <file>]` | Aktualisiert Paketabhängigkeiten deterministisch |
| `zelyra package remove <alias>` | | Entfernt eine Paketabhängigkeit und aktualisiert das Lockfile |
| `zelyra package cache clean` | `[--apply]` | Prüft und bereinigt unreferenzierte Cache-Versionen sicher |
| `zelyra package recover` | `[--apply]` | Stellt nach abgebrochenen Transaktionen Manifest und Lock wieder her |
| `zelyra reflect <entry.zyl>` | `[--format=json\|human]` | Exportiert Symbole, Schemata, Routen und Contracts als JSON |
| `zelyra audit <inspect\|export\|verify\|prune>` | `<file.zyl> [options]` | Prüft, verifiziert oder bereinigt Audit-Logs der Anwendung |
| `zelyra migrate-layout --check` | `[entry.zyl] [--format human\|json]` | Prüft die Konformität mit dem kanonischen Standard-Layout |
| `zelyra db history <file.zyl>` | | Zeigt die angewendete Schema-Migrationshistorie |
| `zelyra update` | `[--check]` | Prüft oder installiert das stabile Release |

Weitere REPL-Befehle, Transaktionsgrenzen und Einschränkungen: [Sprachkonsole](../../language-console.de.md).

---

## Anhang D: Die Standardbibliothek im Überblick (Zelyra 1.0.3 bis 1.0.7)

Die eingebaute Zelyra-Standardbibliothek umfasst Funktionen für Mathematik, Textverarbeitung, Sammlungsoperationen, Mengen, Maps, Datumsrechnung, Fehlerkontrolle, Web-Sicherheit, Encodings, Pfade sowie PDF-Rendering. Pipeline-Stufen werden typgeprüft und unterstützen seit 1.0.7 lexikalische Lambdas. Funktionen, die auf externe Systemressourcen zugreifen (z. B. Zufall, Dateisystem, Uhrzeit, E-Mail), erfordern wie gewohnt die entsprechende Capability.

### 1. Mathematik & Numerik (ohne Capabilities)
- `abs(x: Int | Float | Decimal) -> Int | Float | Decimal`: Liefert den Betrag; Überlauf beim kleinsten `Int` oder `Decimal` sowie nicht-endliche `Float`-Werte ergeben einen Laufzeitfehler.
- `min(a, b)` und `max(a, b)`: Wählen den kleineren oder größeren Wert aus zwei gleich typisierten Zahlen (`Int`, `UInt`, `Float`, `Decimal`) oder Strings. Nicht-endliche `Float`-Werte ergeben einen Laufzeitfehler.
- `clamp(val, min_val, max_val)`: Begrenzt eine gleich typisierte Zahl (`Int`, `UInt`, `Float`, `Decimal`) auf `[min_val, max_val]`; nicht-endliche `Float`-Werte oder `min_val > max_val` ergeben einen Laufzeitfehler.
- `sqrt(x: Float) -> Float`: Berechnet die Quadratwurzel einer endlichen, nicht-negativen Fließkommazahl; negative und nicht-endliche Werte ergeben einen Laufzeitfehler.
- `pow(base: Float, exp: Float) -> Float`: Berechnet `base ^ exp`; undefinierte oder nicht-endliche Ergebnisse ergeben einen Laufzeitfehler.
- `floor(x: Float) -> Float` und `ceil(x: Float) -> Float`: Runden eine endliche Fließkommazahl ab beziehungsweise auf.
- `round(x: Float, decimals: Int) -> Float`: Rundet Halbwerte weg von null; `decimals` muss zwischen `-15` und `15` liegen. Ein nicht-endliches Ergebnis ergibt einen Laufzeitfehler.

### 2. Strings & Textverarbeitung
- `len(text: String) -> Int`: Liefert die Anzahl der Unicode-Codepoints im String.
- `text_to_lower(text: String) -> String`: Konvertiert den gesamten Text in Kleinbuchstaben.
- `text_to_upper(text: String) -> String`: Konvertiert den gesamten Text in Großbuchstaben.
- `text_starts_with(text: String, prefix: String) -> Bool`: Prüft, ob der Text mit `prefix` beginnt.
- `text_ends_with(text: String, suffix: String) -> Bool`: Prüft, ob der Text mit `suffix` endet.
- `text_replace(text: String, search: String, replacement: String) -> String`: Ersetzt alle Vorkommen von `search` durch `replacement`.
- `text_split(text: String, delimiter: String) -> String[]` (Alias: `split`): Zerlegt den Text am Trennzeichen in ein Array.
- `text_pad_left(text: String, target_len: Int, pad: String) -> String` und `text_pad_right(...)`: Füllen auf die Ziellänge in Unicode-Codepoints auf; ein mehrstelliges `pad` wird codepointweise wiederholt. Leere Fülltexte verwenden Leerzeichen. Negative Längen und Ausgaben über 16 MiB ergeben Laufzeitfehler.
- `text_index_of(text: String, sub: String) -> Option<Int>`: Liefert den nullbasierten Unicode-Codepoint-Index des ersten Vorkommens oder `None`.
- `text_repeat(text: String, count: Int) -> String`: Wiederholt den Text `count`-mal; negative Wiederholungen oder Ausgaben über 16 MiB ergeben Laufzeitfehler.
- `text_is_empty(text: String) -> Bool`: Prüft, ob der Text exakt die Länge 0 hat.
- `text_is_blank(text: String) -> Bool`: Prüft, ob der Text nach Entfernung von Whitespace leer ist.
- `text_capitalize(text: String) -> String`: Wandelt das erste Zeichen in einen Großbuchstaben um.
- `text_substring(text: String, start: Int, length: Int) -> String`: Schneidet einen Teilstring ab Position `start` mit Maximallänge `length` aus.
- `text_trim(text: String) -> String`: Entfernt führende und nachgestellte Whitespaces.
- `text_trim_start(text: String) -> String`: Entfernt führende Whitespaces.
- `text_trim_end(text: String) -> String`: Entfernt nachgestellte Whitespaces.
- `text_contains(text: String, sub: String) -> Bool`: Prüft, ob `sub` im Text enthalten ist.
- `text_chars(text: String) -> String[]`: Zerlegt den Text in einzelne Zeichen als Array.

### 3. Arrays & Sammlungen
- `len(array: T[]) -> Int`: Liefert die Anzahl der Elemente im Array.
- `append(array: T[], element: T) -> T[]`: Liefert ein neues Array mit angehängtem Element.
- `concat(a: T[], b: T[]) -> T[]`: Verbindet zwei Arrays desselben Typs.
- `contains(array: T[], element: T) -> Bool`: Prüft, ob das Element im Array vorhanden ist.
- `first(array: T[]) -> Option<T>`: Liefert das erste Element oder `None` bei leerem Array.
- `last(array: T[]) -> Option<T>`: Liefert das letzte Element oder `None` bei leerem Array.
- `sort(array: T[]) -> T[]`: Sortiert die Elemente aufsteigend.
- `sort_desc(array: T[]) -> T[]`: Sortiert die Elemente absteigend.
- `reverse(array: T[]) -> T[]`: Kehrt die Reihenfolge der Elemente um.
- `slice(array: T[], start: Int, length: Int) -> T[]`: Extrahiert einen Teilbereich des Arrays.
- `join(array: T[], delimiter: String) -> String`: Verbindet alle Elemente als String mit dem Trennzeichen.
- `unique(array: T[]) -> T[]` (Alias: `dedup`): Liefert ein neues Array ohne Duplikate unter Beibehaltung der ersten Vorkommen.
- `array_sum(array: T[]) -> T`: Berechnet die Summe von `Int`, `Float` oder `Decimal`; Ganzzahl- und Dezimalüberläufe sowie nicht-endliche Fließkommawerte ergeben einen Laufzeitfehler. Für ein leeres Array ist das Ergebnis `0`.
- `avg(array: T[]) -> Float | Decimal`: Berechnet den Mittelwert von `Int`- und `Float`-Arrays als `Float`, bei `Decimal`-Arrays als `Decimal`. Der Dezimalmittelwert wird mit der festen Decimal-Skala abgeschnitten; ein leeres Array und Überläufe ergeben einen Laufzeitfehler.
- `min_of(array: T[]) -> Option<T>` und `max_of(array: T[]) -> Option<T>`: Wählen das Minimum beziehungsweise Maximum aus Zahlen oder Strings; ein leeres Array ergibt `None`, nicht-endliche Float-Werte ergeben einen Laufzeitfehler.
- `chunk(array: T[], size: Int) -> T[][]`: Teilt das Array in Teil-Arrays der angegebenen Höchstgröße auf; `size` muss größer als null sein.
- `flatten(array: T[][]) -> T[]`: Ebnet ein verschachteltes zweidimensionales Array in ein flaches Array ein.
- `zip(a: T[], b: U[]) -> Tuple<T, U>[]`: Kombiniert Arrays elementweise bis zur Länge des kürzeren Arrays.
- `map(array: T[], transform: Function<(T) -> U>) -> U[]`: Transformiert jedes Element eines Arrays (unterstützt seit 1.0.7 lexikalische Lambdas wie `|x| x * 2`).
- `filter(array: T[], predicate: Function<(T) -> Bool>) -> T[]`: Filtert Elemente eines Arrays nach einer Prädikatsfunktion oder einem Lambda (`|x| x > 0`).
- `find(array: T[], predicate: Function<(T) -> Bool>) -> Option<T>`: Findet das erste Element, das die Bedingung erfüllt, oder liefert `None`.
- `reduce(array: T[], initial: U, accumulator: Function<(U, T) -> U>) -> U`: Faltet das Array mit einem Anfangswert und einer Akkumulatorfunktion zusammen.

### 4. Mengen (Sets)
- `set_union(a: Set<T>, b: Set<T>) -> Set<T>`: Berechnet die Vereinigungsmenge `A ∪ B`.
- `set_intersection(a: Set<T>, b: Set<T>) -> Set<T>`: Berechnet die Schnittmenge `A ∩ B`.
- `set_difference(a: Set<T>, b: Set<T>) -> Set<T>`: Berechnet die Differenzmenge `A \ B`.

### 5. Maps & Wörterbücher
- `get(map: Map<K, V>, key: K) -> Option<V>`: Schlägt den Schlüssel nach und liefert `Some(V)` oder `None`.
- `get_or(map: Map<K, V>, key: K, default: V) -> V` (Alias: `get_or_default`): Liefert den Wert zum Schlüssel oder den Standardwert `default`.
- `has_key(map: Map<K, V>, key: K) -> Bool`: Prüft, ob der Schlüssel in der Map existiert.
- `put(map: Map<K, V>, key: K, value: V) -> Map<K, V>`: Fügt ein Paar hinzu oder aktualisiert es funktional.
- `map_remove(map: Map<K, V>, key: K) -> Map<K, V>`: Liefert eine neue Map ohne den angegebenen Schlüssel.
- `map_merge(a: Map<K, V>, b: Map<K, V>) -> Map<K, V>`: Verschmilzt zwei Maps; bei Kollisionen überschreiben Werte aus `b`.
- `map_increment(map: Map<K, Int>, key: K) -> Result<Map<K, Int>, String>`: Erhöht einen typgeprüften Zähler um eins oder fügt ihn mit Startwert `1` ein. Der Eingabewert bleibt unverändert; ein `Int`-Überlauf wird als `Err` zurückgegeben.
- `keys(map: Map<K, V>) -> K[]`: Liefert alle Schlüssel der Map als Array.
- `values(map: Map<K, V>) -> V[]`: Liefert alle Werte der Map als Array.
- `entries(map: Map<K, V>) -> Tuple<K, V>[]`: Liefert Schlüssel-Wert-Paare als Tupel-Array, das sich mit `for (schluessel, wert) in entries(map)` typgeprüft zerlegen lässt.

### 6. Datums- & Kalenderberechnung
- `date_add_days(d: Date, days: Int) -> Date`: Addiert oder subtrahiert Kalendertage von einem `Date` (`YYYY-MM-DD`); Ergebnisse müssen zwischen den Jahren 0001 und 9999 liegen.
- `date_diff_days(start: Date, end: Date) -> Int`: Berechnet `start - end` in Kalendertagen. Für verbleibende Tage daher `date_diff_days(frist, heute)` aufrufen.
- `is_before(a: Date, b: Date) -> Bool`: Prüft, ob Datum `a` chronologisch vor `b` liegt.
- `is_after(a: Date, b: Date) -> Bool`: Prüft, ob Datum `a` chronologisch nach `b` liegt.

### 7. Ablauf-, Option- und Result-Kontrolle
- `is_ok(res: Result<T, E>) -> Bool`: Prüft, ob das Resultat ein Erfolg `Ok(...)` ist.
- `is_err(res: Result<T, E>) -> Bool`: Prüft, ob das Resultat ein Fehler `Err(...)` ist.
- `is_some(opt: Option<T>) -> Bool`: Prüft, ob die Option einen Wert `Some(...)` enthält.
- `is_none(opt: Option<T>) -> Bool`: Prüft, ob die Option `None` ist.
- `unwrap_or(opt: Option<T>, default: T) -> T`: Entpackt den Wert oder liefert den Fallback `default`.
- `panic(msg: String) -> Never`: Bricht die Programmausführung mit einer kontrollierten Laufzeitfehlermeldung ab.
- `else if`: Flache Verzweigungskaskaden ohne erzwungene tiefe Blockschachtelung.
- `|>` (Pipeline-Operator): Übergibt den linken Ausdruck als erstes Argument an den rechten Funktionsaufruf: `daten |> sort() |> join(", ")`.

### 8. Typkonvertierung & Parsing
- `to_string(value: Any) -> String`: Wandelt jeden beliebigen Zelyra-Wert in seine String-Darstellung um.
- `to_int(value: Int | UInt | Float | Bool | String) -> Int`: Konvertiert zu `Int`; Fließkommazahlen werden gegen null abgeschnitten. Nicht endliche oder außerhalb des `Int`-Bereichs liegende Zahlen und ungültige Strings ergeben einen Laufzeitfehler.
- `to_float(value: Float | Int | UInt | String) -> Float`: Konvertiert zu einer endlichen Fließkommazahl; ungültige oder nicht-endliche Werte ergeben einen Laufzeitfehler.
- `parse_int(text: String) -> Result<Int, String>`: Parst einen String sicher als `Int` oder liefert `Err(String)`.
- `parse_float(text: String) -> Result<Float, String>`: Parst einen String als endliche `Float`-Zahl oder liefert `Err(String)`.
- `parse_bool(text: String) -> Result<Bool, String>`: Parst `"true"`, `"false"`, `"1"`, `"0"` in einen `Bool`; Groß-/Kleinschreibung der Wörter ist egal, ungültige Werte liefern `Err(String)`.

### 9. Sicherheit, Hashing & Web-Encodings
- `hash_sha256(text: String) -> String`: Berechnet den hexadezimalen SHA-256-Hash des Strings.
- `base64_encode(text: String) -> String`: Kodiert den UTF-8-String in Base64.
- `base64_decode(encoded: String) -> Result<String, String>`: Dekodiert einen Base64-String sicher.
- `url_encode(text: String) -> String`: Prozent-kodiert Sonderzeichen für URL-Parameter nach RFC 3986.
- `url_decode(encoded: String) -> Result<String, String>`: Dekodiert Prozentkodierungen sicher; `+` wird als Leerzeichen behandelt.
- `uuid_v4() -> String` (`uses Random`): Erzeugt eine kryptografisch sichere Version-4-UUID nach RFC 4122.

### 10. Pfade & Dateisystem
- `path_join(a: String, b: String) -> String`: Verbindet zwei Pfadsegmente nach den Pfadkonventionen des Hostbetriebssystems (ohne Capability).
- `path_filename(path: String) -> String`: Extrahiert den Dateinamen aus einem Pfad (ohne Capability).
- `path_extension(path: String) -> String`: Extrahiert die Dateiendung ohne führenden Punkt (ohne Capability).
- `path_dir(path: String) -> String`: Extrahiert das Elternverzeichnis eines Pfades (ohne Capability).
- `file_exists(path: String) -> Bool` (`uses FileSystem`): Prüft innerhalb der erlaubten Lesepfade, ob eine Datei existiert.
- `dir_exists(path: String) -> Bool` (`uses FileSystem`): Prüft innerhalb der erlaubten Lesepfade, ob ein Verzeichnis existiert.
- `file_size(path: String) -> Result<Int, String>` (`uses FileSystem`): Liefert die Dateigröße in Bytes.

### 11. Capability-geschützte System-APIs
- `uses Console`:
  - `read_console(prompt: String) -> String?`: Gibt den Prompt auf `stdout` aus und liest eine Zeile von `stdin`; `None` signalisiert EOF.
- `uses Clock`:
  - `now() -> Timestamp`: Liefert den aktuellen Systemzeitstempel in Millisekunden seit der Unix-Epoche.
- `uses Random`:
  - `random_int(min: Int, max: Int) -> Int`: Erzeugt eine ganzzahlige Pseudozufallszahl im geschlossenen Bereich `[min, max]`.
  - `uuid_v4() -> String`: Erzeugt eine kryptografisch sichere Version-4-UUID.
- `uses Environment`:
  - `env(name: String) -> Option<String>`: Liest den Wert einer Systemumgebungsvariablen aus.
- `uses FileSystem`:
  - `read_text(path: String) -> String`: Liest den gesamten Inhalt einer UTF-8-Textdatei.
  - `write_text(path: String, content: String)`: Schreibt Text in eine Datei und überschreibt vorhandenen Inhalt innerhalb der freigegebenen Schreibpfade.
  - `delete_file(path: String)`: Löscht eine Datei innerhalb der Projektfreigaben.
  - `list_dir(dir: String) -> String[]`: Listet die Dateinamen eines Verzeichnisses auf.
  - `file_exists(path: String) -> Bool`: Prüft die Existenz einer Datei.
  - `dir_exists(path: String) -> Bool`: Prüft die Existenz eines Ordners.
  - `file_size(path: String) -> Result<Int, String>`: Liest die Größe einer Datei in Bytes.
- `uses Network`:
  - `http_get(url: String) -> String`: Führt einen synchronen HTTP-GET-Request gegen erlaubte Hosts aus.
  - `http_request(method: String, url: String, headers: String[], body: String?) -> HttpResponse`: Führt eine HTTP-Anfrage mit Methode, URL, Headern und optionalem Text-Body aus.
  - `http_json<Req, Res>(method: String, url: String, headers: String[], body: Req?) -> Res`: Sendet eine HTTP-Anfrage und dekodiert eine erfolgreiche JSON-Antwort.
  - `http_result<Req, Res>(method: String, url: String, headers: String[], body: Req?) -> HttpResult<Res>`: Liefert typisierte HTTP-Ergebnisse einschließlich Statuscode und Headern.
- `uses Process`:
  - `run_process(command: String, args: String[]) -> ProcessResult`: Startet einen explizit freigegebenen Prozess ohne Shell.
- `uses Database`:
  - `sql<T[]> { SELECT ... }`: Führt typisierte relationale Abfragen mit parametrisierten Platzhaltern aus.
  - `transaction { ... }`: Fasst mehrere Schreib- und Leseabfragen in einer atomaren Datenbanktransaktion zusammen.
- `uses Email`:
  - `render_pdf(view_name: String, data: Record) -> Bytes` (neu in 1.0.7): Rendert eine deklarierte View mit Record-Daten lokal in ein paginiertes Text-PDF.
  - `mail_queue(to, subject, text, filename, pdf_bytes)` (neu in 1.0.7): Reiht innerhalb einer Datenbank-Transaktion eine E-Mail mit transaktionalem PDF-Anhang in die Outbox ein.

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
    # 1. SELECT mit typisiertem Rueckgabetyp und sicherem Parameter
    status = false
    gefiltert = sql<Task[]> {
        SELECT id, name, erledigt
        FROM tasks
        WHERE erledigt = :status
    }

    # 2. INSERT in einer Transaktion
    text = "Neue Aufgabe"
    transaction {
        sql {
            INSERT INTO tasks (name, erledigt)
            VALUES (:text, false)
        }
    }

    # 3. UPDATE
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


### Zelyra UI: Designsystem & CSS-Architektur (ab Version 1.0.6)

Ab Zelyra 1.0.6 enthält die Plattform ein vollwertiges, compilergeprüftes UI- und CSS-Framework für datenbankgestützte Geschäftsanwendungen. Es ersetzt unübersichtliche externe Abhängigkeiten durch ein lokal gebündeltes Designsystem (`zelyra-ui.v1.css`).

#### CSS-Architektur mit Standard `@layer`

Das Stylesheet ist hierarchisch in sieben standardkonforme CSS-Schichten strukturiert:
```css
@layer reset, base, theme, layout, components, utilities, overrides;
```
1. **`reset`:** Normalisierung moderner HTML5-Elemente ohne aggressive Resets.
2. **`base`:** Typografie, Standard-Zeilenabstände und semantische Standardelemente.
3. **`theme`:** Design Tokens und Farbsysteme (`light`, `dark`, `system`) über CSS Custom Properties.
4. **`layout`:** App-Shells (`.z-app-shell`), Header, responsive Raster und Navigation.
5. **`components`:** 55 wiederverwendbare Komponenten (Cards, Buttons, Inputs, Badges, Tabs, Alerts, Modals, Tables, Charts).
6. **`utilities`:** Präzise Hilfsklassen für Abstände, Ausrichtungen und Textfluss.
7. **`overrides`:** Sichere projektlokale Anpassungen (aus `zelyra.theme.css`).

#### Design Tokens & CSS Custom Properties

| Token | Standardwert (Light) | Dunkelwert (Dark) | Verwendung |
| :--- | :--- | :--- | :--- |
| `--z-color-primary` | `#2f54eb` | `#a8bfff` | Primäraktionen, Links, Fokus-Rahmen |
| `--z-color-surface` | `#ffffff` | `#1b232d` | Karten- und Container-Hintergrund |
| `--z-color-canvas` | `#f4f6f9` | `#141a22` | Gesamter Seitenhintergrund |
| `--z-color-text` | `#1f2937` | `#edf2f8` | Haupttext |
| `--z-color-border` | `#d9dfe7` | `#3a4655` | Trennlinien und Rahmen |
| `--z-color-success` | `#10b981` | `#8dd6ad` | Erfolgsmeldungen und Validierungs-Feedback |
| `--z-color-warning` | `#f59e0b` | `#f0ca75` | Warnhinweise und Fälligkeitswarnungen |
| `--z-color-danger` | `#ef4444` | `#ffafb2` | Fehlerzustände und Löschaktionen |

#### Themes, Dichte und Barrierefreiheit

Die Steuerung erfolgt über semantische Root-Attribute am `<html>`- oder Shell-Tag:
- `data-z-theme="light"`: Erzwingt helles Farbschema.
- `data-z-theme="dark"`: Erzwingt dunkles Farbschema (angepasster Kontrast, blendfrei).
- `data-z-theme="system"`: Passt sich automatisch den Systemeinstellungen des Nutzers an (`prefers-color-scheme`).
- `data-z-density="compact"`: Reduziert Padding und Steuerelementhöhen (`--z-control-height: 2.25rem`) für datenintensive CRUD-Listen.
- `prefers-reduced-motion`: Schaltet Animationen und Übergangszeiten automatisch auf `0ms` für barrierearme Nutzung.

#### 55 Typisierte Komponenten (`zelyra/ui`)

Das Modul `zelyra/ui` stellt typisierte Standardkomponenten für Geschäftsoberflächen bereit:
- **Layout & Navigation:** `AppShell`, `PageHeader`, `Breadcrumbs`, `NavList`, `Tabs`, `Grid`, `Stack`.
- **Daten & Feedback:** `Card`, `Badge`, `Alert`, `EmptyState`, `Progress`, `Spinner`.
- **Formulare & Interaktion:** `Button`, `LinkButton`, `InputField`, `SelectField`, `CheckboxField`, `ConfirmationForm`.
- **Berichte & Dokumente:** `DataTable`, `DonutChart`, `BarChart`, `LineChart`, `EmailLink`, `PrintTrigger`.

#### Druck- und Dokumenten-Styling (`@media print`)

Für Rechnungen, Lieferscheine und Audit-Berichte enthält Zelyra UI optimierte Druckregeln:
- Ausblenden von Navigation, Suchfeldern und Aktionsschaltflächen (`.z-no-print`).
- Feste Seitenumbrüche (`page-break-inside: avoid` für Positionstabellen).
- Optimierung für den standardmäßigen Browserdialog („Als PDF speichern“), ohne fehleranfällige native PDF-Server-Binaries vorauszusetzen.

#### Stylesheet-Treiber (`ZELYRA_UI_STYLESHEET`)

Über die Umgebungskonfiguration lässt sich das Stylesheet-Verhalten steuern:
- `ZELYRA_UI_STYLESHEET="default"`: Zelyra UI v1 (empfohlen).
- `ZELYRA_UI_STYLESHEET="tabler"`: Integriertes Tabler-CSS (Rückwärtskompatibilität).
- `ZELYRA_UI_STYLESHEET="./assets/custom.css"`: Sicheres, projektrelatives Stylesheet. Externe URLs (`http://...`) werden zum Schutz vor Supply-Chain-Angriffen strikt abgewiesen.

---

## Anhang G: Glossar der Fachbegriffe

Die Einträge verweisen auf das Kapitel, in dem der Begriff erläutert wird. Nicht unterstützte Sprachkonstrukte und CLI-Befehle sind als solche gekennzeichnet.

- **Zelyra UI:** Das in Zelyra 1.0.6 eingeführte, direkt im Compiler und Binary gebündelte Designsystem mit 55 typisierten UI-Komponenten, Barrierefreiheit, Theme- und Dichte-Steuerung. [Anhang F: HTML- und Web-Referenz](#anhang-f-html-und-web-referenz-in-zelyra).
- **Appearance (Darstellungsvariante):** Compilergeprüfte semantische Darstellungsattribute (`default`, `success`, `warning`, `danger`) für UI-Komponenten und Formularfelder. [Kapitel 26: Webseiten ausgeben](#kapitel-26-webseiten-ausgeben).
- **Density (Dichte):** Umschaltbare Darstellungsdichte (`data-z-density="compact"`), die Abstände und Zeilenhöhen für datenintensive Tabellen und Geschäftsübersichten optimiert. [Anhang F: HTML- und Web-Referenz](#anhang-f-html-und-web-referenz-in-zelyra).
- **CSS @layer:** Moderner CSS-Standard zur geordneten Schichtung von Reset, Base, Theme, Layout, Components, Utilities und Overrides, der Style-Konflikte strukturell verhindert. [Anhang F: HTML- und Web-Referenz](#anhang-f-html-und-web-referenz-in-zelyra).
- **Wert (Value):** Eine benannte Information. In Zelyra sind Werte standardmäßig unveränderlich (immutable), um Seiteneffekte und unerwartete Zustandsänderungen auszuschließen. [Kapitel 5: Werte und Datentypen](#kapitel-5-werte-und-datentypen).
- **mutable (Veränderbarkeit):** Schlüsselwort zur bewussten Deklaration veränderlicher Variablen. Signalisiert Entwicklern und Compiler sofort, dass sich dieser Zustand im Programmverlauf ändern wird. [Kapitel 6: Variablen und Unveränderlichkeit](#kapitel-6-variablen-und-unveranderlichkeit).
- **Funktion (fn):** Eine benannte Operation mit expliziten Parametertypen und einem festen Rückgabetyp. Funktionen sind die primären Bausteine jeder Zelyra-Anwendung. [Kapitel 11: Funktionen und Prozeduren](#kapitel-11-funktionen-und-prozeduren).
- **Reine Funktion (Pure Function):** Funktion ohne Seiteneffekte: Ihr Ergebnis hängt nur von den Eingaben ab. Gleiche Eingaben liefern dasselbe Ergebnis. [Kapitel 11: Funktionen und Prozeduren](#kapitel-11-funktionen-und-prozeduren).
- **Typinferenz (Type Inference):** Die Fähigkeit des Zelyra-Compilers, Datentypen automatisch aus Literalen und Ausdrücken abzuleiten. Verbindet statische Typsicherheit mit leichtem, lesbarem Code. [Kapitel 5: Werte und Datentypen](#kapitel-5-werte-und-datentypen).
- **Typed Hole (_):** Der Unterstrich `_` als typisierter Platzhalter. Zeigt Compiler und KI-Werkzeugen an, dass hier eine Implementierung erwartet wird, deren Typ aus dem Kontext abgeleitet wird. [Kapitel 32: KI-Nativität](#kapitel-32-ki-nativitat-warum-zelyra-perfekt-fur-ki-assistenten-ist).
- **Block & Scope:** Lexikalischer Gültigkeitsbereich von Bezeichnern, eingefasst durch geschweifte Klammern `{ ... }`. Innerhalb des Blocks deklarierte Namen sind an diesen Bereich gebunden. [Kapitel 6: Variablen und Unveränderlichkeit](#kapitel-6-variablen-und-unveranderlichkeit).
- **Datentyp (Type):** Eine exakte Spezifikation zulässiger Werte und Operationen. Zelyra unterscheidet skalare Typen (Int, Float, Bool, String, Decimal, Id) und zusammengesetzte Typen. [Kapitel 5: Werte und Datentypen](#kapitel-5-werte-und-datentypen).
- **Option (Some / None):** Sicherer Container für Werte, die vorhanden sein können (`Some(v)`) oder fehlen (`None`). Das Suffix `?` (z. B. `String?`) macht Abwesenheit zur Typpflicht. [Kapitel 18: Option-Muster](#kapitel-18-das-nichts-existiert-nicht-der-sichere-umgang-mit-option).
- **Result (Ok / Err):** Typ für Operationen, die fehlschlagen können. Er gibt Erfolg als `Ok(daten)` und einen Fehler als `Err(grund)` explizit zurück. [Kapitel 17: Result-Muster](#kapitel-17-fehler-als-werte-das-result-muster).
- **Null-Safety (Kein null):** In Zelyra existiert kein `null`, `nil` oder `undefined`. Die sogenannte „Milliarden-Dollar-Fehlerquelle“ von Null-Pointer-Abstürzen ist im Sprachkern strukturell unmöglich. [Kapitel 18: Option-Muster](#kapitel-18-das-nichts-existiert-nicht-der-sichere-umgang-mit-option).
- **Pattern Matching (match):** Vollständige Fallunterscheidung von Datenstrukturen, Optionen und Ergebnissen. Der Compiler verweigert die Kompilierung, wenn ein möglicher Zweig vergessen wurde. [Kapitel 14: Eigene Datentypen](#kapitel-14-eigene-datentypen-erstellen-records-tables).
- **Nominale Fachtypen (Domain Types):** Definition semantisch getrennter Typen wie `type UserId = Id` und `type OrderId = Id`. Verhindert versehentliches Vertauschen gleichartiger ID-Werte in der Fachlogik. [Kapitel 14: Eigene Datentypen](#kapitel-14-eigene-datentypen-erstellen-records-tables).
- **Datensatz (Record / Struct):** Zusammenfassung mehrerer benannter, typisierter Felder zu einer kohärenten Datenstruktur mit Konstruktoren und Destrukturierung. [Kapitel 14: Eigene Datentypen](#kapitel-14-eigene-datentypen-erstellen-records-tables).
- **Aufzählungstyp (Enum / Sum Type):** Ein Aufzählungstyp beschreibt eine feste Menge disjunkter Varianten. Benutzerdefinierte `enum`-Deklarationen gehören derzeit nicht zum dokumentierten Zelyra-Sprachumfang; das Konzeptmuster ist keine ausführbare Zelyra-Syntax. [Kapitel 14: Eigene Datentypen](#kapitel-14-eigene-datentypen-erstellen-records-tables).
- **Design by Contract:** Entwurfsmethode zur formalen Absicherung von Software: Jede Funktion deklariert präzise Vorbedingungen (`requires`) und Nachbedingungen (`ensures`). [Kapitel 12: Verträge und Vorbedingungen](#kapitel-12-vertrage-und-vorbedingungen-design-by-contract).
- **Vorbedingung (requires):** Vertragliche Klausel am Funktionskopf, die gültige Eingabewerte erzwingt, bevor die Funktion ausgeführt wird. Schützt Funktionen vor unzulässigen Aufrufen. [Kapitel 12: Verträge und Vorbedingungen](#kapitel-12-vertrage-und-vorbedingungen-design-by-contract).
- **Nachbedingung (ensures & result):** Vertragliche Garantie für das Berechnungsergebnis. Das Schlüsselwort `result` bezieht sich auf den zurückgegebenen Wert und sichert dessen Korrektheit zu. [Kapitel 12: Verträge und Vorbedingungen](#kapitel-12-vertrage-und-vorbedingungen-design-by-contract).
- **Schleifeninvariante (invariant):** Zusicherung innerhalb einer Schleife, die vor und nach jedem einzelnen Schleifendurchlauf garantiert wahr sein muss. Ermöglicht formale Korrektheitsbeweise. [Kapitel 12: Verträge und Vorbedingungen](#kapitel-12-vertrage-und-vorbedingungen-design-by-contract).
- **Capability (Fähigkeit):** Ausdrückliche Berechtigung (`uses FileSystem`, `uses Database`), ohne die eine Funktion keine externen Seiteneffekte ausführen darf. Verhindert unbefugte Systemzugriffe. [Kapitel 33: Capabilities](#kapitel-33-sicherheit-durch-fahigkeiten-capabilities).
- **Sandboxing & Least Privilege:** Das Capability-Modell begrenzt Systemzugriffe auf die Fähigkeiten, die Funktionen anfordern und die für das Projekt freigegeben sind. [Kapitel 33: Capabilities](#kapitel-33-sicherheit-durch-fahigkeiten-capabilities).
- **Array (Typ[]):** Geordnete Liste gleichartiger Werte. Zelyra stellt dafür Operationen wie `len()`, `append()`, `first()`, `last()` und `contains()` bereit. [Kapitel 13: Arrays & Maps](#kapitel-13-sammlungen-listen-und-worterbucher-arrays-maps).
- **Wörterbuch / Map (Map<K, V>):** Assoziativer Speicher für Schlüssel-Wert-Paare. Das Nachschlagen via `get(map, key)` liefert stets eine sichere `Option<V>`, um Abstürze bei fehlenden Schlüsseln zu verhindern. [Kapitel 13: Arrays & Maps](#kapitel-13-sammlungen-listen-und-worterbucher-arrays-maps).
- **Unveränderliche Listenoperationen:** Unveränderliche Sammlungsoperationen ändern den bestehenden Wert nicht direkt, sondern liefern für Änderungen einen neuen Wert zurück. [Kapitel 13: Arrays & Maps](#kapitel-13-sammlungen-listen-und-worterbucher-arrays-maps).
- **for .. in Iteration:** Schleifenform, die nacheinander über Elemente einer Sammlung läuft. Sie erspart manuelle Indexverwaltung. [Kapitel 10: Wiederholungen und Schleifen](#kapitel-10-wiederholungen-und-schleifen).
- **Tabelle (table):** Direkte Deklaration relationaler Schemata in Zelyra-Syntax inklusive Spaltentypen, Primärschlüsseln (`primary`), Unikatsregeln (`unique`) und Defaults. [Kapitel 24: Tabellen definieren](#kapitel-24-tabellen-definieren-und-daten-modellieren).
- **Migration (db plan & apply):** Kontrollierte Schemaänderung: `zelyra db plan` zeigt den Unterschied zwischen Quellmodell und Datenbank; `zelyra db apply` übernimmt den geprüften Plan. Prüfe vorher mögliche Auswirkungen, denn ein abgebrochener DDL-Schritt kann einen teilweise geänderten Zustand hinterlassen. [Kapitel 24: Tabellen definieren](#kapitel-24-tabellen-definieren-und-daten-modellieren).
- **CRUD-Ressource (crud):** Create, Read, Update, Delete: Deklarative Verknüpfung einer Tabelle mit den 4 Grundoperationen. Generiert Listen-, Detail- und Editiermasken ohne Boilerplate. [Kapitel 28: CRUD-Muster](#kapitel-28-das-vollstandige-crud-muster).
- **Natives SQL (sql { ... }):** Vollwertiges SQL direkt im Code, das zur Compile-Zeit gegen das deklarierte Schema validiert wird. Kombiniert die Ausdrucksstärke von SQL mit absoluter Typprüfung. [Kapitel 25: Daten abfragen](#kapitel-25-daten-abfragen-und-verandern).
- **Transaktion (transaction):** Atomare Bündelung mehrerer Datenbankbefehle: Alle Schreiboperationen gelingen gemeinsam oder werden bei einem Fehler vollständig zurückgerollt. [Kapitel 25: Daten abfragen](#kapitel-25-daten-abfragen-und-verandern).
- **Fremdschlüssel (fk):** Deklarative Beziehungsdefinition zwischen Tabellen. Erzwingt referentielle Integrität und unterstützt Kaskadierungsregeln (`on_delete`). [Kapitel 24: Tabellen definieren](#kapitel-24-tabellen-definieren-und-daten-modellieren).
- **Komponente (component):** Wiederverwendbarer HTML-Baustein mit deklarierten Props, Slots und kontextabhängigem XSS-Schutz. HTML wird als erstklassiger Sprachbestandteil gerendert. [Kapitel 26: Webseiten ausgeben](#kapitel-26-webseiten-ausgeben).
- **Slot (slot: name):** Platzhalter in Layouts und Containern, der beim Aufruf mit dynamischen Kind-Elementen oder spezifischen UI-Aktionen befüllt werden kann. [Kapitel 26: Webseiten ausgeben](#kapitel-26-webseiten-ausgeben).
- **Formular & Validierung (form):** Typisierte Eingabemaske mit synchronisierten Validierungsregeln für Server und Client. Validierungsfehler werden strukturiert an das UI zurückgemeldet. [Kapitel 27: Formulare](#kapitel-27-formulare-und-benutzereingaben).
- **CSRF-Schutz:** Automatischer kryptografischer Token-Schutz gegen Cross-Site-Request-Forgery bei allen statusverändernden HTTP-Anfragen (POST, PUT, DELETE). [Kapitel 27: Formulare](#kapitel-27-formulare-und-benutzereingaben).
- **Route:** Explizite Zuordnung einer HTTP-Methode und URL-Maske zu einem Funktions-Controller oder einer CRUD-Ressource. [Kapitel 26: Webseiten ausgeben](#kapitel-26-webseiten-ausgeben).
- **Authentifizierung (auth):** Natives Sprachkonstrukt zur Konfiguration von Benutzer-Login, Sitzungen und Rollenrechten ohne externe Authentifizierungs-Frameworks. [Kapitel 29: Authentifizierung](#kapitel-29-benutzer-passworter-und-sitzungen).
- **Argon2 (Passwort-Hashing):** Moderner, speicherharter Algorithmus zur sicheren Speicherung von Passwörtern. In Zelyras Auth-System standardmäßig erzwungen. [Kapitel 29: Authentifizierung](#kapitel-29-benutzer-passworter-und-sitzungen).
- **requires auth & permits:** Deklarativer Zugriffsschutz an Routen: Weist unbefugte Aufrufe automatisch mit HTTP 401/403 ab oder leitet zur Anmeldeseite weiter. [Kapitel 29: Authentifizierung](#kapitel-29-benutzer-passworter-und-sitzungen).
- **uses FileSystem:** Sicherheits-Capability für Festplattenoperationen. Jede Funktion, die Dateien liest oder schreibt, muss dieses Recht explizit anfordern. [Kapitel 20: Dateien](#kapitel-20-arbeiten-mit-dateien).
- **read_text & write_text:** Standardfunktionen für das sichere Lesen und Schreiben ganzer UTF-8-Dateien mit automatischem Ressourcen-Cleanup und Result-Fehlerbehandlung. [Kapitel 20: Dateien](#kapitel-20-arbeiten-mit-dateien).
- **uses Clock & now():** Systemzeit-Capability: Funktionen, die aktuelle Zeitstempel ermitteln (`now()`), müssen `uses Clock` deklarieren. Macht Zeit in Tests deterministisch mockbar. [Kapitel 21: Datum & Zeit](#kapitel-21-datum-uhrzeit-zufall-und-strukturierte-daten).
- **uses Random & random_int():** Zufalls-Capability: Erlaubt das Erzeugen pseudozufälliger Werte. Trennt deterministische Berechnungen klar von nicht-deterministischen Zufallswerten. [Kapitel 21: Datum & Zeit](#kapitel-21-datum-uhrzeit-zufall-und-strukturierte-daten).
- **uses Environment & env():** Umgebungsvariablen-Capability: `env("KEY")` liest Konfigurationswerte aus dem System oder `.env` und liefert ein sicheres `Option<String>`. [Kapitel 37: Konfiguration](#kapitel-37-konfiguration-und-umgebungsvariablen).
- **parallel & await:** Ein `parallel`-Block startet unabhängige Aufgaben und wartet mit `await` auf ihre Ergebnisse. Der Gültigkeitsbereich der Aufgaben bleibt im Block sichtbar. [Kapitel 22: Nebenläufigkeit](#kapitel-22-nebenlaufigkeit-und-hintergrundaufgaben).
- **json_encode & json_decode:** Typsichere Umwandlung von Datenstrukturen in JSON und Rückwandlung in typisierte Records (`Result<T, JsonError>`). Schließt Parsing-Crashes aus. [Kapitel 21: JSON & Daten](#kapitel-21-datum-uhrzeit-zufall-und-strukturierte-daten).
- **zelyra.toml (Projekt-Manifest):** Projektmanifest für Metadaten, Capability-Freigaben und unterstützte Projekteinstellungen. Zelyra-Module werden per Projektpfad eingebunden; ein Paketregister ist nicht Teil des dokumentierten Umfangs. [Kapitel 15: Module & Projekte](#kapitel-15-module-und-code-organisation).
- **zelyra verify (Vertragsprüfung):** CLI-Befehl für die dokumentierten Vertrags- und Invariantenprüfungen. Die formale Analyse gilt für den unterstützten Prüfumfang und ersetzt keine allgemeine Korrektheitsgarantie. [Kapitel 19: Qualitätssicherung](#kapitel-19-tests-und-qualitatssicherung).
- **assert (Test-Zusicherung):** Prüft eine Bedingung und meldet einen Fehler, wenn sie falsch ist. Projektbezogene Assertions können in einer ausführbaren `.zyl`-Datei mit `zelyra run` ausgeführt werden; ein separater Test-Unterbefehl ist nicht dokumentiert. [Kapitel 19: Qualitätssicherung](#kapitel-19-tests-und-qualitatssicherung).
- **Source Authority (Verbindliche Quellen):** Regel zur Klärung widersprüchlicher Quellen. Die Handbuchreferenz nennt in dieser Reihenfolge Sprachspezifikation, Compilerimplementierung, offizielle Tests, Standardbibliothek, verifizierte Beispiele und Dokumentation. [Referenz 24: Source Authority](#24-verbindliche-quellen-und-compiler-prufung-source-authority).
- **Drei-Schichten-Architektur:** Empfohlenes Zelyra-Architekturmuster: Saubere Trennung in Präsentationsschicht (Web/HTML), reine Geschäftslogik (pure functions) und Persistenz (Tabellen & Schema). [Kapitel 36: Architektur](#kapitel-36-architektur-und-saubere-codestruktur).
- **TDD (Testgetriebene Entwicklung):** Entwicklungsmethode, bei der Tests vor oder zusammen mit der Implementierung geschrieben werden. Sie unterstützt schrittweises Feedback, garantiert aber keinen fehlerfreien Code. [Kapitel 19: Qualitätssicherung](#kapitel-19-tests-und-qualitatssicherung).
- **Regressionsschutz:** Schutz vor dem Wiederauftreten früherer Fehler. Statische Prüfungen, Vertragsverifikation und projektbezogene Testläufe helfen, unbeabsichtigte Änderungen nach einem Refactoring zu erkennen. [Kapitel 19: Qualitätssicherung](#kapitel-19-tests-und-qualitatssicherung).
- **AST (Abstract Syntax Tree):** Der hierarchische Syntaxbaum, den der Parser aus dem Quelltext aufbaut. Compilerphasen wie Namensauflösung und Typprüfung arbeiten auf seiner strukturierten Repräsentation weiter. [Referenz 24: Source Authority](#24-verbindliche-quellen-und-compiler-prufung-source-authority).
- **Zelyra CLI (zelyra):** Das Kommandozeilenprogramm für Zelyra-Projekte: Es legt Projekte mit `new` an, prüft Quellcode mit `check`, führt Programme mit `run` aus und bietet weitere dokumentierte Unterbefehle. Ein eigener `test`-Unterbefehl ist in der aktuellen CLI-Referenz nicht aufgeführt. [Kapitel 4: Das erste Zelyra-Projekt](#kapitel-4-das-erste-zelyra-projekt).
- **zelyrac (Compiler):** Der Compiler verarbeitet Zelyra-Quellcode hinter der CLI `zelyra`. Die aktuelle CLI-Referenz dokumentiert kein separates Programm oder den Befehl `zelyrac build`; zum Prüfen und Ausführen dienen `zelyra check` und `zelyra run`. [Kapitel 4: Das erste Zelyra-Projekt](#kapitel-4-das-erste-zelyra-projekt).
- **zelyra fmt (Formatierer):** Integrierter Quelltext-Formatierer mit deterministischer Ausgabe. `--check` prüft die Formatierung, ohne die Datei zu ändern. [Kapitel 4: Das erste Zelyra-Projekt](#kapitel-4-das-erste-zelyra-projekt).
- **LSP (Language Server Protocol):** Das Language Server Protocol standardisiert die Verbindung zwischen Programmiersprache und Editor. Die aktuelle CLI-Referenz dokumentiert `zelyra editor` für Studio, aber keinen separaten `zelyra lsp`-Unterbefehl. [Kapitel 4: Das erste Zelyra-Projekt](#kapitel-4-das-erste-zelyra-projekt).
- **KI-Nativität (AI-Native):** Sprach- und Werkzeugmerkmale wie strukturierte Diagnosen, Verträge und Typed Holes können KI-gestützte Arbeit unterstützen. Sie garantieren weder fehlerfreien Code noch ersetzen sie menschliche Prüfung. [Kapitel 32: KI-Nativität](#kapitel-32-ki-nativitat-warum-zelyra-perfekt-fur-ki-assistenten-ist).
- **Standardbibliothek (std):** Überblick über die dokumentierten Standardfunktionen und Capability-geschützten Funktionen wie Dateisystem, Zeit, Zufall und Datenbank. [Anhang D: Standardbibliothek](#anhang-d-die-standardbibliothek-im-uberblick-zelyra-103).

## Anhang H: Lösungen zu den Übungsaufgaben der Kapitel

Willkommen in der vollständigen **Lösungsmöglichkeitsübersicht**! Hier findest du zu jedem der 43 Kapitel des Lehrbuchs didaktisch aufbereitete, praxiserprobte und syntaktisch verifizierte Lösungsvorschläge für alle Übungsaufgaben (Stufe 1 bis 3).

> [!TIP]
> **Der Ehrenkodex für Entwickler:** Wirf erst dann einen Blick in diese Lösungen, wenn du mindestens eine frische Tasse Kaffee lang selbst getüftelt, deinen Code mit `zelyra check` herausgefordert und vielleicht die eine oder andere freundliche Belehrung des Compilers eingesteckt hast. Das echte Programmierer-Glücksgefühl stellt sich erst ein, wenn der Compiler nach eigener Denkarbeit endlich lautlos nickt!

---

### Kapitel 1: Willkommen bei Zelyra

- **Stufe 1 (Leicht – Persönliche Begrüßung):**
```zelyra
fn main() {
    vorname = "Stefan"
    wohnort = "Berlin"
    print("Hallo Welt! Ich bin " + vorname + " aus " + wohnort + " und programmiere mit Zelyra.")
}
```

- **Stufe 2 (Mittel – Abschiedsfunktion):**
```zelyra
fn begruessen(name: String) -> String {
    return "Guten Tag, " + name + "! Bereit fuer saubere Typen und null Bugs?"
}

fn verabschieden(name: String) -> String {
    return "Auf Wiedersehen, " + name + "! Moegen deine Builds immer gruen bleiben."
}

fn main() {
    benutzer = "Alex"
    print(begruessen(benutzer))
    print(verabschieden(benutzer))
}
```

- **Stufe 3 (Anspruchsvoll – Schutz vor Tippfehlern):**
In dynamischen Skriptsprachen führen Tippfehler wie `preis * 1.19` (wenn `preis` als `"100"` als Text übergeben wurde) oft erst zur Laufzeit beim Kunden zum Absturz oder zu kuriosen Ergebnissen wie `"100100..."`. Typische Probleme:
1. Vertauschen von Typen (Text statt Zahl) führt zu Rechenfehlern.
2. Unbemerktes Weiterreichen von `null` stürzt erst 20 Zeilen später in fremden Modulen ab.
3. Falsch geschriebene Eigenschaftsnamen (`user.emial`) werden stillschweigend ignoriert und liefern `undefined`.
Zelyras Compiler prüft diese Beziehungen vor der Ausführung und garantiert, dass Typen, Feldnamen und Signaturen exakt zusammenpassen.

---

### Kapitel 2: Wie ein Programm funktioniert

- **Stufe 1 (Leicht – Ablaufstruktur):**
Der Kontrollfluss verläuft streng sequenziell:
1. Einstiegspunkt: `main()` wird vom Betriebssystem aufgerufen.
2. Variablen-Initialisierung: `aktueller_tag` und `ziel_tag` werden unveränderlich an Werte gebunden.
3. Funktionsaufruf: `tage_bis_ziel(...)` wird mit diesen Argumenten aufgerufen; der Ausführungskontext wechselt in die Funktion.
4. Rückgabe: Der berechnete Wert wird an `main()` zurückgegeben und via `print()` auf der Standardausgabe ausgegeben.

- **Stufe 2 (Mittel – Fristprüfung mit if):**
```zelyra
fn tage_bis_ziel(aktueller_tag: Int, ziel_tag: Int) -> Int {
    if aktueller_tag > ziel_tag {
        return 0
    }
    return ziel_tag - aktueller_tag
}

fn main() {
    print(tage_bis_ziel(10, 25)) # Ausgabe: 15
    print(tage_bis_ziel(30, 25)) # Ausgabe: 0 (Frist bereits abgelaufen)
}
```

- **Stufe 3 (Anspruchsvoll – Ahead-of-Time-Prüfung vs. Interpreter):**
Ein reiner Interpreter liest Code Zeile für Zeile. Ein fataler Syntax- oder Typfehler in einer selten genutzten Fehlerbehandlungsroutine (z. B. Zeile 500) wird erst dann bemerkt, wenn ein Benutzer diesen seltenen Zweig betritt. Zelyra prüft mit `zelyra check` das gesamte Programm vorab vollständig (Namensauflösung, Typkonsistenz, Verträge), wodurch versteckte Fehler bereits vor der Auslieferung eliminiert werden.

---

### Kapitel 3: Zelyra installieren und einrichten

- **Stufe 1 (Leicht – Diagnose aufrufen):**
```bash
zelyra doctor
zelyra --version
```
*Erklärung:* `zelyra doctor` analysiert die Laufzeitumgebung, prüft die Verfügbarkeit von MariaDB/Docker Compose v2 und zeigt die installierte Release-Version an.

- **Stufe 2 (Mittel – JSON-Diagnosen für Werkzeuge):**
```bash
zelyra check --help
zelyra check main.zyl --format json
```
*Erklärung:* Mit `--format json` liefert der Compiler maschinenlesbare Fehlermeldungen mit exakten Spalten-, Zeilen- und Fehlercodes, ideal für CI/CD-Pipelines oder IDE-Erweiterungen.

- **Stufe 3 (Anspruchsvoll – Editor-Integration):**
In Editoren wie Visual Studio Code reicht in der Datei `settings.json` der Eintrag:
```json
{
  "files.associations": {
    "*.zyl": "zelyra"
  }
}
```
Zusammen mit der im Downloadbereich verfügbaren Syntaxerweiterung (`zelyra.vsix`) erhältst du sofort Syntax-Highlighting und Einrückungsregeln.

---

### Kapitel 4: Das erste Zelyra-Projekt

- **Stufe 1 (Leicht – Neues Projekt erzeugen):**
```bash
zelyra new mein_erstes_projekt
cd mein_erstes_projekt
zelyra run main.zyl
```

- **Stufe 2 (Mittel – Projekt-Manifest anpassen):**
In `zelyra.toml`:
```toml
[project]
name = "mein_erstes_projekt"
version = "0.3.0"
authors = ["Stefan Siedelmann"]
```
In `main.zyl`:
```zelyra
fn main() {
    app_titel = "Aufgaben-Zentrale"
    app_version = "0.3.0"
    print(app_titel + " v" + app_version + " erfolgreich gestartet.")
}
```

- **Stufe 3 (Anspruchsvoll – Quelltextformatierung):**
```bash
zelyra fmt main.zyl
zelyra fmt --check main.zyl
```
*Erklärung:* `zelyra fmt` vereinheitlicht Einrückungen und Leerzeichen deterministisch. `--check` liefert den Exit-Code `0`, wenn der Code bereits perfekt formatiert ist.

---

### Kapitel 5: Werte und Datentypen

- **Stufe 1 (Leicht – Film-Metadaten):**
```zelyra
fn main() {
    titel: String = "Inception"
    erscheinungsjahr: Int = 2010
    im_kino_gesehen: Bool = true

    print("Film: " + titel)
}
```

- **Stufe 2 (Mittel – Exakte Kostenberechnung mit Decimal):**
```zelyra
fn berechne_gesamtkosten(stunden: Decimal, stundensatz: Decimal) -> Decimal {
    return stunden * stundensatz
}

fn main() {
    arbeitszeit = decimal("3.500000000")
    satz = decimal("95.000000000")
    gesamt = berechne_gesamtkosten(arbeitszeit, satz)
    print(gesamt)
}
```

- **Stufe 3 (Anspruchsvoll – Nominale Fachtypen):**
Wenn sowohl `BenutzerId` als auch `AufgabenId` einfache Ganzzahlen (`Int`) sind, kann eine Funktion `zuweisen(user: Int, task: Int)` versehentlich als `zuweisen(aufgabe_id, benutzer_id)` aufgerufen werden. Beide sind Ganzzahlen, der Compiler schweigt, aber die Logik ist zerstört.
Mit nominalen Typen wie `type UserId = Id` und `type TaskId = Id` erkennt der Compiler die Verwechslung sofort als Typfehler `E-TYPE-001`.

---

### Kapitel 6: Variablen und Unveränderlichkeit

- **Stufe 1 (Leicht – Unveränderlicher Benutzername):**
```zelyra
fn main() {
    benutzername = "senior_coder_42"
    print("Angemeldeter Benutzer: " + benutzername)
}
```

- **Stufe 2 (Mittel – Punktestand verwalten):**
```zelyra
fn main() {
    mutable punktestand = 100
    print("Start-Punktestand: " + punktestand)

    punktestand = punktestand - 15
    print("Nach Strafe (-15):  " + punktestand)

    punktestand = punktestand + 30
    print("Nach Bonus (+30):   " + punktestand)
}
```

- **Stufe 3 (Anspruchsvoll – Unveränderlichkeit bei Nebenläufigkeit):**
Wenn mehrere Threads oder Hintergrund-Tasks gleichzeitig denselben Speicherbereich verändern können, entstehen sogenannte Race Conditions (Wettlaufsituationen): Das Ergebnis hängt davon ab, welcher Thread den Wert zuletzt überschreibt. Unveränderliche Werte (`immutable`) können gefahrlos von beliebig vielen Aufgaben gleichzeitig gelesen werden, ohne dass sperrige Sperren (Locks/Mutexes) nötig sind.

---

### Kapitel 7: Operatoren und Ausdrücke

- **Stufe 1 (Leicht – Gerade-Zahl-Prüfung):**
```zelyra
fn main() {
    ist_gerade = (10 % 2 == 0)
    print(ist_gerade) # Ausgabe: true
}
```

- **Stufe 2 (Mittel – Altersbereich prüfen):**
```zelyra
fn im_arbeitsfaehigen_alter(alter: Int) -> Bool {
    return alter >= 18 && alter <= 65
}

fn main() {
    print(im_arbeitsfaehigen_alter(25)) # true
    print(im_arbeitsfaehigen_alter(16)) # false
}
```

- **Stufe 3 (Anspruchsvoll – Rabatt-Logik):**
```zelyra
fn qualifiziert_fuer_rabatt(is_vip: Bool, bestellwert: Decimal, ist_neukunde: Bool) -> Bool {
    return is_vip || (bestellwert > decimal("100.000000000") && !ist_neukunde)
}

fn main() {
    print(qualifiziert_fuer_rabatt(false, decimal("150.000000000"), false)) # true
    print(qualifiziert_fuer_rabatt(false, decimal("150.000000000"), true))  # false (Neukunde)
    print(qualifiziert_fuer_rabatt(true, decimal("20.000000000"), true))   # true (VIP sticht immer)
}
```

---

### Kapitel 8: Ein- und Ausgaben

- **Stufe 1 (Leicht – Formatierte Visitenkarte):**
```zelyra
fn main() {
    print("================================")
    print(" Name:  Dr. Stefanie Code")
    print(" Beruf: Lead Software Architect")
    print(" Mail:  stefanie@example.com")
    print("================================")
}
```

- **Stufe 2 (Mittel – Aufgaben-Formatierung):**
```zelyra
fn drucke_aufgabe(id: Int, name: String, erledigt: Bool) {
    status_label = if erledigt { "[X] ERLEDIGT" } else { "[ ] OFFEN" }
    print("----------------------------------------")
    print("Aufgabe #" + id + ": " + name)
    print("Status:      " + status_label)
    print("----------------------------------------")
}

fn main() {
    drucke_aufgabe(42, "Refactoring der Rechnungsmodule", false)
}
```

- **Stufe 3 (Anspruchsvoll – Interaktives Terminal vs. Web-Requests):**
`read_console()` erfordert die Capability `uses Console` und blockiert den ausführenden Thread, bis der Benutzer Enter drückt. Das eignet sich hervorragend für CLI-Tools, Setup-Assistenten und Wartungsskripte. In einer Webanwendung wäre eine blockierende Konsoleneingabe katastrophal: Hier müssen Benutzereingaben als strukturierte HTTP-POST-Requests über Formulare oder JSON-Payloads asynchron verarbeitet werden.

---

### Kapitel 9: Entscheidungen mit Bedingungen

- **Stufe 1 (Leicht – Volljährigkeitsprüfung):**
```zelyra
fn ist_volljaehrig(alter: Int) -> Bool {
    return alter >= 18
}

fn main() {
    print(ist_volljaehrig(17)) # false
    print(ist_volljaehrig(18)) # true
}
```

- **Stufe 2 (Mittel – Monatstage mit match):**
```zelyra
fn monats_tage(monat: Int) -> Int {
    return match monat {
        1 => 31,
        2 => 28,
        3 => 31,
        4 => 30,
        5 => 31,
        6 => 30,
        7 => 31,
        8 => 31,
        9 => 30,
        10 => 31,
        11 => 30,
        12 => 31,
        _ => 0,
    }
}

fn main() {
    print(monats_tage(2))  # 28
    print(monats_tage(10)) # 31
    print(monats_tage(99)) # 0 (Ungültiger Monat)
}
```

- **Stufe 3 (Anspruchsvoll – Passwort-Prüfung):**
```zelyra
fn pruefe_passwort_einfach(pw: String) -> Bool {
    if pw == "12345678" || pw == "passwort" || pw == "admin123" {
        return false
    }
    return len(pw) >= 8
}

fn main() {
    print(pruefe_passwort_einfach("12345678"))       # false
    print(pruefe_passwort_einfach("kurz"))           # false
    print(pruefe_passwort_einfach("meinSicheres#26")) # true
}
```

---

### Kapitel 10: Wiederholungen und Schleifen

- **Stufe 1 (Leicht – Countdown):**
```zelyra
fn main() {
    mutable count = 10
    while count >= 1 {
        print(count)
        count = count - 1
    }
    print("Rakete gestartet!")
}
```

- **Stufe 2 (Mittel – Array summieren):**
```zelyra
fn main() {
    zahlen: Int[] = [10, 20, 30, 40]
    mutable summe = 0
    for z in zahlen {
        summe = summe + z
    }
    print("Gesamtsumme: " + summe) # 100
}
```

- **Stufe 3 (Anspruchsvoll – Suche mit break):**
```zelyra
fn main() {
    liste: Int[] = [3, 9, 14, 42, 55, 88]
    mutable gefunden = false

    for element in liste {
        if element == 42 {
            gefunden = true
            break
        }
    }

    if gefunden {
        print("Gefunden! Die Antwort auf alle Fragen ist im Array.")
    } else {
        print("Nicht gefunden")
    }
}
```

---

### Kapitel 11: Funktionen und Prozeduren

- **Stufe 1 (Leicht – Verdopplung):**
```zelyra
fn verdopple(zahl: Int) -> Int {
    return zahl * 2
}

fn main() {
    print(verdopple(21)) # 42
}
```

- **Stufe 2 (Mittel – Dringlichkeitsbewertung):**
```zelyra
fn ist_dringend(verbleibende_tage: Int) -> Bool {
    return verbleibende_tage < 3
}

fn main() {
    print(ist_dringend(1)) # true
    print(ist_dringend(5)) # false
}
```

- **Stufe 3 (Anspruchsvoll – Statussymbol):**
```zelyra
fn status_symbol(erledigt: Bool) -> String {
    if erledigt {
        return "[OK]"
    }
    return "[OFFEN]"
}

fn main() {
    print(status_symbol(true))  # [OK]
    print(status_symbol(false)) # [OFFEN]
}
```

---

### Kapitel 12: Verträge und Vorbedingungen (Design by Contract)

- **Stufe 1 (Leicht – Quadratwurzel mit Vorbedingung):**
```zelyra
fn quadratwurzel_naiv(x: Int) -> Int
    requires { x >= 0 }
{
    mutable r = 0
    while (r + 1) * (r + 1) <= x {
        r = r + 1
    }
    return r
}

fn main() {
    print(quadratwurzel_naiv(49)) # 7
}
```

- **Stufe 2 (Mittel – Wertbereichsbegrenzung):**
```zelyra
fn begrenze(wert: Int, min_wert: Int, max_wert: Int) -> Int
    requires { min_wert <= max_wert }
    ensures { result >= min_wert && result <= max_wert }
{
    if wert < min_wert { return min_wert }
    if wert > max_wert { return max_wert }
    return wert
}

fn main() {
    print(begrenze(125, 0, 100)) # 100
    print(begrenze(-10, 0, 100)) # 0
    print(begrenze(42, 0, 100))  # 42
}
```

- **Stufe 3 (Anspruchsvoll – Prozentsatz ohne Division durch 0):**
```zelyra
fn prozentsatz(teil: Int, gesamt: Int) -> Int
    requires { gesamt > 0 && teil >= 0 && teil <= gesamt }
    ensures { result >= 0 && result <= 100 }
{
    return (teil * 100) / gesamt
}

fn main() {
    print(prozentsatz(15, 60)) # 25
}
```

---

### Kapitel 13: Sammlungen, Listen und Wörterbücher (Arrays & Maps)

- **Stufe 1 (Leicht – Text-Array iterieren):**
```zelyra
fn main() {
    themen: String[] = ["Compiler", "Datenbanken", "Web-Sicherheit"]
    for t in themen {
        print("- " + t)
    }
}
```

- **Stufe 2 (Mittel – Preise nachschlagen mit Map und Option):**
```zelyra
fn main() {
    mutable preise: Map<String, Int> = map()
    preise = set(preise, "Espresso", 2)
    preise = set(preise, "Cappuccino", 4)

    match get(preise, "Espresso") {
        Some(preis) => print("Espresso kostet " + preis + " Euro"),
        None => print("Artikel nicht gefunden"),
    }

    match get(preise, "Kuchen") {
        Some(preis) => print("Kuchen kostet " + preis + " Euro"),
        None => print("Kuchen ist leider ausverkauft"),
    }
}
```

- **Stufe 3 (Anspruchsvoll – Wörter zählen mit Map):**
```zelyra
fn zaehle_woerter(liste: String[]) -> Map<String, Int> {
    mutable zaehler: Map<String, Int> = map()
    for wort in liste {
        aktuell = match get(zaehler, wort) {
            Some(anzahl) => anzahl + 1,
            None => 1,
        }
        zaehler = set(zaehler, wort, aktuell)
    }
    return zaehler
}

fn main() {
    begriffe: String[] = ["zelyra", "rust", "zelyra", "web"]
    ergebnis = zaehle_woerter(begriffe)
    print("Erledigt.")
}
```

---

### Kapitel 14: Eigene Datentypen erstellen (Records & Tables)

- **Stufe 1 (Leicht – Nominaler Typalias):**
```zelyra
type UserId = Id

fn main() {
    uid: UserId = 1
    print("Benutzer-ID angelegt: " + uid)
}
```

- **Stufe 2 (Mittel – Kategorie-Tabelle):**
```zelyra
table kategorien {
    id: Id primary auto
    kategorie_name: String required
    beschreibung: String
}
```

- **Stufe 3 (Anspruchsvoll – Relationale Notizen-Tabelle):**
```zelyra
table notizen {
    id: Id primary auto
    aufgabe_id: Id required
    notiz_text: String required
    erstellt_am: Timestamp auto
}
```

---

### Kapitel 15: Module und Code-Organisation

- **Stufe 1 (Leicht – Projekt-Inspektion):**
```bash
zelyra new task_manager
cd task_manager
ls -la
```
*Erklärung:* Das Projekt enthält `zelyra.toml` (Konfiguration), `main.zyl` (Einstiegspunkt) und Verzeichnisse für modulare Quelldateien.

- **Stufe 2 (Mittel – Projekt-Manifest erweitern):**
```toml
[project]
name = "task_manager"
version = "0.3.0"
description = "Zuverlässige Aufgabenverwaltung mit Zelyra"
authors = ["Stefan Siedelmann"]
```

- **Stufe 3 (Anspruchsvoll – Modulare Trennung in Funktionen):**
```zelyra
fn initialisiere_app() -> String {
    return "TaskManager bereit"
}

fn verarbeite_aufgabe(titel: String) -> String {
    return "Verarbeite: " + titel
}

fn zeige_status(nachricht: String) {
    print("[STATUS] " + nachricht)
}

fn main() {
    zeige_status(initialisiere_app())
    zeige_status(verarbeite_aufgabe("Release vorbereiten"))
}
```

---

### Kapitel 16: Fehlerarten und ihre Ursachen

- **Stufe 1 (Leicht – Syntaxfehler diagnostizieren):**
Entfernst du in einer Datei die schließende Klammer `}`, bricht `zelyra check` mit einem Fehler über unbalancierte Blöcke ab. Der Compiler nennt die exakte Zeile, an der der Block geöffnet wurde.

- **Stufe 2 (Mittel – Typfehler beheben):**
```zelyra
fn main() {
    # Falsch: anzahl: Int = "zehn"  -> E-TYPE-001
    # Korrekt:
    anzahl: Int = 10
    print(anzahl)
}
```

- **Stufe 3 (Anspruchsvoll – Altersprüfung mit Verträgen):**
```zelyra
fn pruefe_kinofreigabe(alter: Int) -> Bool
    requires { alter >= 0 && alter <= 130 }
{
    return alter >= 16
}

fn main() {
    print(pruefe_kinofreigabe(17)) # true
}
```

---

### Kapitel 17: Fehler als Werte – Das Result-Muster

- **Stufe 1 (Leicht – Gerade-Zahl-Result):**
```zelyra
fn pruefe_gerade(zahl: Int) -> Result<Int, String> {
    if zahl % 2 == 0 {
        return Ok(zahl)
    }
    return Err("Die Zahl ist ungerade!")
}

fn main() {
    match pruefe_gerade(10) {
        Ok(z) => print("Erfolg: " + z + " ist gerade"),
        Err(msg) => print("Fehler: " + msg),
    }
}
```

- **Stufe 2 (Mittel – Benutzernamen validieren):**
```zelyra
fn validiere_benutzername(name: String) -> Result<String, String> {
    if name == "" {
        return Err("Name darf nicht leer sein.")
    }
    if name == "admin" || name == "root" {
        return Err("Name ist reserviert.")
    }
    return Ok(name)
}

fn main() {
    match validiere_benutzername("stefan") {
        Ok(n) => print("Willkommen, " + n),
        Err(e) => print("Ungueltiger Benutzername: " + e),
    }
}
```

- **Stufe 3 (Anspruchsvoll – Division mit Result und Verdopplung):**
```zelyra
fn sichere_division(zaehler: Int, nenner: Int) -> Result<Int, String> {
    if nenner == 0 {
        return Err("Division durch null ist verboten!")
    }
    return Ok(zaehler / nenner)
}

fn main() {
    match sichere_division(100, 5) {
        Ok(quotient) => {
            verdoppelt = quotient * 2
            print("Ergebnis verdoppelt: " + verdoppelt)
        },
        Err(e) => print("Berechnungsfehler: " + e),
    }
}
```

---

### Kapitel 18: Das Nichts existiert nicht – Der sichere Umgang mit Option

- **Stufe 1 (Leicht – Partner-Suchfunktion):**
```zelyra
fn finde_partner(name: String) -> Option<String> {
    if name == "Romeo" {
        return Some("Julia")
    }
    if name == "Bonnie" {
        return Some("Clyde")
    }
    return None
}

fn main() {
    match finde_partner("Romeo") {
        Some(partner) => print("Gefunden: " + partner),
        None => print("Kein Partner hinterlegt"),
    }
}
```

- **Stufe 2 (Mittel – Letztes Element auswerten):**
```zelyra
fn main() {
    zahlen: Int[] = [10, 20, 30]
    match last(zahlen) {
        Some(wert) => print("Letztes Element: " + wert),
        None => print("Achtung: Liste war leer!"),
    }
}
```

- **Stufe 3 (Anspruchsvoll – Position in einer Liste suchen):**
```zelyra
fn finde_index(liste: Int[], ziel: Int) -> Option<Int> {
    mutable idx = 0
    for element in liste {
        if element == ziel {
            return Some(idx)
        }
        idx = idx + 1
    }
    return None
}

fn main() {
    daten: Int[] = [100, 200, 300, 400]
    match finde_index(daten, 300) {
        Some(pos) => print("Gefunden an Index: " + pos),
        None => print("Element nicht in Liste"),
    }
}
```

---

### Kapitel 19: Tests und Qualitätssicherung

- **Stufe 1 (Leicht – Addition testen):**
```zelyra
fn addiere(a: Int, b: Int) -> Int {
    return a + b
}

fn main() {
    assert(addiere(2, 3) == 5)
    assert(addiere(-2, 2) == 0)
    assert(addiere(0, 0) == 0)
    print("Alle Tests fuer addiere() bestanden!")
}
```

- **Stufe 2 (Mittel – Option-Tests):**
```zelyra
fn finde_index(liste: Int[], ziel: Int) -> Option<Int> {
    mutable idx = 0
    for element in liste {
        if element == ziel {
            return Some(idx)
        }
        idx = idx + 1
    }
    return None
}

fn main() {
    test_liste: Int[] = [10, 20, 30]
    match finde_index(test_liste, 20) {
        Some(i) => assert(i == 1),
        None => assert(false),
    }
    match finde_index(test_liste, 99) {
        Some(_) => assert(false),
        None => assert(true),
    }
    print("Suchtests erfolgreich!")
}
```

- **Stufe 3 (Anspruchsvoll – Validierungs-Testsuite):**
```zelyra
fn validiere_titel(t: String) -> Bool {
    return len(t) >= 3 && len(t) <= 50
}

fn main() {
    assert(validiere_titel("OK") == false)
    assert(validiere_titel("Gueltiger Titel") == true)
    assert(validiere_titel("Ein viel zu langer Titel, der mehr als fuenfzig Zeichen umfasst und daher abgewiesen werden muss") == false)
    print("Validierungs-Suite erfolgreich ausgefuehrt.")
}
```

---

### Kapitel 20: Arbeiten mit Dateien

- **Stufe 1 (Leicht – Datei schreiben):**
```zelyra
fn main() uses FileSystem {
    ergebnis = write_text("hallo.txt", "Hallo aus der sicheren Zelyra-Welt!")
    match ergebnis {
        Ok(_) => print("Datei erfolgreich geschrieben."),
        Err(e) => print("Schreibfehler: " + e),
    }
}
```

- **Stufe 2 (Mittel – Datei lesen):**
```zelyra
fn main() uses FileSystem {
    match read_text("hallo.txt") {
        Ok(inhalt) => print("Gelesener Inhalt: " + inhalt),
        Err(e) => print("Konnte Datei nicht lesen: " + e),
    }
}
```

- **Stufe 3 (Anspruchsvoll – Zeilenweises Logging):**
```zelyra
fn logge_ereignis(pfad: String, meldung: String) uses FileSystem {
    bisher = match read_text(pfad) {
        Ok(alt) => alt + "
",
        Err(_) => "",
    }
    neu = bisher + "[LOG] " + meldung
    write_text(pfad, neu)
}

fn main() uses FileSystem {
    logge_ereignis("audit.log", "System gestartet.")
    logge_ereignis("audit.log", "Benutzer angemeldet.")
    print("Log aktualisiert.")
}
```

---

### Kapitel 21: Datum, Uhrzeit, Zufall und strukturierte Daten

- **Stufe 1 (Leicht – Würfeln):**
```zelyra
fn wuerfeln() -> Int uses Random {
    return random_int(1, 6)
}

fn main() uses Random {
    augen = wuerfeln()
    print("Gewuerfelt: " + augen)
}
```

- **Stufe 2 (Mittel – JSON-Codierung):**
```zelyra
fn main() {
    codes: Int[] = [200, 201, 404, 500]
    json_text = json_encode(codes)
    print("JSON: " + json_text)
}
```

- **Stufe 3 (Anspruchsvoll – Zeitstempel-Logging):**
```zelyra
fn main() uses Clock, FileSystem {
    aktueller_zeitpunkt = now()
    logzeile = "Ereignis eingetreten um: " + aktueller_zeitpunkt
    write_text("zeitstempel.log", logzeile)
    print("Zeitstempel geloggt.")
}
```

---

### Kapitel 22: Nebenläufigkeit und Hintergrundaufgaben

- **Stufe 1 (Leicht – Parallele Ausführung):**
```zelyra
fn berechne_teil_a() -> Int { return 20 + 1 }
fn berechne_teil_b() -> Int { return 20 + 1 }

fn main() {
    parallel {
        task_a = berechne_teil_a()
        task_b = berechne_teil_b()
    }
    summe = await(task_a) + await(task_b)
    print("Paralleles Gesamtergebnis: " + summe) # 42
}
```

- **Stufe 2 (Mittel – Zwei Datenquellen parallel abfragen):**
```zelyra
fn lade_kunden_anzahl() -> Int { return 150 }
fn lade_offene_rechnungen() -> Int { return 12 }

fn main() {
    parallel {
        ka = lade_kunden_anzahl()
        re = lade_offene_rechnungen()
    }
    print("Kunden: " + await(ka) + ", Offene Rechnungen: " + await(re))
}
```

- **Stufe 3 (Anspruchsvoll – Robuste Fehlerbehandlung bei Tasks):**
In parallelen Blöcken liefert jeder Task ein typisiertes Ergebnis oder ein `Result`. Schlägt einer der Zweige fehl, fängt das Hauptprogramm den Fehler über `match await(task)` sauber ab, ohne dass die gesamte Anwendung abstürzt.

---

### Kapitel 23: Warum Zelyra die Datenbank direkt versteht

- **Stufe 1 (Leicht – Datenbank-Deklaration):**
```zelyra
database MariaDB {
    url: env("DATABASE_URL")
}
```

- **Stufe 2 (Mittel – Benutzer-Tabelle und SQL-Abfrage):**
```zelyra
table benutzer {
    id: Id primary auto
    email: String required
    aktiv: Bool required
}

fn lade_aktive_benutzer() -> benutzer[] uses Database {
    return sql<benutzer[]> {
        SELECT * FROM benutzer WHERE aktiv = true
    }
}
```

- **Stufe 3 (Anspruchsvoll – Spalten-Prüfung zur Compile-Zeit):**
Wenn du in der SQL-Abfrage `SELECT ungueltige_spalte FROM benutzer` schreibst, verweigert `zelyra check` sofort die Übersetzung mit dem Fehler `E-DB-002: Unknown column 'ungueltige_spalte' on table 'benutzer'`. Zelyra schützt dich vor Runtime-Datenbankfehlern!

---

### Kapitel 24: Tabellen definieren und Daten modellieren

- **Stufe 1 (Leicht – Etiketten-Tabelle):**
```zelyra
table etiketten {
    id: Id primary auto
    name: String(30) required
}
```

- **Stufe 2 (Mittel – Kunden-Tabelle):**
```zelyra
table kunden {
    id: Id primary auto
    vorname: String(50) required
    nachname: String(50) required
    email: Email required
    telefon: String(30)
}
```

- **Stufe 3 (Anspruchsvoll – Relationale Kommentare):**
```zelyra
table kommentare {
    id: Id primary auto
    aufgabe_id: Id required fk tasks(id) on_delete restrict
    kommentar: String required
    erstellt_am: Timestamp auto
}
```

---

### Kapitel 25: Daten abfragen und verändern

- **Stufe 1 (Leicht – Offene Aufgaben abfragen):**
```zelyra
fn lade_offene_aufgaben() -> tasks[] uses Database {
    return sql<tasks[]> {
        SELECT * FROM tasks WHERE erledigt = false ORDER BY id ASC
    }
}
```

- **Stufe 2 (Mittel – Aufgabe löschen):**
```zelyra
fn loesche_aufgabe(aufgabe_id: Id) uses Database {
    sql {
        DELETE FROM tasks WHERE id = :aufgabe_id
    }
}
```

- **Stufe 3 (Anspruchsvoll – Transaktion mit Archivierung):**
```zelyra
fn archiviere_und_neue_aufgabe(alte_id: Id, neuer_titel: String) uses Database {
    transaction {
        sql {
            UPDATE tasks SET archiviert = true WHERE id = :alte_id
        }
        sql {
            INSERT INTO tasks (titel, erledigt, archiviert)
            VALUES (:neuer_titel, false, false)
        }
    }
}
```

---

### Kapitel 26: Webseiten ausgeben

- **Stufe 1 (Leicht – Über-Uns-Seite):**
```zelyra
page "/ueber-uns" {
    html {
        <main>
            <h1>Ueber unser Unternehmen</h1>
            <p>Wir bauen langlebige Software mit Zelyra.</p>
        </main>
    }
}
```

- **Stufe 2 (Mittel – Dynamische Detailseite):**
```zelyra
page "/produkt/{nummer: Int}" {
    html {
        <div>
            <h2>Produkt-Detailansicht</h2>
            <p>Angezeigte Artikelnummer: {nummer}</p>
        </div>
    }
}
```

- **Stufe 3 (Anspruchsvoll – Übersichtsseite mit Navigation):**
```zelyra
page "/uebersicht" {
    html {
        <nav>
            <a href="/">Startseite</a> | <a href="/ueber-uns">Ueber uns</a>
        </nav>
        <section>
            <h2>Unsere Kernbereiche</h2>
            <ul>
                <li>Sichere Web-Entwicklung</li>
                <li>Transaktionale Datenbanken</li>
                <li>KI-gestuetzte Codeanalyse</li>
            </ul>
        </section>
    }
}
```

---

### Kapitel 27: Formulare und Benutzereingaben

- **Stufe 1 (Leicht – Kategorie-Formular):**
```zelyra
form KategorieForm {
    name: String required
    beschreibung: String
}
```

- **Stufe 2 (Mittel – Formularvalidierung auf CLI):**
```bash
zelyra form validate main.zyl
```
*Erklärung:* Zelyra prüft die Formularfelder gegen das zugrundeliegende Tabellenschema.

- **Stufe 3 (Anspruchsvoll – Prioritäts-Validierung im Formular):**
```zelyra
form AufgabeForm {
    titel: String required min_length(3)
    prioritaet: Int required min(1) max(3)
}
```

---

### Kapitel 28: Das vollständige CRUD-Muster

- **Stufe 1 (Leicht – CRUD deklarieren):**
```zelyra
crud Kategorie -> kategorien {
    title: "Kategorien verwalten"
    fields: [kategorie_name, beschreibung]
}
```

- **Stufe 2 (Mittel – Routen prüfen):**
```bash
zelyra routes main.zyl
```
*Erklärung:* Die Ausgabe zeigt alle automatisch generierten Routen wie `GET /kategorien`, `GET /kategorien/new`, `POST /kategorien/new`, `GET /kategorien/{id}` und `POST /kategorien/{id}/delete`.

- **Stufe 3 (Anspruchsvoll – Benutzerdefinierte Duplizier-Aktion):**
```zelyra
action "duplizieren" on tasks (id: Id) uses Database {
    sql {
        INSERT INTO tasks (titel, erledigt)
        SELECT CONCAT(titel, ' (Kopie)'), false FROM tasks WHERE id = :id
    }
}
```

---

### Kapitel 29: Benutzer, Passwörter und Sitzungen

- **Stufe 1 (Leicht – Passwort hashen):**
```bash
zelyra auth hash-password "MeinSuperSicheresPasswort2026!"
```

- **Stufe 2 (Mittel – Route mit Authentifizierung schützen):**
```zelyra
page "/einstellungen" requires auth {
    html {
        <h1>Persoenliche Benutzereinstellungen</h1>
    }
}
```

- **Stufe 3 (Anspruchsvoll – Rollenbasierte Rechtevergabe):**
```zelyra
page "/admin/system" requires auth permits admin {
    html {
        <h1>System-Administration (Nur fuer Admins)</h1>
    }
}
```

---

### Kapitel 30: APIs und Datenaustausch

- **Stufe 1 (Leicht – Versions-Endpunkt):**
```zelyra
page "/api/version" {
    json {
        "system": "Zelyra TaskEngine",
        "version": "1.0.5",
        "status": "operational"
    }
}
```

- **Stufe 2 (Mittel – REST-Delete mit Fehlerbehandlung):**
```zelyra
page "DELETE /api/tasks/{id: Id}" uses Database {
    betroffene = sql {
        DELETE FROM tasks WHERE id = :id
    }
    if betroffene == 0 {
        return http_response(404, "Aufgabe nicht gefunden")
    }
    return http_response(200, "Aufgabe geloescht")
}
```

- **Stufe 3 (Anspruchsvoll – OpenAPI-Spezifikation generieren):**
```bash
zelyra doc --openapi main.zyl > openapi.json
```
*Erklärung:* Erzeugt eine standardkonforme OpenAPI-3.0-Spezifikation aus deinen deklarierten Routen, Tabellen und Typen.

---

### Kapitel 31: Lesbarkeit als oberstes Gebot

- **Stufe 1 (Leicht – Automatisch formatieren):**
```bash
zelyra fmt src/main.zyl
```

- **Stufe 2 (Mittel – Refactoring sprechender Namen):**
```zelyra
# Schlecht: fn f(a: Int, b: Float) -> Float { return a * b * 0.19 }
# Vorbildlich:
fn berechne_mehrwertsteuer(netto_betrag: Decimal, steuersatz: Decimal) -> Decimal {
    return netto_betrag * steuersatz
}
```

- **Stufe 3 (Anspruchsvoll – Klare Funktionsgliederung):**
Zerlege lange Berechnungs-Monster in kleine Funktionen mit jeweils genau einer klaren Aufgabe (Single Responsibility Principle). Jede Funktion sollte ohne horizontales Scrollen auf einen Blick verständlich sein.

---

### Kapitel 32: KI-Nativität – Warum Zelyra perfekt für KI-Assistenten ist

- **Stufe 1 (Leicht – JSON-Diagnosen):**
```bash
zelyra check --format json main.zyl
```

- **Stufe 2 (Mittel – Auswirkungsanalyse mit impact):**
```bash
zelyra impact main.zyl
```
*Erklärung:* Analysiert Abhängigkeiten und zeigt, welche Funktionen oder Tabellen von einer Änderung betroffen sind.

- **Stufe 3 (Anspruchsvoll – Typed Holes):**
```zelyra
fn berechne_rabatt(preis: Decimal) -> Decimal {
    return _
}
```
Führe `zelyra check` aus. Der Compiler antwortet mit einem präzisen Loch-Report: Er nennt dir den erwarteten Rückgabetyp `Decimal` und alle im Gültigkeitsbereich verfügbaren Bezeichner – die perfekte Vorlage für einen KI-Assistenten!

---

### Kapitel 33: Sicherheit durch Fähigkeiten (Capabilities)

- **Stufe 1 (Leicht – Capabilities identifizieren):**
Funktionen ohne `uses ...` sind garantiert reine Funktionen (pure) ohne Netzwerk-, Dateisystem- oder Datenbank-Seiteneffekte.

- **Stufe 2 (Mittel – Mehrere Capabilities kombinieren):**
```zelyra
fn speichere_backup_protokoll(meldung: String) uses Clock, Environment, FileSystem {
    zeit = now()
    pfad = match env("BACKUP_LOG_PATH") {
        Some(p) => p,
        None => "backup.log",
    }
    write_text(pfad, "[" + zeit + "] " + meldung)
}
```

- **Stufe 3 (Anspruchsvoll – Trennung von Logik und I/O):**
Halte alle Berechnungen (z. B. Rechnungsbeträge, Rabatte, Datumsabstände) in reinen Funktionen völlig ohne Capabilities. Deklariere Capabilities ausschließlich an den Rändern des Systems (beim Laden aus der DB oder Schreiben ins Dateisystem).

---

### Kapitel 34: Zelyra im Vergleich

- **Stufe 1 (Leicht – Von Python zu Zelyra):**
Python:
```python
def verdopple(x): return x * 2
```
Zelyra:
```zelyra
fn verdopple(x: Int) -> Int {
    return x * 2
}
```

- **Stufe 2 (Mittel – Eloquent ORM vs. Zelyra SQL):**
In Laravel/Eloquent führt `$orders->each(fn($o) => $o->customer->name)` oft unbemerkt zu 100 separaten Datenbank-Queries (N+1 Problem). In Zelyra schreibst du eine typgeprüfte `sql<T[]>`-Abfrage mit explizitem `JOIN`, die in einem einzigen effizienten Datenbank-Roundtrip ausgeführt wird.

- **Stufe 3 (Anspruchsvoll – Supply-Chain-Sicherheit):**
Wenn ein externes Paket kompromittiert wird, kann es in Node.js oder Python ungehindert `.env`-Dateien lesen oder Passwörter ins Internet senden. In Zelyra kann ein Modul ohne `uses Network` oder `uses FileSystem` technisch unmöglich Daten nach außen senden – der Compiler verweigert den Zugriff.

---

### Kapitel 35: Software planen – Von der Idee zum Entwurf

- **Stufe 1 (Leicht – Domänen-Skizze Notiz-App):**
- Entitäten: `Benutzer`, `Notiz`, `Ordner`.
- Aktionen: Anlegen, Durchsuchen, Aktualisieren, Löschen.

- **Stufe 2 (Mittel – Tabellenschema entwerfen):**
```zelyra
table notizen_app {
    id: Id primary auto
    benutzer_id: Id required
    titel: String(100) required
    inhalt: String required
    erstellt_am: Timestamp auto
}
```

- **Stufe 3 (Anspruchsvoll – Verträge für Kernfunktionen):**
```zelyra
fn erstelle_notiz(titel: String, text: String) -> Bool
    requires { len(titel) >= 1 && len(titel) <= 100 }
    requires { len(text) > 0 }
    ensures { result == true }
{
    return true
}
```

---

### Kapitel 36: Architektur und saubere Codestruktur

- **Stufe 1 (Leicht – Die drei Schichten):**
1. Präsentation: `page`, `html`, `form`
2. Geschäftslogik: Reine Funktionen (`fn berechne...`) mit Verträgen
3. Persistenz: `table`, `sql`, `transaction`

- **Stufe 2 (Mittel – Entkopplung von Route und Logik):**
Schreibe keine Berechnungen direkt in den `html`-Block der Route, sondern delegiere sie an eine typisierte Hilfsfunktion `berechne_brutto(netto, steuer)`.

- **Stufe 3 (Anspruchsvoll – Schichtenmodell für Zeiterfassung):**
- UI: `/timetracking/entry`
- Service: `berechne_dauer(start: Time, ende: Time) -> Duration` (pure!)
- Storage: `speichere_zeiteintrag(eintrag: Zeiteintrag) uses Database`

---

### Kapitel 37: Konfiguration und Umgebungsvariablen

- **Stufe 1 (Leicht – Benutzername auslesen):**
```zelyra
fn main() uses Environment {
    match env("USER_NAME") {
        Some(name) => print("Hallo, " + name + "!"),
        None => print("Hallo, lieber Gast!"),
    }
}
```

- **Stufe 2 (Mittel – Hilfsfunktion mit Fallback):**
```zelyra
fn hole_env_oder_standard(schluessel: String, standard: String) -> String uses Environment {
    return match env(schluessel) {
        Some(wert) => wert,
        None => standard,
    }
}

fn main() uses Environment {
    port = hole_env_oder_standard("APP_PORT", "8080")
    print("Server lauscht auf Port: " + port)
}
```

- **Stufe 3 (Anspruchsvoll – Dev- vs. Prod-Umschaltung):**
```zelyra
fn ist_produktionsmodus() -> Bool uses Environment {
    return match env("APP_ENV") {
        Some(modus) => modus == "production",
        None => false,
    }
}

fn main() uses Environment {
    if ist_produktionsmodus() {
        print("Produktionsmodus aktiv: Strenger Sicherheitsmodus.")
    } else {
        print("Entwicklungsmodus aktiv: Ausfuehrliche Fehlermeldungen.")
    }
}
```

---

### Kapitel 38: Fehlersuche und Optimierung

- **Stufe 1 (Leicht – Umgebungs-Check):**
```bash
zelyra doctor
```

- **Stufe 2 (Mittel – Fehlerisolation in Testfunktionen):**
Wenn eine Berechnung ein unerwartetes Ergebnis liefert, isoliere die Formel in einer separaten kleinen Datei `test_rechnung.zyl` und teste sie isoliert mit festen Werten und `assert()`, ohne Datenbank und Webserver starten zu müssen.

- **Stufe 3 (Anspruchsvoll – Schrittweises Audit-Logging):**
Protokolliere wichtige Zwischenzustände vor und nach kritischen Operationen strukturiert mit Zeitstempel und Statuscodes in eine separate Audit-Tabelle.

---

### Kapitel 39: Bereitstellung und Betrieb

- **Stufe 1 (Leicht – Webserver starten):**
```bash
zelyra serve main.zyl 127.0.0.1:3000
```

- **Stufe 2 (Mittel – Compose-Konfiguration):**
```yaml
services:
  mariadb:
    image: mariadb:11.4
    environment:
      MARIADB_ROOT_PASSWORD: secret
      MARIADB_DATABASE: app_db
    ports:
      - "3306:3306"
```

- **Stufe 3 (Anspruchsvoll – Schema-Migration ohne Datenverlust):**
```bash
zelyra db plan main.zyl
# Prüfe die SQL-Änderungen sorgfältig!
zelyra db apply main.zyl
```

---

### Kapitel 40: Das große Abschlussprojekt: Vollständige Aufgabenverwaltung

- **Stufe 1 (Leicht – Projekt-Verifikation):**
```bash
zelyra check main.zyl
```
Der Compiler bestätigt 0 Fehler und 0 Warnungen.

- **Stufe 2 (Mittel – Fälligkeitsdatum ergänzen):**
In der Tabelle `tasks`:
```zelyra
table tasks {
    id: Id primary auto
    titel: String(100) required
    erledigt: Bool required
    faellig_am: Date
}
```
Passe das Formular und die HTML-Ansicht an, um das Datumsfeld darzustellen.

- **Stufe 3 (Anspruchsvoll – Lokales Live-Deployment):**
Starte MariaDB mit Compose, wende das Schema mit `zelyra db apply main.zyl` an, starte den Webserver mit `zelyra serve` und erstelle deine ersten echten Aufgaben im Browser!

---

### Kapitel 41: Funktionsumfang und sicherer Betrieb

- **Stufe 1 (Leicht – Versions- und Umgebungsdiagnose):**
```bash
zelyra --version
zelyra doctor
```
*Erklärung:* Bestätigt, dass Version 1.0.5 aktiv ist, die Sprachkompatibilität auf `0.1` steht und das Docker Compose v2 Plugin bereitsteht.

- **Stufe 2 (Mittel – Tutorial-Trockenprüfung):**
```bash
zelyra --tutorial invoice --no-start
cd invoice-tutorial
zelyra db plan main.zyl
```
*Erklärung:* `db plan` analysiert das deklarierte Rechnungsschema (Kunden, Artikel, Rechnungen, Positionen) und gibt die geplanten SQL-Statements aus, ohne schreibend in die Datenbank einzugreifen.

- **Stufe 3 (Anspruchsvoll – Risikofreies Migrations- und Backup-Verfahren):**
1. Test-Datenbank über einen alternativen Port starten (z. B. 3307 in `.env`).
2. `zelyra db plan main.zyl` prüfen.
3. Schema mit `zelyra db apply main.zyl` auf der Testdatenbank anwenden.
4. Backup-Erstellung mit `zelyra db backup` testen.
5. Erst nach erfolgreicher Test-Verifikation wird das Update für das Produktivsystem freigegeben.

---

### Kapitel 42: Dein Weg als Zelyra-Entwickler

- **Stufe 1 (Leicht – Model-First-Entwurf):**
Für ein persönliches Haushaltsbuch entwirfst du zuerst das relationale Tabellenschema:
```zelyra
table buchungen {
    id: Id primary auto
    datum: Date required
    kategorie: String(50) required
    betrag: Decimal required
    beschreibung: String
}
```

- **Stufe 2 (Mittel – Vertragliche Absicherung):**
```zelyra
fn buchung_gueltig(betrag: Decimal) -> Bool
    ensures { result == (betrag != decimal("0.000000000")) }
{
    return betrag != decimal("0.000000000")
}
```
*Erklärung:* Buchungen mit einem Betrag von 0,00 € werden durch die Nachbedingung logisch ausgeschlossen.

- **Stufe 3 (Anspruchsvoll – Klare Modultrennung):**
Strukturiere das Projekt in:
- `src/model.zyl`: Tabellen und Record-Definitionen.
- `src/finance.zyl`: Reine Berechnungsfunktionen (Monatssummen, Steuerquoten) ohne Capabilities.
- `src/storage.zyl`: Persistenzfunktionen mit `uses Database`.
- `src/web.zyl`: HTML-Seiten und CRUD-Definitionen.

---

### Kapitel 43: Mit Zelyra Studio arbeiten

- **Stufe 1 (Leicht – Studio starten):**
```bash
cd mein_projekt
zelyra editor .
```
*Erklärung:* Studio startet einen lokalen Webserver auf `127.0.0.1` mit sicherem Einmal-Token und öffnet den Editor im Standardbrowser.

- **Stufe 2 (Mittel – Schemakarte inspizieren):**
Klicke in der Seitenleiste von Studio auf das Datenbanksymbol (**Datenbank**). Die Schemakarte liest die Tabellen- und Fremdschlüssel-Metadaten schreibgeschützt ein und visualisiert die Beziehungen, ohne Datensätze zu manipulieren.

- **Stufe 3 (Anspruchsvoll – CRUD-Vorschau und Routenprüfung):**
1. Wähle im Studio eine deklarierte Tabelle ohne bestehende CRUD-Ansicht aus.
2. Starte den CRUD-Assistenten und wähle die gewünschten Formularfelder.
3. Betrachte die vorgeschlagene Quelltext-Differenz (Diff) in der Vorschau.
4. Vergleiche die generierten Endpunkte mit `zelyra routes main.zyl` im Terminal.
5. Übernimm die Änderung erst nach Prüfung im Editor.
---

## Anhang I: Häufige Fragen und Antworten (FAQ)

**Frage: Wie verwende ich Imports?**
*Antwort:* Mit `import "src/models.zyl" as models` bindest du eine projektlokale Quelldatei ein. Unterstützte öffentliche Deklarationen werden über den Modulnamen angesprochen. Eine Paketregistrierung, dynamische Imports und ein vollständiger Deployment-Abschluss sind nicht enthalten.

**Frage: Kann ich mit Zelyra auch reine Konsolenprogramme schreiben?**
*Antwort:* Ja. `print()` gibt Werte aus. `read_console("Prompt: ")` liest eine Zeile und liefert `String?`; dafür braucht die Funktion `uses Console` und das Projekt gegebenenfalls `console = true`.

**Frage: Warum unterstützt Zelyra MariaDB als bevorzugte Engine?**
*Antwort:* MariaDB bietet herausragende Performance, Open-Source-Freiheit, Stabilität und breite Cloud-Unterstützung für professionelle Webanwendungen.

---

## Anhang J: Weiterführende Ressourcen und Community

- **Offizielles GitHub-Repository:** [https://github.com/sf1976/zelyra](https://github.com/sf1976/zelyra)
- **Dokumentation & Online-Handbuch:** [https://siedelmann.com/handbuch](https://siedelmann.com/handbuch) / [https://siedelmann.com/handbook](https://siedelmann.com/handbook)
- **Beispiele & Vorlagen:** Im Verzeichnis `examples/` des Repositories findest du lauffähige Vorlagen für Authentifizierung, CRUD-Ansichten, APIs und Datenbanken.

---

## Anhang K: Das Zelyra-Cookbook (Praxisrezepte)

Das Zelyra-Cookbook bietet eine kuratierte Sammlung erprobter Entwurfsmuster und praktischer Lösungen für reale Anforderungen in Unternehmens- und Webanwendungen. Jedes Rezept folgt dem bewährten Dreischritt: **Problem**, **Lösung** (vertraglich abgesicherter Zelyra-Code) und **Erklärung**.

---

### Teil 1: Datenverarbeitung & Standardbibliothek

#### Rezept K.1: Exakte kaufmännische Rundung und Skontoberechnung

**Problem:**
In kaufmännischen Anwendungen führen Fließkommazahlen (`Float`) zu Rundungsfehlern bei Rechnungsrabatten, Skontoabzügen und der Mehrwertsteueraufteilung.

**Lösung:**
```zelyra
pub record DiscountResult {
    base_net: Decimal,
    discount_rate: Decimal,
    discount_amount: Decimal,
    final_net: Decimal,
}

pub fn apply_cash_discount(amount: Decimal, rate: Decimal) -> DiscountResult
    requires { amount >= decimal("0.00") && rate >= decimal("0.00") && rate <= decimal("100.00") }
    ensures { result.final_net <= result.base_net && result.discount_amount >= decimal("0.00") }
{
    let discount = (amount * rate) / decimal("100.00")
    let final_amount = amount - discount
    return DiscountResult {
        base_net: amount,
        discount_rate: rate,
        discount_amount: discount,
        final_net: final_amount,
    }
}
```

**Erklärung:**
- Zelyra verwendet für `Decimal` Festkomma-Arithmetik ohne binäre Gleitkomma-Ungenauigkeiten.
- Der Vorbedingungsvertrag `requires` garantiert, dass kein negativer Skontosatz oder ein Satz über 100 % übergeben werden kann.
- Die Nachbedingung `ensures` beweist mathematisch, dass der Endbetrag niemals den ursprünglichen Nettobetrag übersteigen kann.

---

#### Rezept K.2: Webhook-Payloads sicher parsen und typisieren

**Problem:**
Ein externer Zahlungsdienstleister (z. B. Stripe oder PayPal) sendet JSON-Payloads via HTTP-POST an einen Webhook-Endpunkt. Unvollständige oder manipulierte Felder müssen sicher abgewehrt werden.

**Lösung:**
```zelyra
pub enum PaymentStatus {
    Paid,
    Pending,
    Failed,
}

pub record WebhookPayload {
    event_id: String,
    payment_reference: String,
    amount: Decimal,
    status: PaymentStatus,
}

pub fn parse_payment_status(raw: String) -> Option<PaymentStatus> {
    match raw {
        "succeeded" => { return Option::Some(PaymentStatus::Paid) }
        "processing" => { return Option::Some(PaymentStatus::Pending) }
        "requires_payment_method" => { return Option::Some(PaymentStatus::Failed) }
        _ => { return Option::None }
    }
}
```

**Erklärung:**
- Zelyra erzwingt exhaustive Pattern-Matchings (`match`). Unbekannte Ereignistypen werden über die Wildcard `_` abgefangen und als `Option::None` gemeldet, statt unerwartete Null-Pointer-Exceptions zur Laufzeit zu werfen.

---

#### Rezept K.3: Fristen und Fälligkeitstage berechnen

**Problem:**
Eine Rechnung erhält ein Zahlungsziel von 14 Tagen. Fällt der Fälligkeitstag auf ein Wochenende, soll die Frist automatisch auf den nächsten Werktag (Montag) verlängert werden.

**Lösung:**
```zelyra
pub fn calculate_due_date(invoice_date: Date, net_days: Int) -> Date
    requires { net_days >= 0 }
{
    let target = invoice_date.add_days(net_days)
    let weekday = target.day_of_week()
    if weekday == 6 {
        return target.add_days(2)
    } else if weekday == 7 {
        return target.add_days(1)
    } else {
        return target
    }
}
```

**Erklärung:**
- Datumsoperationen in Zelyra sind zeitzonensicher. `add_days` arbeitet direkt auf dem typisierten `Date`-Kalendermodell.

---

### Teil 2: Datenbank, Transaktionen & Mandanten

#### Rezept K.4: Lückenlose Rechnungsnummernkreise (Atomic Sequence)

**Problem:**
Das Finanzamt und die GoBD fordern für Rechnungen eine lückenlose, fortlaufende Nummerierung. Unter parallelen Anfragen können Race-Conditions zu doppelten oder fehlenden Nummern führen.

**Lösung:**
```zelyra
table invoice_sequences scoped by tenant_id: TenantId {
    id: Id primary auto
    tenant_id: TenantId required
    fiscal_year: Int required
    current_value: Int required default 0

    unique within tenant fiscal_year
}

fn next_invoice_number(t_id: TenantId, year: Int) -> String uses Database {
    transaction {
        let seq = query_one(
            "SELECT id, current_value FROM invoice_sequences WHERE tenant_id = ? AND fiscal_year = ? FOR UPDATE",
            [t_id, year]
        )
        let next_val = seq.current_value + 1
        execute(
            "UPDATE invoice_sequences SET current_value = ? WHERE id = ?",
            [next_val, seq.id]
        )
        return "RE-" + year.to_string() + "-" + next_val.to_string()
    }
}
```

**Erklärung:**
- `FOR UPDATE` innerhalb des `transaction`-Blocks reserviert die Zeile mit einer exklusiven Datenbanksperre.
- `scoped by tenant_id` und `unique within tenant fiscal_year` garantieren, dass jeder Mandant für jedes Geschäftsjahr einen unabhängigen, kollisionsfreien Nummernkreis besitzt.

---

#### Rezept K.5: Revisionssicherer Audit-Trail (Temporal Ledger)

**Problem:**
Jede Preis- oder Statusänderung an schützenswerten Datensätzen muss unveränderlich mit ausführendem Benutzer und Zeitstempel dokumentiert werden.

**Lösung:**
```zelyra
table audit_ledger scoped by tenant_id: TenantId {
    id: Id primary auto
    tenant_id: TenantId required
    actor_id: Id required
    entity_name: String(64) required
    entity_id: Id required
    action_type: String(32) required
    payload_hash: String(64) required
    created_at: Instant required
}

fn record_audit_entry(
    t_id: TenantId,
    actor: Id,
    entity: String,
    target_id: Id,
    action: String,
    raw_payload: String
) uses Database, Clock {
    let now_ts = now()
    let hash = hash_sha256(raw_payload)
    execute(
        "INSERT INTO audit_ledger (tenant_id, actor_id, entity_name, entity_id, action_type, payload_hash, created_at) VALUES (?, ?, ?, ?, ?, ?, ?)",
        [t_id, actor, entity, target_id, action, hash, now_ts]
    )
}
```

**Erklärung:**
- Die Funktion deklariert die Capabilities `Database` und `Clock`. In Tests kann die Zeitquelle deterministisch manipuliert werden, während in Produktion echte UTC-Zeitstempel geschrieben werden.
- Der SHA-256-Hash des JSON-Payloads sichert die Unveränderlichkeit des protokollierten Zustands ab.

---

#### Rezept K.6: Keyset-Paginierung für große Tabellen

**Problem:**
Bei Tabellen mit Millionen Zeilen führt `SELECT ... LIMIT 50 OFFSET 500000` zu spürbaren Datenbanklatenzen, da die Datenbank alle übersprungenen Zeilen sequenziell einlesen muss.

**Lösung:**
```zelyra
table customer_logs scoped by tenant_id: TenantId {
    id: Id primary auto
    tenant_id: TenantId required
    created_at: Instant required
    message: String(255) required

    index idx_tenant_cursor (tenant_id, created_at, id)
}

fn fetch_log_page(t_id: TenantId, last_ts: Instant, last_id: Id, page_size: Int) -> CustomerLog[] uses Database {
    return query(
        "SELECT id, tenant_id, created_at, message FROM customer_logs WHERE tenant_id = ? AND (created_at < ? OR (created_at = ? AND id < ?)) ORDER BY created_at DESC, id DESC LIMIT ?",
        [t_id, last_ts, last_ts, last_id, page_size]
    )
}
```

**Erklärung:**
- Keyset-Paginierung nutzt den zusammengesetzten Index `(tenant_id, created_at, id)` optimal aus. Die Abfragegeschwindigkeit bleibt bei Seite 10.000 exakt genauso hoch wie bei Seite 1.

---

### Teil 3: Sicherheit, Authentifizierung & Governance

#### Rezept K.7: TOTP-Zwei-Faktor-Authentifizierung (2FA)

**Problem:**
Sensible Benutzerkonten müssen durch RFC-6238-basierte Einmalkennwörter (Time-Based One-Time Passwords) und sichere Wiederherstellungscodes geschützt werden.

**Lösung:**
```zelyra
table user_mfa_settings scoped by tenant_id: TenantId {
    id: Id primary auto
    tenant_id: TenantId required
    user_id: Id required unique
    secret_key: String(64) required
    is_enabled: Bool required default false
    backup_code_hash: String(128) required
}

fn verify_totp_attempt(user_secret: String, user_token: String) -> Bool uses Clock {
    let valid = totp_verify(user_secret, user_token, 30)
    return valid
}
```

**Erklärung:**
- `totp_verify` toleriert eine Drift von ±1 Zeitschritt (30 Sekunden), um minimale Uhrzeitabweichungen auf Mobiltelefonen abzufedern.

---

#### Rezept K.8: Mandantengebundene API mit Rollenprüfung

**Problem:**
Eine JSON-API zur Freigabe von Belegen muss sicherstellen, dass Benutzer nur Daten des eigenen Mandanten manipulieren und zwingend über die Rolle `AccountingManager` verfügen.

**Lösung:**
```zelyra
table vouchers scoped by tenant_id: TenantId {
    id: Id primary auto
    tenant_id: TenantId required
    voucher_number: String(40) required
    status: String(20) required default "draft"
    total_amount: Decimal(12, 2) required
}

api approve_voucher(voucher_id: Id) scoped by tenant_id {
    permission: "vouchers.approve"
    validates {
        voucher_id > 0: "Ungültige Beleg-ID."
    }

    execute uses Database {
        transaction {
            let row = query_one(
                "SELECT id, status FROM vouchers WHERE id = ? AND tenant_id = ? FOR UPDATE",
                [voucher_id, session.tenant_id]
            )
            if row.status == "approved" {
                return { success: false, error: "Beleg ist bereits freigegeben." }
            }
            execute(
                "UPDATE vouchers SET status = 'approved' WHERE id = ? AND tenant_id = ?",
                [voucher_id, session.tenant_id]
            )
            return { success: true, message: "Beleg erfolgreich genehmigt." }
        }
    }
}
```

**Erklärung:**
- `scoped by tenant_id` verknüpft die API untrennbar mit der aktiven Mandantensitzung (`session.tenant_id`). Ein Zugriff auf fremde Mandantendaten ist auf Sprachebene ausgeschlossen.

---

#### Rezept K.9: Schutz vor Brute-Force durch Ratenbegrenzung

**Problem:**
Passwort-Reset- und Login-Endpunkte müssen vor Wörterbuchangriffen und Credential-Stuffing geschützt werden.

**Lösung:**
```zelyra
table rate_limits {
    id: Id primary auto
    client_ip: String(45) required
    action: String(30) required
    attempts: Int required default 1
    locked_until: Instant?

    index idx_client_action (client_ip, action)
}

fn check_rate_limit(ip: String, action_name: String, max_attempts: Int) -> Bool uses Database, Clock {
    let now_ts = now()
    let record = query_one_optional(
        "SELECT id, attempts, locked_until FROM rate_limits WHERE client_ip = ? AND action = ?",
        [ip, action_name]
    )
    match record {
        Option::Some(r) => {
            if r.attempts >= max_attempts {
                return false
            }
            execute("UPDATE rate_limits SET attempts = attempts + 1 WHERE id = ?", [r.id])
            return true
        }
        Option::None => {
            execute("INSERT INTO rate_limits (client_ip, action, attempts) VALUES (?, ?, 1)", [ip, action_name])
            return true
        }
    }
}
```

**Erklärung:**
- Das Muster blockiert weitere Versuche, sobald `max_attempts` erreicht wird. Die IP-basierte Prüfung schützt CPU und Datenbank vor Überlastung.

---

### Teil 4: Web, CRUD & Master-Detail

#### Rezept K.10: Master-Detail-Rechnungsformular mit Positionen (`has_many`)

**Problem:**
Ein Rechnungsbeleg besteht aus Kopfdaten (Kunde, Rechnungsdatum) und einer Liste beliebig vieler Belegpositionen, die gemeinsam in einer Transaktion gespeichert und aufsummiert werden müssen.

**Lösung:**
```zelyra
table invoices scoped by tenant_id: TenantId {
    id: Id primary auto
    tenant_id: TenantId required
    invoice_number: String(32) required unique within tenant
    invoice_date: Date required
    customer_name: String(100) required
    total_net: Decimal(12, 2) required default decimal("0.00")
    total_gross: Decimal(12, 2) required default decimal("0.00")
}

table invoice_items scoped by tenant_id: TenantId {
    id: Id primary auto
    tenant_id: TenantId required
    invoice: Invoice required on_delete cascade
    description: String(120) required
    quantity: Decimal(8, 2) required
    unit_price: Decimal(10, 2) required
    line_total: Decimal(12, 2) required
}

crud Invoice -> invoices {
    title: "Rechnung erfassen"

    has_many items: InvoiceItem {
        title: "Rechnungspositionen"
        fields: [description, quantity, unit_price, line_total]
    }

    aggregates {
        total_net = sum(items.line_total)
        total_gross = sum(items.line_total) * decimal("1.19")
    }
}
```

**Erklärung:**
- `has_many items` erzeugt eine tabellarische Kinddatensatz-Eingabe direkt im Hauptformular.
- Der Block `aggregates` berechnet die Gesamtsummen serverseitig in der Speichertransaktion. Manipulationen über das Browser-Frontend werden unterbunden.

---

#### Rezept K.11: GoBD-Schutz mit Lifecycle-Guards

**Problem:**
Rechnungen, die bereits an den Kunden versandt oder bezahlt wurden, dürfen aus steuerrechtlichen Gründen nicht mehr nachträglich verändert oder gelöscht werden.

**Lösung:**
```zelyra
table sales_invoices scoped by tenant_id: TenantId {
    id: Id primary auto
    tenant_id: TenantId required
    doc_number: String(30) required
    is_finalized: Bool required default false
    is_cancelled: Bool required default false
}

crud SalesInvoice -> sales_invoices {
    title: "Ausgangsrechnungen"

    guard edit when is_finalized == true {
        error: "Festgeschriebene Belege können nicht mehr editiert werden. Bitte Stornobuchung durchführen."
    }

    guard delete when is_finalized == true {
        error: "Festgeschriebene Rechnungen dürfen gemäß GoBD niemals gelöscht werden."
    }
}
```

**Erklärung:**
- Zelyra führt Guards unmittelbar in der Datenbank-Mutationstransaktion aus.
- TableView-Aktionsleisten und Schaltflächen blenden Bearbeiten- und Löschen-Aktionen für gesperrte Datensätze automatisch aus.

---

#### Rezept K.12: Feldübergreifende Invariantenprüfungen (`validate`)

**Problem:**
Bei Hotel- oder Fahrzeugbuchungen darf das Abreisedatum niemals vor oder am Anreisedatum liegen.

**Lösung:**
```zelyra
table hotel_bookings scoped by tenant_id: TenantId {
    id: Id primary auto
    tenant_id: TenantId required
    guest_name: String(100) required
    check_in: Date required
    check_out: Date required
    rooms_count: Int required default 1

    validate {
        check_out > check_in: "Das Abreisedatum muss nach dem Anreisedatum liegen."
        rooms_count >= 1 && rooms_count <= 10: "Es können maximal 10 Zimmer pro Buchung reserviert werden."
    }
}
```

**Erklärung:**
- Invarianten werden sowohl bei API-Aufrufen als auch in CRUD-Formularen vor dem Datenbank-Write ausgewertet.
- Bei Verletzung antwortet Zelyra automatisch mit standardisierten HTTP-422-Fehlermeldungen und markiert die betroffenen Formularfelder mit `aria-invalid="true"`.

---

### Teil 5: Hintergrund-Dienste, E-Mail & Export

#### Rezept K.13: Transaktionale PDF-Erstellung und Mail-Outbox

**Problem:**
Nach Abschluss einer Bestellung muss eine PDF-Auftragsbestätigung erzeugt und per E-Mail versandt werden. Schlägt die Datenbanktransaktion fehl, darf keine E-Mail versendet werden.

**Lösung:**
```zelyra
fn finalize_order(order_id: Id, recipient_mail: String) uses Database {
    transaction {
        let order = query_one("SELECT id, order_number, total_gross FROM orders WHERE id = ?", [order_id])
        let pdf_bytes = render_pdf("templates/OrderConfirmation.html", order)
        mail_queue(
            recipient_mail,
            "Ihre Auftragsbestätigung " + order.order_number,
            "Vielen Dank für Ihre Bestellung. Im Anhang finden Sie Ihre Bestätigung.",
            order.order_number + ".pdf",
            pdf_bytes
        )
        execute("UPDATE orders SET status = 'confirmed' WHERE id = ?", [order_id])
    }
}
```

**Erklärung:**
- `mail_queue` schreibt die Nachricht in die MariaDB-Tabelle `mail_queue` innerhalb der laufenden Transaktion.
- Bricht die Transaktion ab (Rollback), wird auch der E-Mail-Auftrag verworfen. Erst wenn der Commit erfolgreich ist, greift der Hintergrund-Worker die Mail auf.

---

#### Rezept K.14: Idempotente Cronjob-Abarbeitung mit Leases

**Problem:**
Ein Hintergrund-Job zur Prüfung offener Mahnungen soll jede Nacht um 02:00 Uhr laufen. Mehrere Serverinstanzen dürfen den Job niemals doppelt ausführen.

**Lösung:**
```zelyra
schedule "0 2 * * *" {
    name: "nightly_payment_reminders"
    timeout: 3600

    execute uses Database, Clock {
        transaction {
            let pending_invoices = query(
                "SELECT id, customer_mail, due_date FROM invoices WHERE status = 'open' AND due_date < CURRENT_DATE() FOR UPDATE SKIP LOCKED",
                []
            )
            for inv in pending_invoices {
                execute("UPDATE invoices SET status = 'reminded' WHERE id = ?", [inv.id])
            }
        }
    }
}
```

**Erklärung:**
- `SKIP LOCKED` stellt sicher, dass konkurrierende Worker nur nicht blockierte Datensätze verarbeiten.
- Der Scheduler vergibt automatische Leases mit Timeouts, sodass abgestürzte Prozesse nach Ablauf des Zeitfensters automatisch freigegeben werden.

---

#### Rezept K.15: Streaming-CSV-Export für DATEV / Buchhaltung

**Problem:**
Für den Steuerberater müssen hunderttausende Buchungssätze in ein normiertes CSV-Format exportiert werden, ohne dass der Arbeitsspeicher des Webservers überläuft.

**Lösung:**
```zelyra
fn export_datev_csv(t_id: TenantId, year: Int) -> String uses Database {
    mutable csv_output = "Datum;Beleg;Konto;Gegenkonto;Betrag;Text\n"
    let entries = query(
        "SELECT booking_date, voucher_no, account, contra_account, amount, notice FROM bookings WHERE tenant_id = ? AND fiscal_year = ? ORDER BY booking_date ASC",
        [t_id, year]
    )
    for row in entries {
        csv_output = csv_output + row.booking_date.to_string() + ";" + row.voucher_no + ";" + row.account.to_string() + ";" + row.contra_account.to_string() + ";" + row.amount.to_string() + ";" + row.notice + "\n"
    }
    return csv_output
}
```

**Erklärung:**
- Die typisierte Iteration stellt sicher, dass Währungswerte und Datumsfelder exakt nach vorgegebenen Formatierungsstandards exportiert werden.
