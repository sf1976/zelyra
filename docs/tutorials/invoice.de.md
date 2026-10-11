# Rechnungstutorial

`zelyra --tutorial invoice` erstellt ein neues Projekt und richtet es sofort
ein: Zelyra schreibt die Anwendung, das lokale `.env`, Docker-Compose-Dateien
und das professionelle Theme, startet MariaDB und die Webanwendung und spielt
das Datenbankschema ein. Es ist weder ein manuelles Setzen von
`DATABASE_URL` noch ein separates `db setup` oder `serve` erforderlich.

Voraussetzung ist eine laufende Docker-Installation mit Docker Compose. Zelyra
installiert Docker nicht selbst. Fehlt Compose oder kann der Container nicht
starten, bleiben die Projektdateien erhalten; nach Behebung des Docker-Problems
startet `cd invoice-tutorial && zelyra setup --all` Datenbank, Anwendung und
Schema gemeinsam. Bei einem anderen Zielordner verwende den entsprechenden
Pfad, zum Beispiel `zelyra --tutorial invoice ./meine-rechnungen`.
Für reine Projektgenerierung ohne Docker-Start kannst du `--no-start`
anhängen. Auch dann erzeugt Zelyra bereits eine geschützte lokale `.env` mit
`ZELYRA_DATABASE_MAIN_URL` und dem kompatiblen `DATABASE_URL`-Fallback; die
Zugangsdaten werden nicht in der Ausgabe gezeigt. Die Datenbank und Anwendung
starten erst mit `zelyra setup --all`. Die Ausgabe nennt den Startbefehl und
die vorgesehene Browseradresse. Auch `zelyra new ... --mariadb`
zeigt schon beim Anlegen die Adresse an, die nach dem Start erreichbar ist.
Wenn der Schema-Schritt nach dem Start fehlschlägt, bleibt die Adresse in der
Fehlermeldung sichtbar.

Die fertige Anwendung läuft auf der in der Befehlsausgabe genannten lokalen
Adresse (standardmäßig `http://127.0.0.1:3000`). Sie enthält fünf verbundene
Bereiche:

1. **Kunden** — Kundendatensätze unter `/customers` anlegen und suchen.
2. **Artikel** — wiederverwendbare Produkte, Preise und Aktivstatus unter
   `/items` pflegen.
3. **Dashboard** — unter `/` starten und die durchsuchbare Rechnungsübersicht
   unter `/views/invoicedashboard` öffnen.
4. **Rechnungen** — Entwurf erstellen, Kunden auswählen, Rechnungsnummer und
   Status unter `/invoices` verwalten.
5. **Rechnungspositionen** — unter `/invoice_lines` Rechnung und Artikel
   auswählen und Menge sowie Netto-Stückpreis eintragen. Aus den Positionen
   berechnet die Rechnungsansicht Nettobetrag, 19% Umsatzsteuer und Brutto.

## Projektaufbau: eine Aufgabe pro Datei

Das Tutorial ist ein echtes Mehrdateienprojekt. `main.zyl` ist der Einstieg und
bindet die Module mit `import` ein. Die fachlichen Bereiche liegen getrennt:

~~~text
invoice-tutorial/
├── main.zyl                 Einstieg und Startseite
├── zelyra.toml              Projekt- und Laufzeitkonfiguration
├── zelyra.theme.css         Design-Anpassungen
└── src/
    ├── database.zyl         gemeinsame MariaDB-Deklaration
    ├── shell.zyl            wiederverwendbares Seitenlayout
    ├── customers.zyl        Kundenmodell, Tabelle und CRUD-Seiten
    ├── items.zyl            Artikelmodell, Tabelle und CRUD-Seiten
    ├── invoices.zyl         Rechnungen, Positionen und Druckansicht
    └── dashboard.zyl        Summenabfrage und Rechnungsübersicht
~~~

So kann ein Einsteiger einzelne Teile lesen oder ändern, während Compiler und
Runtime das gesamte importierte Projekt gemeinsam prüfen. `zelyra check
main.zyl` und `zelyra routes main.zyl` starten im Projektordner am Einstieg;
Studio zeigt dieselben Quelldateien im Projektbaum.

Das responsiv gestaltete Theme umfasst Navigation, Formulare, Schaltflächen,
Tabellen und eine eigene Druckansicht. Mit `zelyra editor invoice-tutorial`
öffnest du Studio, um das Programm und seine Datenbankdeklarationen anzusehen
und zu bearbeiten.

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
