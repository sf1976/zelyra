# Lesbare Textbearbeitung mit regulären Ausdrücken

**Status:** Entwurf für den geplanten Meilenstein 0.8.0; noch kein
implementierter CLI-Vertrag.

Wiederkehrende Quelltextänderungen sollen auf einen Blick verständlich sein.
Ein Änderungsplan benennt Zweck, Zieldateien, regulären Ausdruck, Ersetzung und
erwartete Trefferzahl. Standardmäßig erscheint eine Vorschau. Dateien werden
nur mit ausdrücklichem `--apply` geschrieben.

## Beispiel

Eine TOML-Datei hält Erklärung und Ausdruck zusammen:

```toml
[[change]]
name = "Familienname zuerst anzeigen"
files = ["src/**/*.zyl"]
find = '''(?x)
  (?P<vorname> [[:alpha:]]+ )
  \s+
  (?P<familienname> [[:alpha:]]+ )
'''
replace = "${familienname}, ${vorname}"
expect = 3
```

Vom Projektstamm ausführen:

```sh
zelyra text edit namen.zedit
# Die Vorschau gibt eine SHA-256-Plan-ID für Regeln und Eingabedateien aus.
zelyra text edit namen.zedit --apply --plan-id sha256:<id-aus-der-vorschau>
```

Der erste Befehl zeigt jede Datei, die Trefferzahl, den vorgeschlagenen Diff
und eine SHA-256-Plan-ID, ohne Dateien zu ändern. Der zweite wendet genau
diesen geprüften Plan an. Zelyra berechnet die ID aus Regeln und aktuellem
Dateiinhalt neu; wurde seit der Vorschau etwas geändert, wird abgebrochen.
Weicht die Trefferzahl von `expect` ab, wird ebenfalls abgebrochen; die CLI
nennt Dateien und Trefferzahlen. Das Anwenden ohne die ausgegebene Plan-ID
wird abgelehnt. Eine breite Regel kann so nach der Vorschau nicht unbemerkt
eine andere Menge Code ändern.

## Sprach- und Sicherheitsregeln

- Eine dokumentierte Regex-Engine mit linearer Laufzeit verwenden. Benannte
  Gruppen haben die Form `(?P<name>...)`, Ersetzungen greifen mit `${name}`
  darauf zu. Rückverweise im Muster, Look-around, Codeauswertung und
  Shell-Interpolation werden abgelehnt.
- Regex-Optionen stehen direkt im Muster (`(?i)` für Groß-/Kleinschreibung,
  `(?m)` für mehrzeilige Anker und `(?x)` für Leerraum und Kommentare). So gibt
  es keine versteckten Kommandozeilenmodi.
- Dateimuster bleiben unterhalb des erkannten Projektstamms. Symlinks nach
  außerhalb werden nicht verfolgt; `.git`, Build-Ausgaben und Abhängigkeits-
  Caches sind standardmäßig ausgeschlossen. Vor dem Anwenden werden alle
  einbezogenen Dateien aufgelistet.
- UTF-8 wird strikt gelesen, Zeilenenden jeder Datei bleiben erhalten. Binäre
  oder zu große Eingaben werden mit einer klaren Diagnose abgelehnt. Kein
  Treffer, eine abweichende Trefferzahl, überlappende Regeln oder eine seit
  der Vorschau geänderte Datei blockieren die Anwendung.
- Erst alle vorgeschlagenen Inhalte vorbereiten und prüfen, dann jede Datei
  atomar ersetzen, Dateirechte erhalten und bis zum Abschluss eine
  Wiederherstellungskopie behalten. Zum Schluss folgt eine Zusammenfassung pro
  Datei.
- Im Hintergrund werden weder `sed`, Perl noch eine Benutzershell aufgerufen.
  Das Verhalten ist auf Windows, macOS und Linux gleich.

## Umfangsgrenze

Das ist ein geführter Quelltext-Änderungsbefehl, keine allgemeine
Skriptsprache und kein IDE-Suchdialog. Die erste Version bearbeitet UTF-8-Text
innerhalb eines Projektverzeichnisses und bietet reproduzierbare Vorschau,
ausdrückliches Anwenden und wiederherstellbares Fehlerverhalten. Sie ändert
keine Syntaxsemantik; eine Compilerprüfung nach der Anwendung kann für
Zelyra-Dateien optional sein.
