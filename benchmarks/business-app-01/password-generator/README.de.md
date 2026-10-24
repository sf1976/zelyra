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

Die Startseite ist das Anwendungs-Dashboard mit Zelyra-AppShell und einer
Kachel pro Anwendung. Der Passwortgenerator liegt unter `/generator`; die
Warenwirtschaft ist als nächste Anwendung markiert und noch in Entwicklung.
Eine öffentliche Selbstregistrierung ist absichtlich nicht vorhanden. Ein berechtigter Administrator legt Konten unter
`/admin/users` an und verwaltet sie dort. Bei der Ersteinrichtung erhält die
Rolle `admin` die Rechte `identity.users.manage` und
`password_generator.use`.

Nach der Anmeldung führt **Profil** zu `/account/profile`. Dort kann ein
Benutzer den Anzeigenamen und die E-Mail-Adresse nach Bestätigung des aktuellen
Passworts ändern sowie sein Passwort ändern. Ein Passwortwechsel widerruft die
anderen Sitzungen. `/account/sessions` bietet die Verwaltung aktiver Sitzungen;
Konten, Rollen und Berechtigungen bleiben Administratoraufgaben. Die Anmeldung
ist ein zentraler Bestandteil des Identitätsmoduls und erscheint deshalb nicht
als eigene Anwendungskachel auf dem Dashboard.

Die Anwendung besteht aus drei lokalen Paketen:

- `zelyra_identity`: wiederverwendbare Zelyra-Anmeldung, dauerhafte Sitzungen,
  Rollen, Audit-Einträge, geschützte Benutzerverwaltung und das eigene
  Benutzerprofil.
- `password_app`: geschützte Seiten und der typisierte Formular-Handler.
- `password_engine`: die Passwortgenerierung.

Das Paketsystem ist noch experimentell. Das Beispiel zeigt die Integration
lokaler Pakete; es ist kein veröffentlichtes Registry-Paket und keine stabile
Plugin-ABI.

Das Projekttheme liegt als reguläres CSS in `zelyra.theme.css` und nutzt die
nativen Zelyra-UI-Tokens und -Komponenten. Es gestaltet eine gebrandete,
responsive Anmeldung und die passende Generatoroberfläche. Der
Umgebungshinweis ist in der Produktansicht ausgeblendet. Das Original-Logo liegt
unter `public/zelyra-logo.png`.

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
