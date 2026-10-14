# NULL-bewusste Edit-Snapshots

Stand: aktuelles stabiles Release Zelyra 1.0.4. [English](edit-snapshots.en.md)

Erzeugte MariaDB-/MySQL-Editformulare signieren die aus der Datenbank geladenen
Feldwerte. Vor einem Update sperrt und liest dieselbe Transaktion den aktuellen
Datensatz, prüft den Snapshot und schreibt erst danach. Geänderte Datensätze
liefern HTTP 409; das vorgesehene Update wird nicht ausgeführt.

Native SQL-NULL-Masken gehören zu den signierten Daten. NULL, der Literalstring
`\N` und ein leerer String sind unterschiedliche Zustände. Aus Treibertext wird
keine Abwesenheit geraten. Fehlende/inkonsistente NULL-Metadaten, falsche
Zeilenbreiten oder mehrere Zeilen werden abgewiesen. Die Signatur bleibt an
CSRF-Schlüssel und Feldmenge gebunden und verwendet das bestehende HMAC-SHA256-
Format mit 64 Hexzeichen. Nicht-NULL-Payloads behalten ihre bisherigen Signaturen.
Vor dieser Änderung geöffnete Formulare mit NULL-Feldern können ein Neuladen
benötigen, weil ihre alten Signaturen NULL nicht eigens erfassten.

Echtes NULL erscheint als leere Eingabe; Literaltext `\N` bleibt sichtbar.
Fehlende oder leere optionale Felder binden natives SQL-NULL, auch bei nullable
Spalten ohne ausdrückliches `Option<T>`. Pflichtfeld-/Readonly-Prüfungen bleiben
wirksam. Leere optionale Eingaben folgen der bestehenden NULL-Konvention;
soll ein leerer String getrennt gespeichert werden, ist dies ausdrücklich in
SQL möglich.

Veraltete Edits bei NULL-/Literal-Wechseln werden in beiden Richtungen erkannt.
Fehlende SQL-Parameter bei ausgelassenen, implizit nullable Feldern sind ebenfalls
behoben. Rohe SQL-/HTTP-/JSON-Zeilenformate bleiben unverändert. Audit-Detailtext
behält seine bisherige Darstellung; dies ist kein allgemeines Auditformat-Upgrade.

PostgreSQLs geschützte Edit-Abläufe benötigen weiterhin das eigene Paket für
Entscheidungstransaktionen/Runtime. Kein Guard wird umgangen; keine Migration,
neue Umgebungsvariable, Abhängigkeit oder externer Dienst wird eingeführt.
Siehe [Paketnachweise](release-readiness/0.9.9-null-edit-snapshots.de.md) und
[Releaseplan](release-plans/0.9.9.de.md).
