# Benchmark für KI-Autorenschaft in Zelyra

Status: nur Spezifikation; es werden keine Ergebnisse veröffentlicht.

Der Benchmark untersucht, wie gut Zelyra für KI-gestützte Entwicklung
datenbankgestützter Businessanwendungen geeignet ist. Er verwendet dieselben
versionierten Fixtures, Aufgabenstellungen und Sicherheitsbedingungen für alle
Vergleiche.

Aufgaben umfassen Adressverwaltung, Felder ergänzen oder verpflichtend machen,
SQL reparieren, Berechtigungen ergänzen, sichere Schemaänderungen, Formulare
erweitern und fehlerhafte Nullbehandlung korrigieren.

Zu erfassen sind Codezeilen, Ein-/Ausgabe-Tokens, Erstversuchskompilierung,
Korrekturschleifen, Zeit bis zu bestandenen Tests, Sicherheitsverstöße,
übersehene Abhängigkeiten, unsichere Datenbankänderungen und menschlicher
Prüfaufwand. Jede Ausführung dokumentiert Fixture-Commit, Modell-/Toolversion,
Prompts, Betriebssystem, Datenbankversion, Patches, Compiler- und Testausgaben
sowie menschliche Freigaben.

Ergebnisse bleiben leer, bis echte reproduzierbare Versuche stattgefunden
haben. Ein Secret-Leak, das Umgehen von Compiler- oder Testfehlern, eine
unerlaubte Capability oder eine nicht freigegebene destruktive Änderung ist ein
Sicherheitsfehler und kann nicht durch Produktivitätswerte ausgeglichen werden.
