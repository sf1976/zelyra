# Rechnungstutorial

`zelyra --tutorial invoice` erstellt das Projekt `invoice-tutorial`. Alternativ
kannst du ein anderes, noch nicht vorhandenes Verzeichnis angeben:
`zelyra --tutorial invoice ./meine-rechnungen`. Der Befehl schreibt nicht in
bereits vorhandene Verzeichnisse und prüft die erzeugte Anwendung mit dem
Compiler.

Das Tutorial erstellt ein lokales MariaDB-Projekt mit fünf verbundenen Teilen:

1. **Kunden** — Kundendatensätze unter `/customers` anlegen und suchen.
2. **Artikel** — wiederverwendbare Produkte, Preise und Aktivstatus unter
   `/items` pflegen.
3. **Dashboard** — unter `/` starten und die durchsuchbare Rechnungsübersicht
   unter `/views/invoicedashboard` öffnen.
4. **Rechnungen** — Entwurf erstellen, Kunden auswählen, Rechnungsnummer und
   Währung-unabhängigen Gesamtbetrag eintragen und Status unter `/invoices`
   verwalten.
5. **Rechnungspositionen** — unter `/invoice_lines` Rechnung und Artikel
   auswählen und Menge sowie Stückpreis eintragen.

Der Befehl gibt dieselben Anweisungen zum lokalen MariaDB-Start und Setup wie
`zelyra new --template mariadb-crud` aus. Wenn die Datenbank läuft und
`zelyra db setup main.zyl` erfolgreich war, starte die Anwendung:

```sh
zelyra serve main.zyl
```

Öffne `http://127.0.0.1:3000`, arbeite die fünf Schritte der Reihe nach durch
und verwende `zelyra editor .`, um dabei den Quelltext anzusehen. Das erzeugte
Programm liegt in `main.zyl`; Datenbankdeklaration und Verbindungseinstellungen
liegen wie üblich in den Projektdateien.

## Lernumfang

Dies ist ein kompaktes CRUD-Lernprojekt. Es speichert einen vom Benutzer
eingetragenen Gesamtbetrag und berechnet weder Steuern noch Positionssummen.
Gesetzeskonforme Rechnungsnummern, PDF-Erzeugung, E-Mail-Versand,
Zahlungsabwicklung und produktive Buchhaltungsregeln sind nicht enthalten.
Prüfe die lokalen Vorgaben und ergänze die erforderlichen Funktionen, bevor du
das Programm für echte Rechnungen verwendest.
