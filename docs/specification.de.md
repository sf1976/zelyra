# Zelyra-0.1-Spezifikationsindex

Die Implementierungsspezifikation des Repositorys verteilt sich auf
Phasendokumente, das zweisprachige Handbuch und die Roadmap. Diese Datei legt
den querschnittlichen KI-nativen Vertrag fest.

## KI-native, KI-unabhängige Entwicklung

> Die KI schreibt. Zelyra prüft.

Menschen und KI-Systeme dürfen Code schreiben, aber keiner von beiden wird
automatisch als vertrauenswürdig behandelt. Compiler und Tests entscheiden,
ob Quellcode gültig ist. Die Sprache bleibt ohne Modell, Anbieter, Cloud oder
Netzwerk vollständig nutzbar.

KI-Schnittstellen sind offen, deterministisch, maschinenlesbar, versioniert,
lokal nutzbar und herstellerneutral. Die ersten Schnittstellen sind:

```text
zelyra check <file.zyl> --format=json
zelyra context <file.zyl> --format=json
```

Beide verwenden Maschinen-Schema-Version `1`. JSON geht ausschließlich auf
stdout, Logs auf stderr. Diagnosen besitzen stabile Codes und halb-offene
Source-Spans mit nullbasierten UTF-8-Byte-Offsets sowie einsbasierten
Zeilen-/Byte-Spalten. Ausgaben sind deterministisch und enthalten keine
Secrets, Zeitstempel, Zufalls-IDs, absoluten Pfade oder Live-Datenbankinhalte.

Der Compiler muss weiterhin Namen, Typen, Nullability, SQL, Schemata,
Formulare, Views, APIs, Berechtigungen, Contracts, Capabilities, Tests und
Freigaben destruktiver Änderungen prüfen. KI-Code darf Capabilities nicht
unbemerkt ergänzen, Berechtigungen nicht erweitern, kein destruktives SQL
ausführen, Prüfungen nicht abschwächen und keine Secrets ausgeben. Typed
Holes, Wirkungsanalyse, semantische Änderungen, feinere Effekte und Benchmarks
sind versionierte Roadmap-Arbeiten und noch nicht implementiert.
