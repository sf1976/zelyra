# Zelyra Passwortgenerator

Eine kleine, vollständig serverseitige Zelyra-Webanwendung. Formular,
Validierung, Zufallslogik und HTML-Ausgabe werden von Zelyra verarbeitet; die
Anwendung verwendet weder JavaScript noch Datenbank, externe Dienste oder
gespeicherte Passwortwerte.

## Starten

Die Projektdatei `.env` setzt `ZELYRA_LEVEL=work` und `ZELYRA_LANGUAGE=de`.
Starte die Anwendung aus diesem Verzeichnis:

```sh
/opt/zelyra/bin/zelyra check main.zyl
/opt/zelyra/bin/zelyra run tests.zyl
/opt/zelyra/bin/zelyra serve main.zyl
```

Danach die von Zelyra ausgegebene lokale Adresse öffnen. Der Generator nimmt
Längen von 8 bis 64 Zeichen an und bietet Kleinbuchstaben, Ziffern, Buchstaben,
Buchstaben mit Ziffern sowie alle Zeichenklassen an.

## Sicherheitsverhalten

- `random_int` verwendet Zelyras explizite `Random`-Capability und inklusive,
  sichere Ganzzahlbereiche.
- Die Ausgabe wird aus unabhängigen Zufallsziehungen erzeugt und nur dann
  akzeptiert, wenn alle im Profil ausgewählten Zeichengruppen vorkommen. Das
  ist Rejection Sampling; gültige Zeichenfolgen bleiben dadurch gleichverteilt.
- Das Standalone-Formular erzwingt CSRF-Schutz und serverseitige Feldvalidierung.
- Das Ergebnis wird HTML-escaped und mit `Cache-Control: no-store, private`
  ausgeliefert. Es wird weder in einer Datenbank gespeichert noch in eine URL
  geschrieben.
- Die Ausgabe kann im Browser markiert werden; es gibt keinen Clipboard-JavaScript-Code.

Das Projekt demonstriert außerdem Zelyras native Standalone-Form-Handler.
Diese allgemeine Sprach-/Web-Erweiterung erlaubt einem Formular, eine
typisierte Zelyra-Funktion aufzurufen und deren `String`-Ergebnis sicher
anzuzeigen.
