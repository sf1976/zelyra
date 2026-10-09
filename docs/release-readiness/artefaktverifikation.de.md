# Zelyra-Releaseartefakte verifizieren

Für ein veröffentlichtes Release das Archiv und seine `.sha256`-Datei aus
demselben GitHub-Release herunterladen. Vor dem Entpacken Prüfsumme und
GitHub-Release-Attestierung prüfen:

```sh
sha256sum --check zelyra-v0.4.0-x86_64-unknown-linux-gnu.tar.gz.sha256
gh attestation verify ./zelyra-v0.4.0-x86_64-unknown-linux-gnu.tar.gz --repo sf1976/zelyra
```

Für Windows das entsprechende Archiv `x86_64-pc-windows-msvc.zip` und dessen
Prüfsummendatei verwenden. `Get-FileHash -Algorithm SHA256 <Archiv>` lässt
sich mit dem Hex-Digest in der `.sha256`-Datei vergleichen;
`gh attestation verify` funktioniert mit beiden Archivformaten.

Der Releaseworkflow hängt an jedes getaggte Plattformarchiv eine GitHub
Artifact Attestation mit Build-Provenance und SPDX-SBOM. Der Prüf-Befehl
kontrolliert die signierte Build-Provenance des Archivs anhand der im
Repository gespeicherten Attestierung. Die SBOM wird zusätzlich als separates Releaseartefakt
veröffentlicht und kann direkt eingesehen werden. Diese Workflow-Konfiguration
braucht noch eine erfolgreiche Probe mit einem veröffentlichten Kandidaten,
bevor das 0.4.0-Gate abgeschlossen ist. Eine passende Prüfsumme allein belegt
nicht, wer ein Artefakt gebaut hat.
