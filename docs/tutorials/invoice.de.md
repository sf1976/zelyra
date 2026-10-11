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
   Status unter `/invoices` verwalten.
5. **Rechnungspositionen** — unter `/invoice_lines` Rechnung und Artikel
   auswählen und Menge sowie Netto-Stückpreis eintragen. Aus den Positionen
   berechnet die Rechnungsansicht den Nettobetrag, 19% Umsatzsteuer und den
   Bruttobetrag.

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

## Rechnung ansehen und als PDF speichern

Öffne `/invoices/RECHNUNGSNUMMER/print` mit der Rechnungsnummer. Die Seite fasst
alle Positionen zusammen und zeigt Netto, 19% Umsatzsteuer sowie Brutto. Über
**Rechnung drucken oder als PDF speichern** öffnet sich der Druckdialog des
Browsers. Wähle dort **Als PDF speichern**. Die Druckansicht blendet Navigation
und Druckschaltfläche aus.

Das Beispiel nimmt für alle Positionen den allgemeinen deutschen Steuersatz
von 19% an und speichert Artikelpreise als Nettopreise. Andere Steuersätze,
Steuerbefreiungen und Sonderfälle müssen in einer echten Anwendung korrekt
abgebildet werden. Die Vorlage ist ein Lernbeispiel und enthält keine
vollständige Prüfung aller gesetzlichen Rechnungsangaben, keine
GoBD-/Buchhaltungsfunktionen, keinen E-Mail-Versand und keine native
PDF-Erzeugung innerhalb von Zelyra. Vor produktivem Einsatz müssen die
geltenden Anforderungen ergänzt und geprüft werden.
