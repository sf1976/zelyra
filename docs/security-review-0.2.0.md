# Zelyra 0.2.0 security review / Sicherheitsreview

**Status:** Local quality checks, full branch CI, MariaDB compatibility tests,
database-backed acceptance, and Linux/Windows release validation pass. The
release remains experimental. This is not an external audit or penetration
test.

## English

### Scope and method

This review covers the 0.2.0 database-to-business-app path: generated MariaDB
projects, browser authentication and sessions, CRUD/forms/API authorization,
HTML rendering, SQL parameters, and schema planning/application. It consists
of source inspection and regression tests in the repository. It does not
cover a deployed production environment, third-party infrastructure, or
independent penetration testing.

### Findings and changes

1. **Cross-site request forgery and forged hosts — fixed in this branch.**
   The former CSRF token was process-scoped and was accepted without checking
   the browser request origin. A token visible to one visitor could therefore
   be replayed from a cross-origin form. Browser form writes now require a
   valid token and a matching `Origin` or `Referer` against `Host` and the
   effective request scheme. API requests carrying browser-origin data or a
   session cookie receive the same origin check, including `GET`; an exact
   configured CORS origin is the explicit cross-origin exception. A matching
   Origin/Host pair alone is not enough: every supplied `Host` is checked
   against `ZELYRA_ALLOWED_HOSTS`, which defaults to loopback and blocks
   DNS-rebinding requests to arbitrary names. Custom domains must be explicitly
   configured. The response `Referrer-Policy: same-origin` lets same-origin
   authenticated API GETs provide a `Referer` without sending it to another
   origin. Regression tests cover malformed and forwarded-HTTPS origins,
   attempted login-CSRF replay, forged hosts, allowed custom hosts, and invalid
   configuration.
2. **Session-cookie transport — improved, deployment-dependent.** Session
   cookies retain `HttpOnly` and `SameSite=Lax`. When a trusted TLS-terminating
   proxy supplies `X-Forwarded-Proto: https`, Zelyra now adds `Secure` to the
   session cookie and checks the external HTTPS origin. The built-in server
   itself remains HTTP-only; TLS and trusted-proxy enforcement are not
   implemented by Zelyra.
3. **SQL parameters and schema changes — existing controls reviewed.** Query
   values are bound separately from SQL text. Unsupported schema changes fail
   closed; destructive and review-required changes need explicit CLI approval,
   and preflights prevent unsafe required-column/nullability changes before
   plan SQL is applied.
4. **HTML output and credentials — existing controls reviewed.** Dynamic HTML
   values are escaped in the covered generated views. Database credentials
   are passed to the MariaDB client through its password environment variable;
   the release E2E checks assert that setup output does not disclose them.

### Residual risks and release boundary

- The CSRF token is still process-scoped, not individually stored per user
  session. Origin validation is therefore a required security control, not
  optional defense in depth.
- The host allowlist defaults to loopback only. Operators must explicitly add
  any public hostname and configure the reverse proxy safely; this setting
  does not provide TLS, proxy authentication, or network isolation.
- Zelyra does not terminate TLS. Do not expose the application port directly
  to an untrusted network. Use a trusted TLS proxy that preserves `Host`,
  overwrites `X-Forwarded-Proto`, and blocks direct access to the app port.
- Login throttling is in-memory and process-local; it is not a distributed
  rate limiter and resets when the process restarts.
- The application and release remain experimental and are not approved for
  production. This review is not a claim of mathematical proof, complete
  security, or external certification.

### Verification record

- Focused `zelyra-web` unit tests: 83 passed, including host allowlist,
  DNS-rebinding, same-origin, CORS, forwarded-HTTPS, and cookie cases.
- `zelyra-cli` tests: 105 unit tests and 45 machine-interface integration
  tests passed, including `.env` precedence and generated Compose settings.
- Release-packaging tests: 4 passed for byte-identical Linux/Windows archives,
  normalized archive timestamps/metadata, and checksums.
- Shell syntax checks passed for the MariaDB authentication, protected CRUD,
  and CRUD integration scripts after their Origin headers were added.
- Full workspace tests, formatting, locked workspace check, Clippy, handbook
  snippet validation, and release-package tests passed locally. The final
  branch's [full CI run](https://github.com/sf1976/zelyra/actions/runs/35504163055)
  passed, including the generated Docker application, protected CRUD/API, schema
  safety, SQLite/PostgreSQL integration, and MariaDB 10.11.19, 11.4.13, 11.8.9,
  and 12.3.3 compatibility jobs. The [release validation
  run](https://github.com/sf1976/zelyra/actions/runs/35504163084) passed for
  Linux and Windows, including byte-identical repeated release binaries and
  validated archives. These are internal automated checks, not an external
  audit.

## Deutsch

### Umfang und Methode

Dieses Review betrachtet den 0.2.0-Weg von der Datenbank zur
Businessanwendung: erzeugte MariaDB-Projekte, Browser-Authentifizierung und
Sessions, CRUD-/Formular-/API-Berechtigungen, HTML-Ausgabe, SQL-Parameter sowie
Schema-Planung und -Anwendung. Es besteht aus Quellcodeprüfung und
Regressionstests im Repository. Eine bereitgestellte Produktionsumgebung,
Drittanbieter-Infrastruktur und ein unabhängiger Penetrationstest sind nicht
enthalten.

### Feststellungen und Änderungen

1. **Cross-Site Request Forgery und gefälschte Hosts — in diesem Branch
   behoben.**
   Das bisherige CSRF-Token galt pro Prozess; die Anwendung prüfte den
   Browser-Origin nicht. Dadurch konnte ein Besucher ein sichtbares Token aus
   einem fremden Origin wiederverwenden. Schreibende Browserformulare benötigen
   jetzt ein gültiges Token sowie einen zu `Host` und dem effektiven Schema
   passenden `Origin`- oder `Referer`-Header. API-Anfragen mit Browser-Origin-
   Daten oder Session-Cookie durchlaufen dieselbe Origin-Prüfung, auch bei
   `GET`; nur ein exakt konfigurierter CORS-Origin ist die ausdrückliche
   Ausnahme. Ein passendes Origin-/Host-Paar genügt nicht: Jeder vorhandene
   `Host` wird zusätzlich mit `ZELYRA_ALLOWED_HOSTS` verglichen. Standardmäßig
   sind nur Loopback-Hosts freigegeben; beliebige DNS-Rebinding-Hosts werden
   blockiert. Eigene Domains müssen ausdrücklich konfiguriert werden.
   Die Antwort-Policy `Referrer-Policy: same-origin` erlaubt authentifizierten
   gleichursprünglichen API-GETs einen `Referer` als Nachweis, ohne Referrer an
   andere Origins zu senden.
   Regressionstests decken fehlerhafte und per HTTPS weitergeleitete Origins,
   Login-CSRF-Wiedergabe, manipulierte Hosts, freigegebene eigene Hosts und
   ungültige Konfiguration ab.
2. **Transport des Session-Cookies — verbessert, abhängig vom Deployment.**
   Session-Cookies behalten `HttpOnly` und `SameSite=Lax`. Meldet ein
   vertrauenswürdiger TLS-Proxy `X-Forwarded-Proto: https`, setzt Zelyra nun
   zusätzlich `Secure` und prüft den externen HTTPS-Origin. Der eingebaute
   Server unterstützt weiterhin nur HTTP; TLS und die Durchsetzung eines
   vertrauenswürdigen Proxys sind keine Zelyra-Funktion.
3. **SQL-Parameter und Schemaänderungen — bestehende Kontrollen geprüft.**
   Query-Werte werden getrennt vom SQL-Text gebunden. Nicht unterstützte
   Schemaänderungen werden fail-closed abgelehnt; destruktive und
   prüfpflichtige Änderungen benötigen eine ausdrückliche CLI-Freigabe.
   Vorprüfungen verhindern unsichere Pflichtspalten-/Nullbarkeitsänderungen,
   bevor SQL des Plans angewendet wird.
4. **HTML-Ausgabe und Zugangsdaten — bestehende Kontrollen geprüft.**
   Dynamische HTML-Werte werden in den geprüften generierten Views escaped.
   Datenbank-Zugangsdaten erhält der MariaDB-Client über seine
   Passwort-Umgebungsvariable; der Release-E2E-Test prüft, dass Setup-Ausgaben
   diese nicht offenlegen.

### Verbleibende Risiken und Release-Grenze

- Das CSRF-Token gilt weiterhin pro Prozess und wird nicht einzeln pro
  Benutzersession gespeichert. Die Origin-Prüfung ist daher notwendiger
  Sicherheitsbestandteil und nicht nur zusätzliche Absicherung.
- Die Host-Allowlist erlaubt standardmäßig nur Loopback-Hosts. Betreiber
  müssen öffentliche Hostnamen ausdrücklich ergänzen und den Reverse-Proxy
  sicher konfigurieren; die Einstellung bietet weder TLS noch
  Proxy-Authentifizierung oder Netzwerkisolation.
- Zelyra terminiert TLS nicht. Den Anwendungsport nicht direkt einem
  nicht vertrauenswürdigen Netzwerk aussetzen. Einen vertrauenswürdigen
  TLS-Proxy verwenden, der `Host` erhält, `X-Forwarded-Proto` überschreibt und
  direkten Zugriff auf den Anwendungsport verhindert.
- Die Login-Drosselung liegt nur im Prozessspeicher; sie ist kein verteilter
  Rate-Limiter und wird beim Neustart zurückgesetzt.
- Anwendung und Release bleiben experimentell und sind nicht für Produktion
  freigegeben. Dieses Review behauptet weder mathematisch bewiesene noch
  vollständige Sicherheit oder eine externe Zertifizierung.

### Prüfprotokoll

- Fokussierte `zelyra-web`-Unit-Tests: 83 bestanden, einschließlich Host-
  Allowlist, DNS-Rebinding, Same-Origin, CORS, HTTPS-Proxy und Cookie-Fällen.
- `zelyra-cli`-Tests: 105 Unit- und 45 Machine-Interface-Integrationstests
  bestanden, einschließlich `.env`-Vorrang und erzeugter Compose-Einstellungen.
- Release-Packagingtests: 4 bestanden; geprüft werden byte-identische Linux-
  und Windows-Archive, normalisierte Archivmetadaten und Prüfsummen.
- Shell-Syntaxprüfungen der MariaDB-Authentifizierungs-, Protected-CRUD- und
  CRUD-Integrationsskripte nach Ergänzung der Origin-Header bestanden.
- Workspace-Tests, Formatierung, gesperrte Workspace-Prüfung, Clippy,
  Handbuch-Codeblockprüfung und Release-Pakettests bestanden lokal. Die
  vollständige [Branch-CI](https://github.com/sf1976/zelyra/actions/runs/35504163055)
  bestand, einschließlich erzeugter Docker-Anwendung, geschütztem CRUD/API,
  Schema-Sicherheit, SQLite-/PostgreSQL-Integration sowie MariaDB-Kompatibilität
  für 10.11.19, 11.4.13, 11.8.9 und 12.3.3. Die
  [Release-Prüfung](https://github.com/sf1976/zelyra/actions/runs/35504163084)
  für Linux und Windows bestand ebenfalls, einschließlich byte-identischer
  wiederholter Release-Builds und geprüfter Archive. Dies sind interne
  automatisierte Prüfungen, kein externes Audit.
