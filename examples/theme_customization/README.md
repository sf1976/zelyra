# Theme customization example / Theme-Anpassungsbeispiel

This example demonstrates the supported project-local customization model:

- `zelyra.theme.css` changes the shared palette, typography, and card radius;
- `Customer` uses a project-local layout and named slots;
- `Ticket` intentionally keeps the standard generated application shell;
- both resources retain compiler-checked fields, SQL, validation, CSRF, and
  authorization boundaries.

Dieses Beispiel zeigt das unterstützte projektlokale Anpassungsmodell:

- `zelyra.theme.css` ändert Palette, Typografie und Kartenrundung;
- `Customer` verwendet ein projektlokales Layout und benannte Slots;
- `Ticket` behält absichtlich den Standardrahmen der generierten Anwendung;
- beide Ressourcen behalten compilergeprüfte Felder, SQL, Validierung, CSRF und
  Autorisierungsgrenzen.

Validate the source / Quellcode prüfen:

```bash
zelyra check examples/theme_customization/main.zyl
```
