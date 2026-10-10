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

Der 0.7-Compiler erzwingt die deklarierte Linie, enthält aber noch keine
vollständige releaseübergreifende Konformitätstestsammlung und friert den
Sprachvertrag 0.1 nicht für den Produktiveinsatz ein. Beides bleibt Arbeit auf
dem Weg zu 1.0.0. Das 1.0-Release muss unterstützte Grammatik,
Kompatibilitätsversprechen und geprüfte Konformitätssammlung gemeinsam
veröffentlichen.

## Kompatibilitätsprüfungen

Jedes Release mit Unterstützung für 0.1 muss Sprach-Fixtures durch Lexing,
Parsing, Typprüfung, Formatierung und – wo passend – Ausführung schicken.
Fixtures sollen gültige und ungültige Syntax, Diagnosen, Unicode-Quellpositionen,
Arithmetik und Decimal-Regeln, Option/Result, Effekte, Verträge, SQL-Typprüfung
und Modulgrenzen abdecken. Golden-Tests für Maschinenausgaben bleiben an ihre
separat versionierten Schemata gebunden.

Die [Release-Roadmap](ROADMAP.de.md) beschreibt die geplante
Konformitätssammlung. Das [Implementierungsverzeichnis](implemented.de.md)
führt den tatsächlich veröffentlichten Stand auf.
