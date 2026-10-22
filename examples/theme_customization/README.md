# Theme customization example / Theme-Anpassungsbeispiel

This example demonstrates the supported project-local customization model.
Use `zelyra.theme.css` to adapt the design through documented `--z-` tokens;
the shared framework CSS stays managed by Zelyra:

- `zelyra.theme.css` changes brand colors, navigation surfaces, page width,
  radii, shadows, and CRUD card styling;
- `Customer` uses a project-local layout and named slots;
- `Ticket` intentionally keeps the standard generated application shell;
- both resources retain compiler-checked fields, SQL, validation, CSRF, and
  authorization boundaries.

Dieses Beispiel zeigt das unterstützte projektlokale Anpassungsmodell. Mit
`zelyra.theme.css` passt du das Design über dokumentierte `--z-`-Tokens an;
das gemeinsame Framework-CSS bleibt von Zelyra verwaltet:

- `zelyra.theme.css` ändert Markenfarben, Navigation, Seitenbreite, Rundungen,
  Schatten und CRUD-Karten;
- `Customer` verwendet ein projektlokales Layout und benannte Slots;
- `Ticket` behält absichtlich den Standardrahmen der generierten Anwendung;
- beide Ressourcen behalten compilergeprüfte Felder, SQL, Validierung, CSRF und
  Autorisierungsgrenzen.

Validate the source / Quellcode prüfen:

```bash
zelyra check examples/theme_customization/main.zyl
```
