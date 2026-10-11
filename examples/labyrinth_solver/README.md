# Labyrinthsuche mit Breitensuche

Dieses mehrteilige Zelyra-Konsolenbeispiel lädt ein rechteckiges Labyrinth,
findet mit Breitensuche einen kürzesten Weg und zeigt Weg, Start, Ziel,
Schrittzahl und untersuchte Felder an.

## Start und Tests

Im Repository:

```sh
cargo run -p zelyra-cli -- run examples/labyrinth_solver/main.zyl
cargo run -p zelyra-cli -- run examples/labyrinth_solver/tests.zyl
```

Mit installierter CLI im Projektverzeichnis:

```sh
cd examples/labyrinth_solver
zelyra run main.zyl
zelyra run tests.zyl
```

Das Programm lädt `data/maze.txt`. `S` markiert den Start, `G` das Ziel,
`.` ein freies Feld und `#` eine Wand. Raster dürfen bis zu 100 Zeilen und
100 Spalten groß sein. Beim Laden werden Rechteckigkeit, gültige Zeichen,
Start und Ziel geprüft. CRLF-Dateien und ein abschließender Zeilenumbruch
werden unterstützt.

## Algorithmus und Ausgabe

Die Suche verwendet eine FIFO-Warteschlange, eine Besuchs-/Vorgängermenge und
eine Zahl für den nächsten Warteschlangeneintrag. Jedes Feld wird beim
Entdecken markiert, bevor es eingereiht wird; dadurch kommt es höchstens einmal
in die Warteschlange. Nachbarn werden nur nach oben, unten, links und rechts
betrachtet und erst nach einer Randprüfung adressiert. Die Vorgänger werden
vom Ziel zum Start zurückverfolgt; die gespeicherte Pfadlänge ist damit der
kürzeste Weg in Schritten.

`*` zeichnet begehbare Wegfelder ein; `S` und `G` bleiben sichtbar. Die
Koordinaten beginnen bei `(0, 0)` links oben. „Untersuchte Felder“ zählt
entnommene Warteschlangeneinträge bis einschließlich Ziel beziehungsweise bis
die Warteschlange leer ist. Bei unerreichbarem Ziel wird dies ausdrücklich
gemeldet.

## Dateiformat und Tests

Die Labyrinthdatei ist UTF-8-Text mit einer Rasterzeile pro Zeile und den vier
oben genannten ASCII-Zeichen. Gespeicherte Raster werden atomar ersetzt.
`tests.zyl` prüft kürzeste Wege, Randstart, Sackgasse, unerreichbares Ziel,
geschlossene Bereiche, Dateiroundtrip, CRLF sowie fehlerhafte Zeilenlängen,
Zeichen und Start-/Zielanzahl. Ein fehlgeschlagener Test beendet Zelyras
Programmlauf mit einem Fehlerstatus.
