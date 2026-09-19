# Zelyra-Manifest: Digitale Souveränität und meisterhafte Baukunst

Zelyra steht für digitale Selbstbestimmung: Software soll den Menschen und
ihren Anwendungen dienen, nicht einer fremden Plattform. Dieses Manifest ist
ein verbindlicher Kompass für Architektur, Produktentscheidungen und
öffentliche Aussagen.

Die folgenden Werte beschreiben das Zielbild. Sie sind keine pauschale
Zertifizierung des aktuellen Entwicklungsstands. Implementierte Fähigkeiten,
offene Arbeit und belegte Garantien sind in der
[Roadmap](ROADMAP.de.md), der [Sprachspezifikation](specification.de.md) und
den Prüfergebnissen ausgewiesen.

> **Zelyra — Von der Datenbank zur Anwendung.**
>
> **Absicht beschreiben. Korrektheit beweisen.**

## Z — Zuverlässigkeit (Zero Compromise)

Zelyra setzt auf explizite Typen, Null-Sicherheit, geprüfte SQL- und
Schema-Beziehungen, Contracts, Capabilities und reproduzierbare Tests. Der
Verifier unterscheidet `PROVEN`, `RUNTIME_CHECK`, `UNPROVEN` und `FAILED`.
Ein Beweis gilt nur für seinen ausgewiesenen Geltungsbereich. Design by
Contract ist kein allgemeiner mathematischer Beweis, dass eine vollständige
Anwendung fehlerfrei ist; solche Garantien werden nicht behauptet.

## E — Eigenständigkeit (Souveränität)

Zelyra soll ohne verpflichtendes Cloud-Konto, KI-Anbieter oder geschlossene
Laufzeitplattform verwendbar und selbst betreibbar sein. Schnittstellen sollen
offen, externe Effekte sichtbar und Abhängigkeiten prüfbar sowie ersetzbar
bleiben. Eigenständigkeit bedeutet nicht, dass ein reales Projekt keinerlei
Bibliotheken, Netzwerk oder externe Dienste verwenden darf.

## L — Logische Klarheit

Quellcode soll ausdrücken, was er tut. Typen, Datenmodelle, Berechtigungen und
Seiteneffekte sollen sichtbar und prüfbar bleiben. Komfortable Abstraktionen
wie CRUD dürfen SQL, Geschäftslogik oder Sicherheitsgrenzen nicht verbergen
oder dem Entwickler die Kontrolle nehmen.

## Y — Yield (Ertragskraft und Präzision)

Eine fachliche Definition soll möglichst viele sichere Anwendungen speisen:
Schema, Typen, SQL-Prüfung, Validierung, Formulare, Views, CRUD und APIs.
Wiederverwendung soll Redundanz verringern, ohne Fachlogik in undurchsichtige
Magie zu verwandeln. Aussagen über Geschwindigkeit, Speicherverbrauch oder
Produktivität benötigen reproduzierbare Messungen.

## R — Robustheit (heute in Rust gebootstrapped)

Zelyra soll durch sorgfältige Implementierung, Sicherheitsgrenzen,
Regressionstests und messbare Ressourcenbudgets robust werden. Der aktuelle
Bootstrap-Compiler ist in Rust geschrieben. Rust ist jedoch nicht die
Zelyra-Sprache und keine erforderliche Endnutzer-Laufzeitabhängigkeit; der
langfristige Self-Hosting-Pfad ist geplant, nicht abgeschlossen.

## A — Autonomie (Datenhoheit)

Anwender sollen ihre Daten, ihre Infrastruktur und den Betriebsort bestimmen.
Lokale Ausführung, offene Datenformate und dokumentierte Backend-Grenzen sind
Leitlinien. MariaDB ist derzeit das primäre Backend; SQLite wird unterstützt,
und PostgreSQL besitzt Schema- und Planungsunterstützung, aber noch keine
vollständige Laufzeitparität.

Die Produktregel lautet: Zelyra darf keine Telemetrie erheben. Analytics,
Crash-Reporting und Hintergrund-„Phone-home“ sind ausgeschlossen.
Netzwerkzugriff für ausdrücklich gewählte Anwendungsfunktionen oder bewusst
gestartete Werkzeuge ist davon verschieden, muss aber sichtbar, begrenzt und
überprüfbar bleiben. Ein automatischer CI-Regressionstest gegen künftig
ungefragte Netzwerkaktivität ist noch geplant; siehe Roadmap.
Vom Anwender ausdrücklich gestartete Exporte eigener Anwendungs- oder Auditdaten
sind davon getrennte Datenhoheit, keine Nutzungs-Telemetrie.

## Verbindliche Entscheidungsfragen

Jede neue Funktion, Abhängigkeit, Voreinstellung und öffentliche Behauptung
muss geprüft werden:

1. Bleiben Code, Daten und Infrastruktur unter der Kontrolle des Anwenders?
2. Entsteht ein verpflichtender Cloud-, Anbieter- oder Netzwerkzwang?
3. Ist jeder externe Effekt explizit und sind Datenflüsse nachvollziehbar?
4. Sind Abhängigkeiten, Datenformate und Backend-Grenzen offen und wartbar?
5. Bleibt der einfache Standardweg erhalten und ist zusätzliche Komplexität
   optional?
6. Sind Korrektheits-, Sicherheits-, Leistungs- und Unabhängigkeitsversprechen
   auf einen getesteten Umfang begrenzt?
7. Sind Speicher- und Laufzeitkosten gemessen statt mit Superlativen beworben?

Wenn eine Antwort ein Ziel verfehlt, wird das offen benannt und als
Einschränkung oder Roadmap-Arbeit dokumentiert. Die
[Beitragsregeln](../AGENTS.md) machen diese Prüfung verbindlich.
