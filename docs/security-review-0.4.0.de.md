# Sicherheitsreview 0.4.0 — Arbeitsstand

**Status:** technische Bestandsaufnahme, kein externes Audit und keine
Releasefreigabe. Geprüft wurde der 0.4-Entwicklungszweig zum Zeitpunkt dieses
Commits. Dieses Dokument beschreibt implementierte Grenzen und bekannte Lücken;
es ersetzt weder Penetrationstest noch menschliche Abnahme.

## HTTP- und API-Grenzen

- Zelyras eingebauter HTTP-Server terminiert kein TLS. Für öffentliche oder
  LAN-erreichbare Dienste ist ein vertrauenswürdiger TLS-Proxy erforderlich;
  der Anwendungsport muss vor direktem Zugriff geschützt sein.
- API-Kontingente verwenden die TCP-Peer-IP. `X-Forwarded-For` und ähnliche
  Proxy-Header werden nicht als Clientidentität akzeptiert. Hinter einem Proxy
  werden daher zunächst alle Requests unter dessen Peer-IP gezählt.
- API- und Login-Limiter liegen im Prozessspeicher. Ein Neustart setzt den
  Zustand zurück; getrennte Instanzen koordinieren sich nicht. API-Buckets sind
  auf 4096 begrenzt und bei vollem Speicher für neue Schlüssel fail-closed.
- Request-IDs werden auf höchstens 128 ASCII-Zeichen aus Buchstaben, Ziffern,
  Punkt, Unterstrich und Bindestrich beschränkt. Ungültige Werte werden ersetzt.
  Die ID ist Korrelationsmetadatum, keine Authentifizierung und kein Geheimnis.
- Rate Limits erschöpfen mit `429` und `Retry-After`. Sie sind keine
  DDoS-Abwehr und ersetzen weder Edge-Limits noch persistente Kontrollen.

## Konten und Datenbankbetrieb

- Login-Fehlversuche sind pro Auth-Definition konfigurierbar und prozesslokal.
  Passwortzurücksetzung, Reset-Tokens und Reset-Limits sind nicht implementiert;
  es gibt dafür noch keine sichere E-Mail- oder Tokenzustellung.
- Sitzungsadministration ist teilweise umgesetzt. Gerätemetadaten und die
  vollständige Selbstverwaltung bleiben offen; die Kontolebenszyklus-Abnahme
  ist nicht bestanden.
- MariaDB-TLS prüft Zertifikatskette und Hostnamen gemäß dokumentiertem Modus.
  TLS endet weiterhin am jeweiligen Datenbankserver; App-HTTP bleibt separat.
  Windows-TLS braucht noch einen eigenen Matrixnachweis.
- Versionierte, reversible Schema-Migrationen, Drift-/Rollback-Nachweise und
  Wiederherstellung unter Abbruch sind nicht abgeschlossen. Vorhandene
  Backups bleiben Betreiberverantwortung.

## Lieferkette und verbleibende Gates

- Linux- und Windows-Release-Builds erzeugen SPDX-2.3-SBOMs aus dem
  aufgelösten Cargo-Graph und prüfen den Binary-SHA-256. Nicht als SPDX
  ausdrückbare Cargo-Lizenzfelder stehen als `NOASSERTION` samt Originalwert.
- Das SBOM ist kein Lizenzgutachten. Ein vollständiger Dependency-/Lizenzaudit,
  signierte Provenance und Prüfung jedes nativen Artefakts fehlen weiterhin.
- Granulare Effekte, vollständige HTTP-Response-Deadlines, unabhängiger
  Sicherheitsreview und menschliche Abnahme sind offen.

## Releaseentscheidung

Dieses Review schließt kein P0-Gate. Ein finales `v0.4.0` bleibt gesperrt, bis
der Releaseplan und das [Risikoregister](release-readiness/0.4.0-risikoregister.de.md)
für denselben Kandidaten vollständig belegt sind.
