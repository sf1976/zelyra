# Textänderungen sicher planen

`zelyra text edit` liest einen TOML-Plan und zeigt standardmäßig eine Vorschau.
Dateien werden erst geändert, wenn du denselben Plan mit der angezeigten
SHA-256-ID ausdrücklich bestätigst.

```toml
[[change]]
name = "Nachnamen zuerst anzeigen"
files = ["src/**/*.zyl"]
find = '''(?P<vorname> [[:alpha:]]+ )\s+(?P<nachname> [[:alpha:]]+ )'''
replace = "${nachname}, ${vorname}"
expect = 3
```

Im Projektverzeichnis:

```sh
zelyra text edit namen.zedit
zelyra text edit namen.zedit --apply --plan-id sha256:<ID-aus-der-Vorschau>
```

Die erste Zeile zeigt die Trefferzahl je Regel, betroffene Dateien und einen
Unified-Diff. Stimmen Trefferzahl oder Eingabeinhalt nicht, wird abgebrochen.
Jede Regel braucht mindestens einen erwarteten Treffer. Zwei Regeln dürfen
nicht dieselbe Datei auswählen; so bleibt die Vorschau eindeutig.

Pläne und Dateien müssen innerhalb eines Zelyra-Projekts liegen. Muster sind
relativ und unterstützen `*`, `?` und `**`; `..`, absolute Pfade und
Symlinks werden abgelehnt. `.git`, `target` und `node_modules` bleiben
ausgeschlossen. Ein Plan kann höchstens 100 Regeln und 500 Dateien enthalten;
einzelne Pläne sind auf 256 KiB, Dateien auf 2 MiB begrenzt. Es wird strikt
UTF-8 gelesen. Regeln verwenden Rusts lineare Regex-Engine; Rückverweise,
Look-around und auszuführende Ersetzungen werden nicht unterstützt.

Die Vorschau prüft keine Zelyra-Semantik. Führe nach einer Quelltextänderung
`zelyra check` oder `zelyra build` aus.
