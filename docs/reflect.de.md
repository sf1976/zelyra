# Projektmetadaten exportieren

`zelyra reflect main.zyl --format=json` prüft das Projekt mit dem normalen
Projektloader und Compilerpfad und gibt danach einen deterministischen
JSON-Export aus. Die oberste Eigenschaft `reflect_schema_version` versioniert
dieses Exportformat unabhängig von der allgemeinen CLI-Diagnoseschnittstelle.

Der Export enthält aktuell:

- `source_files` und `modules` mit projektbezogenen Importen und Exporten,
- `symbols` für Funktionen, Records, Enums, Aliase, Views und Komponenten,
- `tables` samt Spaltentypen und strukturellen Eigenschaften,
- `routes` einschließlich generierter Formular-, CRUD- und Auth-Routen,
- `apis` mit Ein-/Ausgabetypen, Fehlerstatus und Berechtigungsmetadaten,
- `forms` mit Feldtypen, Validierungsregeln und Aktionsmetadaten,
- Quellpositionen relativ zum Projekt.

Beispiel:

```sh
zelyra reflect main.zyl --format=json > project-reflection.json
```

Der Befehl führt keinen Programmcode aus, verbindet sich nicht mit einer
Datenbank und liest keine `.env`-Datei. UI-Texte, SQL-Abfragetexte,
Standardwerte, Datenbankinhalte und Laufzeitwerte werden nicht exportiert.
`completeness.live_database_inspected` steht deshalb auf `false`.

Der Export ist ein statisches Inventar, kein Laufzeit- oder Live-Schema-Beweis.
Aktuell werden keine Markdown- oder OpenAPI-Exporte erzeugt. Prüfe nach einer
Änderung des Exportformats `reflect_schema_version` und aktualisiere
verbrauchende Werkzeuge entsprechend.
