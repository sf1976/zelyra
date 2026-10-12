# Sprachkompatibilität von Zelyra

Die Compiler-Paketversion und die Sprachkompatibilitätslinie beschreiben
unterschiedliche Dinge. Ein Compiler-Release wie `0.7.0` kann Funktionen
ergänzen, während ein Projekt weiterhin auf Sprachlinie `0.1` zielt.

## Projekterklärung

Erzeugte Projekte tragen die Linie in `zelyra.toml` ein:

```toml
[project]
name = "rechnungs-app"
version = "0.1.0"
zelyra = "0.1"
```

Der Compiler 0.7 akzeptiert für `zelyra check`, `build`, `run` und `serve` die
Linie `0.1`. Eine ausdrücklich verlangte, vom Compiler nicht unterstützte Linie
bricht mit `E-LANG-001` ab. Projekte ohne `zelyra`-Angabe verwenden weiterhin
`0.1` als historischen Standard; ältere Projekte bleiben dadurch lauffähig.
Das Feld wählt einen Sprachvertrag, weder ein Datenbank-Backend noch eine
Compiler-Binärdatei.

## Umfang der Linie 0.1

Die Linie umfasst Quellgrammatik, Typ- und Namensregeln sowie das Verhalten der
Sprachkonstrukte und eingebauten Funktionen. Die aktuelle Grammatik und
unterstützten Konstrukte beschreiben der
[Spezifikationsindex](specification.de.md) und die Phasenreferenzen. CLI-
Maschinenschemata, Projekteinstellungen, Datenbank-Backends und erzeugtes
Webverhalten besitzen eigene dokumentierte Verträge.

Solange Zelyra vor 1.0 steht, wächst die Linie 0.1 aktiv und bleibt ein
experimentelles Kompatibilitätsziel. Ergänzungen dürfen die Sprache erweitern,
müssen aber gültige 0.1-Programme weiterhin gültig lassen. Eine Änderung, die
gültigen Quelltext absichtlich bricht oder die Bedeutung einer vorhandenen
Konstruktion ändert, benötigt eine eigene Sprachlinie und Migrationshinweise.
Sie darf nicht stillschweigend unter `0.1` erscheinen.

## Ausbau bis 1.0.0

Die Sprachlinie `0.1` bleibt während aller Compiler-Releases bis einschließlich
`0.9.9` dieselbe Linie. Versionsnummern wie `0.8.0` und `0.9.9` bezeichnen
Zelyra-Releases, keine neuen Sprachlinien. Ein Projekt wechselt seine
Sprachlinie niemals automatisch beim Compiler-Upgrade. Eine neue Linie wird
erst eingeführt, wenn eine notwendige Änderung gültige Programme oder deren
bisherige Bedeutung bricht; sie braucht eine eigene Spezifikation,
Migrationshinweise und eine explizite Projektänderung.

Der Ausbau erfolgt in prüfbaren Stufen:

| Release | Sprachvertrag und Nachweis |
| --- | --- |
| `0.7.0` | `zelyra = "0.1"` erzwingen, nicht unterstützte explizite Linien mit `E-LANG-001` melden, Projekte ohne Feld beibehalten und ein erstes ausführbares Korpus für akzeptierte, abgelehnte und zur Laufzeit geprüfte Beispiele ergänzen. |
| `0.8.0` | Korpus für Lexing, Parsing, Typprüfung und Formatierung systematisch ausbauen. Jede Grammatikfamilie erhält positive und negative Fixtures; Formatter-Ausgabe wird auf Idempotenz geprüft. |
| `0.9.0` | ✅ Laufzeit- und Modul-Fixtures zu Operatoren, eingebauten Funktionen, Fehlerpfaden, Effekten, Contracts, Imports und SQL-Typisierung erweitern; zugesagte Datenbankabläufe separat prüfen. |
| `0.9.1–0.9.4` | `#` als Standard für Zeilenkommentare und alleinstehendes `##` als nicht verschachteltes Blockkommentar-Paar ergänzen; `//` kompatibel halten. Compiler-gestützte IDE-Dienste und Mehrdateitests gemeinsam ausbauen: Diagnosen, Quellpositionen, Vervollständigung, Navigation, Formatierung, SQL-/Schemasymbole und Formulargenerierung müssen den dokumentierten Sprachvertrag verwenden. |
| `0.9.5–0.9.6` | Effekte und Capabilities für Mail, Templates und Benachrichtigungsereignisse ausdrücklich festlegen; akzeptierte, abgelehnte und Laufzeit-Fixtures für Geheimnisse, Zustellfehler und Wiederholungen ergänzen. |
| `0.9.7–0.9.8` | ✅ Kritische Lücken aus dem 0.9.7-Korpus mit Regressionstests schließen: kontrollierte Rekursion, vollständige Rückgabepfade, `Int::MIN`, Kalender-/Zeitstempelarithmetik, Decimal-Quantisierung, payloadfreie Enums, exklusive Bereichsschleifen, wertsemantische Record-/Arrayänderungen und optionales Laufzeitbudget. Der explizite getaggte JSON-Codec für `Result` ist enthalten. |
| `0.9.9` | 🧪 Kernsprache vor dem Freeze vervollständigen: generische Funktionen, Records und Enums, payloadtragende Varianten sowie benannte typisierte Funktionswerte und sichere indirekte Aufrufe. Verbleibende Grenzen wie allgemeine Constraints, Closures und generische Funktionsreferenzen ausdrücklich dokumentieren; vollständigen Sprachkorpus und alle technischen Release-Gates bestehen, bevor Grammatik, Typregeln und Semantik von `0.1` eingefroren werden. |
| `1.0.0` | Nach unabhängiger menschlicher Abnahme den geprüften Vertrag, das Korpus und die Supportmatrix gemeinsam als erste produktive Sprachversion veröffentlichen. |

Diese Stufen sind Abnahmekriterien und keine Behauptung, dass geplante Arbeit
bereits implementiert ist. Die Stufen 0.7 bis 0.9 sind umgesetzt; der Korpus
bleibt eine wachsende, begrenzte Sammlung und ist keine vollständige
releaseübergreifende Konformitätsprüfung. Er friert den Sprachvertrag nicht für
den Produktiveinsatz ein. Das 1.0-Release
muss unterstützte Grammatik, Kompatibilitätsversprechen und geprüfte
Konformitätssammlung gemeinsam veröffentlichen.

## Was Kompatibilität bedeutet

Für eine festgelegte `0.1`-Version gelten diese Regeln:

- Ein gültiges Programm bleibt syntaktisch gültig und typkorrekt. Neue
  Schlüsselwörter, Operatoren oder eingebaute Namen dürfen es nicht
  nachträglich umdeuten oder ungültig machen.
- Definierte Laufzeitsemantik ändert sich nicht still. Bugfixes, die beobachtbares
  Verhalten gültiger Programme ändern, benötigen eine dokumentierte Bewertung,
  einen Regressionstest und gegebenenfalls eine neue Sprachlinie.
- Neu eingeführte Funktionen müssen über Syntax, Typen, Effekte, Diagnose,
  Formatter und Laufzeit hinweg zusammenpassen. Unvollständige oder
  backendabhängige Funktionen werden als solche gekennzeichnet und dürfen nicht
  als allgemeine Sprachgarantie erscheinen.
- Neue Compiler-Versionen behalten die Unterstützung deklarierter Linien bei,
  solange sie in ihrer Supportmatrix stehen. Das Compiler-Paket darf unabhängig
  davon neue Funktionen und Fehlerkorrekturen erhalten.
- Diagnosecodes sind die stabile Schnittstelle für Werkzeuge. Diagnosetexte und
  Quellpositionen dürfen präziser werden; Tests sichern Codes, relevante Spannen
  und verständliche Beispiele.
- Formatter-Ausgaben müssen idempotent sein und dürfen die Bedeutung des
  Programms nicht ändern. Eine Stiländerung ist dokumentiert und kein
  Sprachlinienwechsel.

Compiler-Binärdateien, CLI-Maschinenschemata, Projektkonfiguration,
Datenbanktreiber und generiertes Webverhalten werden über ihre eigenen
Support- und Versionsregeln verwaltet. Die Sprachlinie verspricht keine
automatische Portabilität zwischen Backends.

## Kompatibilitätsprüfungen

Jedes Release mit Unterstützung für 0.1 muss den versionierten Sprachkorpus
ausführen. Ein Fixture benennt Eingabe, erwartete Phase, Erfolg oder stabilen
Diagnosecode und – falls relevant – erwartetes Laufzeitresultat. Golden-Tests
für Maschinenausgaben bleiben an ihre separat versionierten Schemata gebunden.
Das Korpus deckt gültige und ungültige Syntax, Diagnosen, Unicode-Quellpositionen,
Arithmetik und Decimal-Regeln, Option/Result, Effekte, Verträge, SQL-Typprüfung,
Modulgrenzen und Formatter-Idempotenz ab. Neue Sprachfunktionen bringen
positive und negative Fixtures mit. Änderungen an erwarteten Ergebnissen
brauchen eine Begründung in denselben Review-Unterlagen.

Die [Release-Roadmap](ROADMAP.de.md) beschreibt die geplante
Konformitätssammlung. Das [Implementierungsverzeichnis](implemented.de.md)
führt den tatsächlich veröffentlichten Stand auf.
