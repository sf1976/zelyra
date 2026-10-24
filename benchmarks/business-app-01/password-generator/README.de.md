# Zelyra-Passwortgenerator-Benchmark

Dies ist eine geschützte, serverseitig gerenderte Zelyra-Anwendung. Anmeldung,
Sitzungen, Rollen, Kontoverwaltung, Formular, Validierung und Zufallsgenerierung
werden aus nativen Zelyra-Deklarationen und wiederverwendbaren lokalen Paketen
zusammengesetzt.

## Starten

Die Projektdatei `.env` setzt `ZELYRA_LEVEL=work` und die deutsche UI
(`de`). Für Benutzer, Passwort-Hashes, Rollen, Audit-Einträge und Sitzungen
benötigt die Anwendung außerdem eine eigene MariaDB-Datenbank. `DATABASE_URL`
konfigurieren und anschließend ausführen:

```sh
zelyra package lock main.zyl
zelyra db setup main.zyl
zelyra check main.zyl
zelyra run tests.zyl
zelyra serve main.zyl
```

Unter `/login` meldest du dich an. Eine öffentliche Selbstregistrierung ist
absichtlich nicht vorhanden. Ein berechtigter Administrator legt Konten unter
`/admin/users` an und verwaltet sie dort. Bei der Ersteinrichtung erhält die
Rolle `admin` die Rechte `identity.users.manage` und
`password_generator.use`.

Die Anwendung besteht aus drei lokalen Paketen:

- `zelyra_identity`: wiederverwendbare Zelyra-Anmeldung, dauerhafte Sitzungen,
  Rollen, Audit-Einträge und geschützte Benutzerverwaltung.
- `password_app`: geschützte Seiten und der typisierte Formular-Handler.
- `password_engine`: die Passwortgenerierung.

Das Paketsystem ist noch experimentell. Das Beispiel zeigt die Integration
lokaler Pakete; es ist kein veröffentlichtes Registry-Paket und keine stabile
Plugin-ABI.

Das Projekttheme liegt als reguläres CSS in `zelyra.theme.css` und nutzt die
nativen Zelyra-UI-Tokens und -Komponenten. Es gestaltet eine gebrandete,
responsive Anmeldung und die passende Generatoroberfläche. Der
Umgebungshinweis ist in der Produktansicht ausgeblendet.

## Verhalten des Generators

Der Generator akzeptiert Längen von 8 bis 64 Zeichen und bietet Kleinbuchstaben,
Ziffern, Buchstaben, Buchstaben mit Ziffern und alle Zeichenklassen an.
`random_int` verwendet Zelyras explizite `Random`-Capability. Unabhängige sichere
Ziehungen mit Rejection Sampling stellen sicher, dass alle ausgewählten
Zeichenklassen enthalten sind, ohne die akzeptierten Ergebnisse zu verzerren.
Erzeugte Werte werden escaped, mit `Cache-Control: no-store, private`
ausgeliefert und weder gespeichert noch in eine URL geschrieben.

Die serverseitige Authentifizierungs- und Berechtigungsprüfung schützt den
Standalone-Handler, bevor seine Funktion ausgeführt wird. Das Formular erzwingt
zusätzlich CSRF-Schutz und typisierte serverseitige Validierung.
