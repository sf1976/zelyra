# Statische Dateien mit `public/`

`zelyra init` und `zelyra new` legen `public/` als expliziten Ordner für
öffentlich erreichbare Dateien an. `zelyra serve` liefert Dateien aus diesem
Ordner aus, wenn keine deklarierte Anwendungsroute die Anfrage verarbeitet.
Die Route `/` bleibt eine normale Zelyra-Route aus `main.zyl`; `public/index.html`
wird unter `/index.html` erreicht und ersetzt nicht automatisch die Anwendung.
Ist `public/` eine Datei oder ein Symlink, bricht der Start mit `E-WEB-006` ab.

```text
projekt/
├── main.zyl
└── public/
    ├── bilder/logo.png
    ├── css/site.css
    └── dokumente/agb.pdf
```

Diese Dateien sind dann unter `/bilder/logo.png`, `/css/site.css` und
`/dokumente/agb.pdf` verfügbar. Apache und Nginx können weiter als Reverse
Proxy vor `zelyra serve` stehen; sie führen keine Zelyra-Quelldateien aus.

## Grenzen und Schutz

- Nur `GET` und `HEAD` liefern statische Dateien aus. Anwendungsrouten werden
  zuerst verarbeitet und haben Vorrang.
- Der öffentliche Stammordner muss ein echtes Verzeichnis sein. Ein Symlink
  als `public/` oder innerhalb eines angefragten Dateipfads wird abgewiesen.
  Auch Symlinks, deren Ziel innerhalb von `public/` liegt, werden nicht
  ausgeliefert.
- Pfadsegmente werden dekodiert und geprüft. `..`, codierte Trenner,
  Backslashes, fehlerhaftes UTF-8 und versteckte Dateien werden nicht
  ausgeliefert; `.well-known/` ist für standardisierte Web-Challenges erlaubt.
- Einzelne Dateien sind auf 4 MiB begrenzt, damit eine Anfrage nicht
  unbegrenzt Speicher belegen kann. Größere Dateien liefern HTTP 413.
- Der Content-Type wird anhand einer begrenzten Erweiterungsliste bestimmt;
  unbekannte Erweiterungen erhalten `application/octet-stream`. Antworten
  enthalten `X-Content-Type-Options: nosniff` und werden eine Stunde gecacht.
- Die Inhalte sind öffentlich. Speichere keine `.env`, Zugangsdaten oder
  privaten Dateien unter `public/`.

Die statische Ablage unterstützt keine Verzeichnisauflistung, Byte-Ranges,
Upload- oder Änderungsfunktionen. Für große Downloads oder private Dateien
verwende eine dafür geeignete Anwendungsroute oder einen dedizierten
Dateidienst.
