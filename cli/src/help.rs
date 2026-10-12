use std::env;

pub(super) fn print_if_requested(command: &str, arguments: &[String]) -> bool {
    let Some(last) = arguments.last() else {
        return false;
    };
    if !matches!(last.as_str(), "--help" | "-h") {
        return false;
    }
    let preceding = &arguments[..arguments.len() - 1];
    let german = env::var("ZELYRA_LANGUAGE")
        .map(|language| language.eq_ignore_ascii_case("de"))
        .unwrap_or(true);
    let help = help_text(command, preceding, german);
    let Some(help) = help else {
        return false;
    };
    println!("{help}");
    true
}

fn help_text(command: &str, arguments: &[String], german: bool) -> Option<&'static str> {
    let first = arguments.first().map(String::as_str).unwrap_or_default();
    let second = arguments.get(1).map(String::as_str).unwrap_or_default();
    let key = match (command, first, second) {
        ("migrate", "", _) => "migrate",
        ("--tutorial", _, _) => "tutorial",
        ("db", "plan", _) => "db-plan",
        ("db", "apply", _) => "db-apply",
        ("db", "map", _) => "db-map",
        ("db", "inspect", _) => "db-inspect",
        ("db", "create", _) => "db-create",
        ("db", "setup", _) => "db-setup",
        ("db", "bootstrap", _) => "db-bootstrap",
        ("db", "history", _) => "db-history",
        ("auth", "hash-password", _) => "auth-hash-password",
        ("auth", "role", _) => "auth-role",
        ("auth", "role-permission", _) => "auth-role-permission",
        ("audit", "inspect", _) => "audit-inspect",
        ("audit", "export", _) => "audit-export",
        ("audit", "verify", _) => "audit-verify",
        ("audit", "prune", _) => "audit-prune",
        ("form", "validate", _) => "form-validate",
        ("module", "plan", _) => "module-plan",
        ("module", "bundle", _) => "module-bundle",
        ("migrate", "laravel", _) => "migrate-laravel",
        ("text", "edit", _) => "text-edit",
        ("outbox", "setup", _) => "outbox-setup",
        ("outbox", "list", _) => "outbox-list",
        ("outbox", "requeue", _) => "outbox-requeue",
        ("outbox", "run", _) => "outbox-run",
        ("setup", _, _) if arguments.iter().any(|argument| argument == "--web") => "setup-web",
        ("update", _, _) => "update",
        ("new", _, _) => "new",
        ("init", _, _) => "init",
        ("setup", _, _) => "setup",
        ("check", _, _) => "check",
        ("fmt", _, _) => "fmt",
        ("impact", _, _) => "impact",
        ("edit", _, _) => "edit",
        ("context", _, _) => "context",
        ("config", _, _) => "config",
        ("build", _, _) => "build",
        ("run", _, _) => "run",
        ("serve", _, _) => "serve",
        ("routes", _, _) => "routes",
        ("editor", _, _) => "editor",
        ("doctor", _, _) => "doctor",
        ("verify", _, _) => "verify",
        ("doc", _, _) => "doc",
        ("db", _, _) => "db",
        ("auth", _, _) => "auth",
        ("audit", _, _) => "audit",
        ("form", _, _) => "form",
        ("module", _, _) => "module",
        ("outbox", _, _) => "outbox",
        ("text", _, _) => "text",
        _ => return None,
    };
    Some(if german {
        german_help(key)
    } else {
        english_help(key)
    })
}

fn german_help(key: &str) -> &'static str {
    match key {
        "db" => "Befehle: zelyra db <create|setup|bootstrap|inspect|map|plan|apply|history> <datei.zyl>

Verwaltet das in Zelyra deklarierte Datenbankschema. `plan` zeigt Änderungen zuerst nur an; `apply` wendet einen geprüften Plan an.

Datenbankverbindung:
  ZELYRA_DATABASE_<NAME>_URL  Verbindung einer benannten Projektdatenbank, zum Beispiel ZELYRA_DATABASE_MAIN_URL
  DATABASE_URL               Rückfallwert für Projekte mit nur einer Datenbank

Beispiele:
  zelyra db plan main.zyl
  zelyra db plan main.zyl --format json
  zelyra db apply main.zyl

Die verfügbaren Operationen und ihre Grenzen:
  create     Geprüftes CREATE-SQL ausgeben; verbindet sich nicht mit der Datenbank
  setup      Datenbank bei MariaDB/MySQL anlegen und deklariertes Schema einrichten; SQLite initialisieren
  bootstrap  Alias für `setup`
  inspect    Vorhandene Datenbankstruktur lesen
  map        Deklarierte und vorhandene Tabellen vergleichen
  plan       Schemaänderungen vorschauen, ohne die Datenbank zu ändern
  apply      Geprüften Schemaänderungsplan anwenden
  history    Journalisierte MariaDB-/MySQL-Migrationen anzeigen

Eine benannte Verbindung hat Vorrang vor `DATABASE_URL`. Zugangsdaten werden nicht in der Hilfe ausgegeben.",
        "fmt" => "Befehl: zelyra fmt <datei.zyl> [--check]\n\nFormatiert eine Zelyra-Datei kanonisch.\n  --check  Prüft, ob die Datei bereits formatiert ist; schreibt nichts.\n\nBeispiele:\n  zelyra fmt main.zyl\n  zelyra fmt src/rechnungen.zyl --check\n\nDatenbank: nicht erforderlich.",
        "init" => "Befehl: zelyra init [ordner] [Optionen]\n\nInitialisiert ein Zelyra-Projekt im angegebenen Ordner; ohne Ordner wird das aktuelle Verzeichnis verwendet.\n\nOptionen:\n  --mariadb                         MariaDB-Projekt mit lokalem Docker-Setup erstellen\n  --template minimal|mariadb-crud|mariadb-auth|mariadb-business\n                                    Projektvorlage auswählen\n  --web-port <port>                 Port im Webcontainer (Standard: 3000)\n  --host-port <port>                Lokaler Port der Webanwendung\n  --db-host-port <port>             Lokaler MariaDB-Port\n\nBeispiele:\n  zelyra init\n  zelyra init ./mein-projekt\n  zelyra init ./rechnung --mariadb --host-port 8080\n\nOhne `--mariadb` wird keine Datenbank eingerichtet.",
        "new" => "Befehl: zelyra new <ordner> [Optionen]\n\nErstellt ein neues Zelyra-Projekt aus einer Vorlage.\n\nOptionen:\n  --mariadb                         MariaDB-Projekt mit lokalem Docker-Setup erstellen\n  --template minimal|mariadb-crud|mariadb-auth|mariadb-business\n                                    Projektvorlage auswählen\n  --web-port <port>                 Port im Webcontainer (Standard: 3000)\n  --host-port <port>                Lokaler Port der Webanwendung\n  --db-host-port <port>             Lokaler MariaDB-Port\n\nBeispiele:\n  zelyra new ./kunden --template mariadb-crud\n  zelyra new ./rechnung --mariadb --host-port 8080\n\nOhne `--mariadb` wird keine Datenbank eingerichtet.",
        "setup" => "Befehl: zelyra setup [ordner] [Aktion]\n\nBereitet ein MariaDB-Projekt vor. Ohne Aktion prüft und erstellt der Befehl die geschützte lokale `.env`.\n\nAktionen und Optionen:\n  --database       MariaDB und Webanwendung starten\n  --schema         Projektschema anwenden\n  --all            Stack starten und Schema anwenden\n  --host-port <p>  Lokalen Webport festlegen\n  --db-host-port <p> Lokalen Datenbankport festlegen\n  --web [--port p] Lokale Setup-Oberfläche starten\n\nBeispiele:\n  zelyra setup\n  zelyra setup ./rechnung --all\n  zelyra setup ./rechnung --web\n\n`--all` benötigt Docker Compose und die Projektkonfiguration. Fortschritt und Webadresse werden ausgegeben. Änderungen, die eine ausdrückliche Freigabe brauchen, werden abgelehnt; prüfe `zelyra db plan main.zyl` und bestätige sie danach gezielt mit `zelyra db apply main.zyl --allow-risky`.",
        "setup-web" => "Befehl: zelyra setup [ordner] --web [--port <port>]\n\nStartet die lokale Setup-Oberfläche, über die Datenbank und Schema vorbereitet werden.\n  --port <port>  Lokaler HTTP-Port (Standard: 3030)\n\nBeispiel:\n  zelyra setup ./rechnung --web --port 3031\n\nDer Befehl zeigt eine lokale URL mit Sitzungstoken. Nur lokale Verbindungen sind erlaubt.",
        "tutorial" => "Befehl: zelyra --tutorial invoice [ordner] [--no-start]\n\nErstellt die geführte Rechnungsanwendung mit Kunden, Artikeln, Dashboard, Rechnungen, 19% Umsatzsteuer und Druckansicht. Standardmäßig startet Zelyra MariaDB und die Webanwendung, richtet das Schema ein und öffnet anschließend den Standardbrowser.\n\nOptionen:\n  [ordner]     Zielordner (Standard: ./invoice-tutorial)\n  --no-start   Dateien erstellen, Docker nicht starten und keinen Browser öffnen\n\nBeispiele:\n  zelyra --tutorial invoice\n  zelyra --tutorial invoice ./meine-rechnungen --no-start\n\nDocker Compose wird für den automatischen Start benötigt. Wenn kein grafischer Browser verfügbar ist, gibt Zelyra die Anwendungsadresse aus.",
        "check" => "Befehl: zelyra check <datei.zyl> [--format human|json]\n\nPrüft Syntax, Typen und bekannte Projektabhängigkeiten und gibt Diagnosen aus.\n  --format human  Lesbare Diagnosen (Standard)\n  --format json   Maschinenlesbare Diagnosen\n\nBeispiele:\n  zelyra check main.zyl\n  zelyra check src/kunden.zyl --format json\n\nDatenbankzugriff wird nicht benötigt.",
        "routes" => "Befehl: zelyra routes <entry.zyl> [--format human|json]\n\nListet deklarierte Seiten, APIs und erzeugte Routen für CRUD, Formulare, Authentifizierung und Tabellenansichten auf. Doppelte oder überlappende Routen werden als Fehler gemeldet.\n\nBeispiele:\n  zelyra routes main.zyl\n  zelyra routes main.zyl --format json\n\nDie Analyse verwendet den Quelltext; eine laufende Anwendung oder Datenbank ist nicht erforderlich.",
        "migrate" => "Befehle: zelyra migrate laravel <projekt> [--report <pfad>] [--output <ziel>]\n\nDer Laravel-Migrator erstellt einen schreibgeschützten Analysebericht oder auf Wunsch ein getrenntes Zelyra-Gerüst. Er führt weder PHP noch Artisan aus und liest keine `.env`-Dateien.\n\nDetails:\n  zelyra migrate laravel --help",
        "migrate-laravel" => "Befehl: zelyra migrate laravel <projekt> [--report <pfad>] [--output <ziel>] [--format markdown|json]\n\nAnalysiert Laravel-Quelltext statisch und erstellt einen Bericht außerhalb des Quellprojekts. PHP, Artisan, Datenbank und `.env`-Dateien werden nicht ausgeführt, verbunden oder gelesen.\n\nOptionen:\n  --report <pfad>             Neuer, noch nicht existierender Berichtspfad außerhalb des Projekts\n  --output <ziel>             Neues, separates Zelyra-Projekt ohne kopierte Laravel-Logik\n  --format markdown|json      Ausgabeformat; Standard folgt der Dateiendung\n\nBeispiele:\n  zelyra migrate laravel ./alte-app --report ./reports/alte-app.md --output ./neue-app\n  zelyra migrate laravel ./alte-app --report ./reports/alte-app.md\n  zelyra migrate laravel ./alte-app --report ./reports/alte-app.json --format json\n\nErkannte Muster sind kein Beleg für Geschäftslogik- oder Sicherheitsparität. Unklare Stellen bleiben für die manuelle Prüfung markiert.",
        "editor" => "Befehl: zelyra editor [ordner] [--port <port>] [--no-open]\n\nStartet Zelyra Studio lokal und öffnet den Standardbrowser. Ohne Ordner wird das aktuelle Verzeichnis verwendet.\n\nOptionen:\n  --port <port>  Lokaler Studio-Port (Standard: 4177)\n  --no-open      Browser nicht automatisch öffnen\n\nBeispiele:\n  zelyra editor\n  zelyra editor ./rechnung --port 4180\n\nStudio bietet Bearbeitung, Compilerdiagnosen, Konsolenausführung und lokale Webvorschau. Eine Webanwendung benötigt ihre erreichbare Datenbank.",
        "run" => "Befehl: zelyra run <datei.zyl> [--max-steps <n>]\n\nFührt ein Zelyra-Konsolenprogramm aus. Datenbank-, Netzwerk- und Prozesseffekte richten sich nach Projektkonfiguration und Freigaben. Mit `--max-steps` aktivierst du ein Schrittbudget für begrenzte Läufe; ohne Option bleiben Dauerläufe erlaubt.\n\nBeispiel:\n  zelyra run main.zyl --max-steps 100000\n\nFür Webanwendungen verwende `zelyra serve`; Studio kann eine lokale Webvorschau starten.",
        "serve" => "Befehl: zelyra serve <datei.zyl> [adresse]\n\nStartet eine deklarative Zelyra-Webanwendung.\n  adresse  Bindeadresse (Standard: 127.0.0.1:3000)\n\nBeispiele:\n  zelyra serve main.zyl\n  zelyra serve main.zyl 127.0.0.1:8080\n\nDie benötigte Projektdatenbank muss erreichbar und in der Prozessumgebung konfiguriert sein.",
        "db-plan" => "Befehl: zelyra db plan <datei.zyl> [--format text|json]\n\nVergleicht die deklarierte Struktur mit der konfigurierten Datenbank und zeigt einen sicheren Änderungsplan. Der Plan ändert keine Datenbank.\n\nBeispiele:\n  zelyra db plan main.zyl\n  zelyra db plan main.zyl --format json\n\nBenötigt die konfigurierte Projektdatenbank.",
        "db-apply" => "Befehl: zelyra db apply <datei.zyl> [--plan-id <sha256:…>] [--allow-risky]\n\nWendet einen Schemaänderungsplan an. Mit `--plan-id` muss der aktuelle Plan exakt der zuvor geprüften Kennung entsprechen.\n\nBeispiele:\n  zelyra db plan main.zyl --format json\n  zelyra db apply main.zyl --plan-id sha256:<kennung>\n\nRiskante Änderungen benötigen die ausdrückliche Option `--allow-risky`; destructive Änderungen behalten eine eigene Bestätigung. Benötigt die konfigurierte Projektdatenbank.",
        "db-map" => "Befehl: zelyra db map <datei.zyl> [--format text|json]\n\nZeigt Tabellen gruppiert nach Quellmodul, deklarierte Beziehungen und nicht zugeordnete Live-Tabellen. Die Zuordnung zu Modulen ist informativ.\n\nBeispiel:\n  zelyra db map main.zyl --format json\n\nBenötigt die konfigurierte Projektdatenbank.",
        "db-inspect" => "Befehl: zelyra db inspect <datei.zyl>\n\nLiest den aktuellen Zustand der konfigurierten Projektdatenbank aus.\n\nBeispiel:\n  zelyra db inspect main.zyl\n\nÄndert keine Datenbankobjekte.",
        "db-create" => "Befehl: zelyra db create <datei.zyl>\n\nGibt das geprüfte CREATE-SQL für die deklarierten Tabellen aus. Der Befehl verbindet sich nicht mit einer Datenbank und legt weder die Datenbank noch Tabellen an.\n\nBeispiel:\n  zelyra db create main.zyl\n  zelyra db create main.zyl > schema.sql\n\nZum Anlegen der MariaDB- oder MySQL-Datenbank und Einrichten des Schemas: `zelyra db setup main.zyl`. Für ein lokales MariaDB-Docker-Projekt: `zelyra setup . --all`.",
        "db-setup" | "db-bootstrap" => "Befehl: zelyra db setup|bootstrap <datei.zyl>\n\nMariaDB und MySQL: legt die deklarierte Datenbank bei Bedarf an und richtet das Schema ein. SQLite: initialisiert die konfigurierte Datei und Tabellen. PostgreSQL: `setup` wird nicht unterstützt; verwende `zelyra db apply`. Prüfe vorher mit `zelyra db plan`, welche Änderungen anstehen.\n\nBeispiele:\n  zelyra db plan main.zyl\n  zelyra db setup main.zyl\n\nBenötigt die konfigurierte Projektdatenbank.",
        "db-history" => "Befehl: zelyra db history <datei.zyl> [--format text|json]\n\nZeigt den Fortschritt journalisierter MariaDB- und MySQL-Schemamigrationen.\n\nBeispiel:\n  zelyra db history main.zyl\n\nBenötigt die konfigurierte MariaDB- oder MySQL-Projektdatenbank.",
        _ => common_german_help(key),
    }
}

fn common_german_help(key: &str) -> &'static str {
    match key {
        "impact" => "Befehl: zelyra impact <datei.zyl> [Optionen]\n\nZeigt bekannte Abhängigkeiten und Auswirkungen einer Quelldatei.\n  --format human|json  Ausgabeformat\n  --symbol <art:name>  Auf ein bekanntes Symbol begrenzen\n\nBeispiel: zelyra impact main.zyl --format json\n\nDatenbankzugriff wird nicht benötigt.",
        "context" => "Befehl: zelyra context <datei.zyl> [--format human|json]\n\nGibt strukturierten Compilerkontext für Studio und Werkzeuge aus.\n\nBeispiel: zelyra context main.zyl --format json\n\nDatenbankzugriff wird nicht benötigt.",
        "config" => "Befehl: zelyra config <datei.zyl> [--format human|json]\n\nZeigt die aufgelöste Projektkonfiguration und wirksame Fähigkeiten.\n\nBeispiel: zelyra config main.zyl\n\nGeheimnisse werden nicht als Konfigurationshilfe benötigt.",
        "build" => "Befehl: zelyra build <datei.zyl>\n\nPrüft und baut ein Zelyra-Projekt für die aktuell unterstützte Ausführung.\n\nBeispiel: zelyra build main.zyl\n\nDatenbankzugriff wird nicht benötigt.",
        "edit" => "Befehl: zelyra edit --format=json [--apply] <änderung.json>\n\nPrüft einen strukturierten Quelltextänderungsplan. Ohne `--apply` wird nur eine Vorschau erzeugt.\n\nBeispiel: zelyra edit --format=json change.json\n\nÄnderungen werden nur nach ausdrücklicher Anwendung geschrieben.",
        "doctor" => "Befehl: zelyra doctor [datei.zyl] [--port <port>] [--json]\n\nPrüft lokale Voraussetzungen, Projektkonfiguration und optionale Web-Erreichbarkeit.\n\nBeispiel: zelyra doctor main.zyl --json\n\nFür generierte MariaDB-Projekte unterstützt `--env-file <pfad>` die explizite `.env`-Angabe.",
        "verify" => "Befehl: zelyra verify <datei.zyl> [--json]\n\nFührt die verfügbaren statischen Verifikationsbedingungen aus und meldet ihren Status. Das Ergebnis gilt nur für den dokumentierten Umfang.\n\nBeispiel: zelyra verify main.zyl --json",
        "doc" => "Befehl: zelyra doc <datei.zyl> [--openapi|--typescript]\n\nErzeugt Dokumentation beziehungsweise unterstützte API-Ausgaben aus einem Zelyra-Projekt.\n\nBeispiele:\n  zelyra doc main.zyl --openapi\n  zelyra doc main.zyl --typescript",
        "auth" => "Befehle: zelyra auth <aktion>\n\n  hash-password [--stdin]                         Passwort sicher hashen\n  role grant|revoke <datei.zyl> <user-id> <rolle> Rolle vergeben oder entziehen\n  role-permission grant|revoke <datei.zyl> <rolle> <recht> Recht zuweisen oder entziehen\n\nBeispiele:\n  zelyra auth hash-password\n  printf '%s' \"$PASSWORT\" | zelyra auth hash-password --stdin\n  zelyra auth role grant main.zyl 42 editor\n\nRollenänderungen benötigen eine konfigurierte MariaDB und eine Auth-Deklaration.",
        "auth-hash-password" => "Befehl: zelyra auth hash-password [--stdin]\n\nErzeugt einen sicheren Passwort-Hash. Interaktive Eingabe wird nicht angezeigt.\n  --stdin  Passwort aus der Standardeingabe lesen (für Automatisierung)\n\nBeispiele:\n  zelyra auth hash-password\n  printf '%s' \"$PASSWORT\" | zelyra auth hash-password --stdin",
        "auth-role" => "Befehle:\n  zelyra auth role grant <datei.zyl> <user-id> <rolle>\n  zelyra auth role revoke <datei.zyl> <user-id> <rolle>\n\nVergibt oder entzieht eine Rolle. Benötigt Auth-Rollentabellen und eine MariaDB-URL.\n\nBeispiel: zelyra auth role grant main.zyl 42 editor",
        "auth-role-permission" => "Befehle:\n  zelyra auth role-permission grant <datei.zyl> <rolle> <recht>\n  zelyra auth role-permission revoke <datei.zyl> <rolle> <recht>\n\nWeist einer Rolle ein Recht zu oder entzieht es. Benötigt Auth-Rollentabellen und eine MariaDB-URL.\n\nBeispiel: zelyra auth role-permission grant main.zyl editor invoices.read",
        "audit" => "Befehle: zelyra audit inspect|export|verify|prune <datei.zyl>\n\n  inspect [--limit n]                  Ereignisse lesbar anzeigen\n  export [--limit n] [--format json|csv] Ereignisse exportieren\n  verify                               Verkettete Audit-Prüfsumme validieren\n  prune --before <zeit> [--confirm]    Alte Ereignisse nach Bestätigung löschen\n\nBeispiele:\n  zelyra audit inspect main.zyl\n  zelyra audit export main.zyl --format csv",
        "audit-inspect" => "Befehl: zelyra audit inspect <datei.zyl> [--limit <n>]\n\nZeigt die neuesten Audit-Ereignisse an.\n\nBeispiel: zelyra audit inspect main.zyl --limit 50",
        "audit-export" => "Befehl: zelyra audit export <datei.zyl> [--limit <n>] [--format json|csv]\n\nExportiert Audit-Ereignisse als JSON oder CSV.\n\nBeispiel: zelyra audit export main.zyl --format csv",
        "audit-verify" => "Befehl: zelyra audit verify <datei.zyl>\n\nPrüft die Verkettung und Integrität der gespeicherten Audit-Ereignisse.\n\nBeispiel: zelyra audit verify main.zyl",
        "audit-prune" => "Befehl: zelyra audit prune <datei.zyl> --before <zeit> [--confirm]\n\nEntfernt alte Audit-Ereignisse. Ohne `--confirm` wird nichts gelöscht.\n\nBeispiel: zelyra audit prune main.zyl --before 2025-01-01T00:00:00Z --confirm",
        "form" => "Befehl: zelyra form validate <datei.zyl> <FormName> [feld=wert ...]\n\nValidiert Formulardaten mit den Regeln der Formdeklaration. Decimal erlaubt höchstens 9 Nachkommastellen; Money höchstens 15 Vor- und 4 Nachkommastellen. Nicht-endliche Werte und stille Rundung werden abgewiesen.\n\nBeispiel: zelyra form validate main.zyl Customer name=Anna email=anna@example.org",
        "form-validate" => "Befehl: zelyra form validate <datei.zyl> <FormName> [feld=wert ...]\n\nValidiert Formulardaten mit den Regeln der Formdeklaration. Decimal erlaubt höchstens 9 Nachkommastellen; Money höchstens 15 Vor- und 4 Nachkommastellen. Nicht-endliche Werte und stille Rundung werden abgewiesen.\n\nBeispiel: zelyra form validate main.zyl Customer name=Anna email=anna@example.org",
        "module" => "Befehle: zelyra module plan|bundle <entry.zyl> <modul.zyl|resource-id>\n\n`plan` zeigt Abhängigkeiten. `bundle` schreibt ein geprüftes Modulbündel in ein Zielverzeichnis.\n\nDetails: zelyra module plan --help und zelyra module bundle --help",
        "module-plan" => "Befehl: zelyra module plan <entry.zyl> <modul.zyl|resource-id>\n\nZeigt bekannte Quell-, Tabellen-, Datenbank- und Ressourcendependencies eines Moduls.\n\nBeispiel: zelyra module plan main.zyl src/invoices.zyl",
        "module-bundle" => "Befehl: zelyra module bundle <entry.zyl> <modul.zyl|resource-id> --output <ordner> [Optionen]\n\nErzeugt ein isoliertes Modulbündel.\n  --dry-run                 Nur maschinenlesbaren Dateiplan anzeigen\n  --docker                  Docker-Artefakte erzeugen\n  --compiler-ref <commit>   40-stelligen Compiler-Commit für Docker festlegen\n\nBeispiel: zelyra module bundle main.zyl src/invoices.zyl --output dist/invoices --dry-run",
        "outbox" => "Befehle: zelyra outbox setup|list|requeue|run\n\nZeigt oder verarbeitet transaktionale Outbox-Ereignisse. `run` benötigt mindestens einen Handler.\n\nDetails: zelyra outbox setup --help, zelyra outbox list --help, zelyra outbox requeue --help, zelyra outbox run --help",
        "outbox-setup" => "Befehl: zelyra outbox setup [datei.zyl]\n\nRichtet die Tabellen der transaktionalen Outbox ein.\n\nBeispiel: zelyra outbox setup main.zyl",
        "outbox-list" => "Befehl: zelyra outbox list [datei.zyl] [--limit <1..200>] [--format text|json]\n\nListet wartende, gesperrte und fehlgeschlagene Ereignisse auf.\n\nBeispiel: zelyra outbox list main.zyl --limit 50 --format json",
        "outbox-requeue" => "Befehl: zelyra outbox requeue <event-id> [datei.zyl]\n\nSetzt ein fehlgeschlagenes Outbox-Ereignis zur erneuten Verarbeitung zurück.\n\nBeispiel: zelyra outbox requeue evt_123 main.zyl",
        "outbox-run" => "Befehl: zelyra outbox run [datei.zyl] --handler <event.type=funktionsname> [--once] [--lease-seconds <5..3600>]\n\nStartet einen Worker mit ausdrücklich zugeordneten Handlern.\n\nBeispiel: zelyra outbox run main.zyl --handler invoice.created=send_invoice --once",
        "text" => "Befehle: zelyra text edit <plan.zedit> [--apply --plan-id sha256:<id>]\n\nErstellt eine sichere Vorschau regulärer Textersetzungen oder wendet eine zuvor geprüfte Vorschau an.\n\nDetails: zelyra text edit --help",
        "text-edit" => "Befehl: zelyra text edit <plan.zedit> [--apply --plan-id sha256:<id>]\n\nLiest einen strukturierten Änderungsplan mit Regex-Regeln. Standardmäßig werden nur Vorschau und Prüfsumme ausgegeben; Dateien bleiben unverändert.\n\nAnwenden:\n  zelyra text edit change.zedit\n  zelyra text edit change.zedit --apply --plan-id sha256:<id-aus-der-vorschau>\n\nJede Regel prüft ihre erwartete Trefferzahl. Der Plan muss innerhalb des Projekts liegen; Symlinks und sich überschneidende Regeln werden abgewiesen.",
        "update" => "Befehl: zelyra update [--check]\n\nPrüft auf ein neueres veröffentlichtes Zelyra-Release oder installiert es.\n  --check  Nur Version prüfen; nichts installieren\n\nBeispiele:\n  zelyra update --check\n  zelyra update",
        "tutorial" | "check" | "routes" | "init" | "new" | "setup" | "setup-web" | "editor" | "migrate-laravel" | "run" | "serve" | "db-plan" | "db-apply" | "db-map" | "db-inspect" | "db-create" | "db-setup" | "db-bootstrap" | "db-history" => unreachable!(),
        _ => "Für diesen Befehl gibt es noch keine ausführliche Hilfe. `zelyra --help` zeigt die Befehlsübersicht.",
    }
}

fn english_help(key: &str) -> &'static str {
    match key {
        "db" => "Commands: zelyra db <create|setup|bootstrap|inspect|map|plan|apply|history> <file.zyl>

Manage the database schema declared by a Zelyra project. `plan` previews changes; `apply` applies a reviewed plan.

Database connection:
  ZELYRA_DATABASE_<NAME>_URL  Named project database connection, for example ZELYRA_DATABASE_MAIN_URL
  DATABASE_URL               Fallback for projects with one database

Examples:
  zelyra db plan main.zyl
  zelyra db plan main.zyl --format json
  zelyra db apply main.zyl

Available operations and their limits:
  create     Emit checked CREATE SQL; does not connect to a database
  setup      Create a MariaDB/MySQL database and set up its schema; initialize SQLite
  bootstrap  Alias for `setup`
  inspect    Read the existing database structure
  map        Compare declared and live tables
  plan       Preview schema changes without modifying the database
  apply      Apply a reviewed schema change plan
  history    Inspect journaled MariaDB/MySQL migrations

A named connection takes precedence over `DATABASE_URL`. Help never prints credentials.",
        "fmt" => "Command: zelyra fmt <file.zyl> [--check]\n\nFormat a Zelyra source file canonically.\n  --check  Check whether the file is formatted; do not write changes.\n\nExamples:\n  zelyra fmt main.zyl\n  zelyra fmt src/invoices.zyl --check\n\nNo database access required.",
        "init" => "Command: zelyra init [directory] [options]\n\nInitialize a Zelyra project in the given directory; defaults to the current directory.\n\nOptions:\n  --mariadb                         Create a MariaDB project with local Docker setup\n  --template minimal|mariadb-crud|mariadb-auth|mariadb-business\n                                    Select a project template\n  --web-port <port>                 Port inside the web container (default: 3000)\n  --host-port <port>                Local web application port\n  --db-host-port <port>             Local MariaDB port\n\nExamples:\n  zelyra init\n  zelyra init ./my-project\n  zelyra init ./invoice --mariadb --host-port 8080\n\nWithout `--mariadb`, no database is configured.",
        "new" => "Command: zelyra new <directory> [options]\n\nCreate a Zelyra project from a selected template.\n\nOptions:\n  --mariadb                         Create a MariaDB project with local Docker setup\n  --template minimal|mariadb-crud|mariadb-auth|mariadb-business\n                                    Select a project template\n  --web-port <port>                 Port inside the web container (default: 3000)\n  --host-port <port>                Local web application port\n  --db-host-port <port>             Local MariaDB port\n\nExamples:\n  zelyra new ./customers --template mariadb-crud\n  zelyra new ./invoice --mariadb --host-port 8080\n\nWithout `--mariadb`, no database is configured.",
        "setup" => "Command: zelyra setup [directory] [action]\n\nPrepare a MariaDB project. Without an action, create or verify the protected local `.env`.\n\nActions and options:\n  --database        Start MariaDB and the web application\n  --schema          Apply the project schema\n  --all             Start the stack and apply the schema\n  --host-port <p>   Set the local web port\n  --db-host-port <p> Set the local database port\n  --web [--port p]  Start the local setup interface\n\nExamples:\n  zelyra setup\n  zelyra setup ./invoice --all\n  zelyra setup ./invoice --web\n\n`--all` requires Docker Compose and project configuration. Progress and the web address are printed. Changes requiring explicit approval are refused; review `zelyra db plan main.zyl` and then approve them deliberately with `zelyra db apply main.zyl --allow-risky`.",
        "setup-web" => "Command: zelyra setup [directory] --web [--port <port>]\n\nStart the local setup interface for preparing the database and schema.\n  --port <port>  Local HTTP port (default: 3030)\n\nExample:\n  zelyra setup ./invoice --web --port 3031\n\nThe command prints a local URL with a session token. Only local connections are accepted.",
        "tutorial" => "Command: zelyra --tutorial invoice [directory] [--no-start]\n\nCreate the guided invoice app with customers, items, dashboard, invoices, 19% VAT, and a print view. By default Zelyra starts MariaDB and the web app, applies the schema, and opens the default browser.\n\nOptions:\n  [directory]  Target directory (default: ./invoice-tutorial)\n  --no-start   Create files without starting Docker or opening a browser\n\nExamples:\n  zelyra --tutorial invoice\n  zelyra --tutorial invoice ./my-invoices --no-start\n\nDocker Compose is required for automatic startup. If no graphical browser is available, Zelyra prints the application URL.",
        "check" => "Command: zelyra check <file.zyl> [--format human|json]\n\nCheck syntax, types, and known project dependencies and report diagnostics.\n  --format human  Readable diagnostics (default)\n  --format json   Machine-readable diagnostics\n\nExamples:\n  zelyra check main.zyl\n  zelyra check src/customers.zyl --format json\n\nNo database access required.",
        "routes" => "Command: zelyra routes <entry.zyl> [--format human|json]\n\nList declared pages, APIs, and generated CRUD, form, authentication, and table-view routes. Duplicate or overlapping routes are reported as errors.\n\nExamples:\n  zelyra routes main.zyl\n  zelyra routes main.zyl --format json\n\nReads source only; no running app or database is required.",
        "migrate" => "Command: zelyra migrate laravel <project> [--report <path>] [--output <target>]\n\nThe Laravel Migrator creates a read-only report or, when requested, a separate Zelyra scaffold. It does not run PHP or Artisan and does not read `.env` files.\n\nDetails:\n  zelyra migrate laravel --help",
        "migrate-laravel" => "Command: zelyra migrate laravel <project> [--report <path>] [--output <target>] [--format markdown|json]\n\nStatically analyzes Laravel source and creates a report outside the source project. PHP and Artisan are not run, no database is contacted, and `.env` files are not read.\n\nOptions:\n  --report <path>             New report path outside the project; it must not exist\n  --output <target>           New, separate Zelyra project without copied Laravel logic\n  --format markdown|json      Output format; defaults from the file extension\n\nExamples:\n  zelyra migrate laravel ./legacy-app --report ./reports/legacy-app.md --output ./new-app\n  zelyra migrate laravel ./legacy-app --report ./reports/legacy-app.md\n  zelyra migrate laravel ./legacy-app --report ./reports/legacy-app.json --format json\n\nRecognized syntax does not prove business or security parity. Ambiguous findings remain marked for manual review.",
        "editor" => "Command: zelyra editor [directory] [--port <port>] [--no-open]\n\nStart Zelyra Studio locally and open the default browser. Defaults to the current directory.\n\nOptions:\n  --port <port>  Local Studio port (default: 4177)\n  --no-open      Do not open a browser automatically\n\nExamples:\n  zelyra editor\n  zelyra editor ./invoice --port 4180\n\nStudio provides editing, compiler diagnostics, console execution, and a local web preview. The web app's database must already be reachable.",
        "run" => "Command: zelyra run <file.zyl> [--max-steps <n>]\n\nRun a Zelyra console program. Database, network, and process effects follow project configuration and grants. `--max-steps` activates a statement budget for bounded runs; without the option, long-running programs are allowed.\n\nExample: zelyra run main.zyl --max-steps 100000\n\nUse `zelyra serve` for web apps; Studio can start a local web preview.",
        "serve" => "Command: zelyra serve <file.zyl> [address]\n\nStart a declarative Zelyra web application.\n  address  Bind address (default: 127.0.0.1:3000)\n\nExamples:\n  zelyra serve main.zyl\n  zelyra serve main.zyl 127.0.0.1:8080\n\nThe project database must be reachable and configured in the process environment.",
        "db-plan" => "Command: zelyra db plan <file.zyl> [--format text|json]\n\nCompare the declared schema to the configured database and preview a safe change plan. This does not change the database.\n\nExamples:\n  zelyra db plan main.zyl\n  zelyra db plan main.zyl --format json\n\nRequires the configured project database.",
        "db-apply" => "Command: zelyra db apply <file.zyl> [--plan-id <sha256:…>] [--allow-risky]\n\nApply a schema change plan. With `--plan-id`, the current plan must match the reviewed identifier exactly.\n\nExamples:\n  zelyra db plan main.zyl --format json\n  zelyra db apply main.zyl --plan-id sha256:<id>\n\nRisky changes require `--allow-risky`; destructive changes retain a separate confirmation. Requires the configured project database.",
        "db-map" => "Command: zelyra db map <file.zyl> [--format text|json]\n\nShow tables grouped by source module, declared relationships, and undeclared live tables. Module ownership is informational.\n\nExample: zelyra db map main.zyl --format json\n\nRequires the configured project database.",
        "db-inspect" => "Command: zelyra db inspect <file.zyl>\n\nRead the current state of the configured project database.\n\nExample: zelyra db inspect main.zyl\n\nDoes not modify database objects.",
        "db-create" => "Command: zelyra db create <file.zyl>\n\nEmit checked CREATE SQL for the declared tables. This command does not connect to a database and does not create the database or tables.\n\nExample:\n  zelyra db create main.zyl\n  zelyra db create main.zyl > schema.sql\n\nTo create the MariaDB or MySQL database and apply its schema, run `zelyra db setup main.zyl`. For a local MariaDB Docker project, run `zelyra setup . --all`.",
        "db-setup" | "db-bootstrap" => "Command: zelyra db setup|bootstrap <file.zyl>\n\nMariaDB and MySQL: create the declared database if needed and apply its schema. SQLite: initialize the configured file and tables. PostgreSQL: `setup` is unsupported; use `zelyra db apply`. Review pending changes first with `zelyra db plan`.\n\nExamples:\n  zelyra db plan main.zyl\n  zelyra db setup main.zyl\n\nRequires the configured project database.",
        "db-history" => "Command: zelyra db history <file.zyl> [--format text|json]\n\nShow the progress of journaled MariaDB and MySQL schema migrations.\n\nExample: zelyra db history main.zyl\n\nRequires the configured MariaDB or MySQL project database.",
        _ => english_common_help(key),
    }
}

fn english_common_help(key: &str) -> &'static str {
    match key {
        "impact" => "Command: zelyra impact <file.zyl> [options]\n\nShow known dependencies and the impact of a source file.\n  --format human|json  Output format\n  --symbol <kind:name> Focus on one known symbol\n\nExample: zelyra impact main.zyl --format json\n\nNo database access required.",
        "context" => "Command: zelyra context <file.zyl> [--format human|json]\n\nOutput structured compiler context for Studio and tooling.\n\nExample: zelyra context main.zyl --format json\n\nNo database access required.",
        "config" => "Command: zelyra config <file.zyl> [--format human|json]\n\nShow resolved project configuration and effective capabilities.\n\nExample: zelyra config main.zyl\n\nConfiguration output does not require secrets.",
        "build" => "Command: zelyra build <file.zyl>\n\nCheck and build a Zelyra project for the currently supported execution path.\n\nExample: zelyra build main.zyl\n\nNo database access required.",
        "edit" => "Command: zelyra edit --format=json [--apply] <change.json>\n\nCheck a structured source change plan. Without `--apply`, this only produces a preview.\n\nExample: zelyra edit --format=json change.json\n\nChanges are written only after explicit apply.",
        "doctor" => "Command: zelyra doctor [file.zyl] [--port <port>] [--json]\n\nCheck local prerequisites, project configuration, and optional web reachability.\n\nExample: zelyra doctor main.zyl --json\n\nFor generated MariaDB projects, `--env-file <path>` explicitly selects an environment file.",
        "verify" => "Command: zelyra verify <file.zyl> [--json]\n\nRun available static verification conditions and report their status. Results apply only to the documented scope.\n\nExample: zelyra verify main.zyl --json",
        "doc" => "Command: zelyra doc <file.zyl> [--openapi|--typescript]\n\nGenerate documentation or supported API output from a Zelyra project.\n\nExamples:\n  zelyra doc main.zyl --openapi\n  zelyra doc main.zyl --typescript",
        "auth" => "Commands: zelyra auth <action>\n\n  hash-password [--stdin]                          Hash a password safely\n  role grant|revoke <file.zyl> <user-id> <role>   Grant or revoke a role\n  role-permission grant|revoke <file.zyl> <role> <permission> Grant/revoke permission\n\nExamples:\n  zelyra auth hash-password\n  zelyra auth role grant main.zyl 42 editor\n\nRole changes require a configured MariaDB and auth definition.",
        "auth-hash-password" => "Command: zelyra auth hash-password [--stdin]\n\nCreate a secure password hash. Interactive input is not echoed.\n  --stdin  Read the password from standard input for automation\n\nExamples:\n  zelyra auth hash-password\n  printf '%s' \"$PASSWORD\" | zelyra auth hash-password --stdin",
        "auth-role" => "Commands:\n  zelyra auth role grant <file.zyl> <user-id> <role>\n  zelyra auth role revoke <file.zyl> <user-id> <role>\n\nGrant or revoke a role. Requires auth role tables and a MariaDB URL.\n\nExample: zelyra auth role grant main.zyl 42 editor",
        "auth-role-permission" => "Commands:\n  zelyra auth role-permission grant <file.zyl> <role> <permission>\n  zelyra auth role-permission revoke <file.zyl> <role> <permission>\n\nGrant or revoke a permission. Requires auth role tables and a MariaDB URL.\n\nExample: zelyra auth role-permission grant main.zyl editor invoices.read",
        "audit" => "Commands: zelyra audit inspect|export|verify|prune <file.zyl>\n\n  inspect [--limit n]                   Display events\n  export [--limit n] [--format json|csv] Export events\n  verify                                Validate chained audit integrity\n  prune --before <time> [--confirm]     Delete old events after confirmation\n\nExamples:\n  zelyra audit inspect main.zyl\n  zelyra audit export main.zyl --format csv",
        "audit-inspect" => "Command: zelyra audit inspect <file.zyl> [--limit <n>]\n\nDisplay the latest audit events.\n\nExample: zelyra audit inspect main.zyl --limit 50",
        "audit-export" => "Command: zelyra audit export <file.zyl> [--limit <n>] [--format json|csv]\n\nExport audit events as JSON or CSV.\n\nExample: zelyra audit export main.zyl --format csv",
        "audit-verify" => "Command: zelyra audit verify <file.zyl>\n\nVerify the integrity chain of stored audit events.\n\nExample: zelyra audit verify main.zyl",
        "audit-prune" => "Command: zelyra audit prune <file.zyl> --before <time> [--confirm]\n\nRemove old audit events. Without `--confirm`, nothing is deleted.\n\nExample: zelyra audit prune main.zyl --before 2025-01-01T00:00:00Z --confirm",
        "form" | "form-validate" => "Command: zelyra form validate <file.zyl> <FormName> [field=value ...]\n\nValidate input using the declared form rules. Decimal allows at most 9 fractional digits; Money at most 15 integer and 4 fractional digits. Nonfinite values and silent rounding are rejected.\n\nExample: zelyra form validate main.zyl Customer name=Anna email=anna@example.org",
        "module" => "Commands: zelyra module plan|bundle <entry.zyl> <module.zyl|resource-id>\n\n`plan` previews dependencies. `bundle` writes a checked module bundle to a target directory.\n\nDetails: zelyra module plan --help and zelyra module bundle --help",
        "module-plan" => "Command: zelyra module plan <entry.zyl> <module.zyl|resource-id>\n\nPreview known source, table, database, and resource dependencies of a module.\n\nExample: zelyra module plan main.zyl src/invoices.zyl",
        "module-bundle" => "Command: zelyra module bundle <entry.zyl> <module.zyl|resource-id> --output <directory> [options]\n\nCreate an isolated module bundle.\n  --dry-run                 Print a machine-readable file plan only\n  --docker                  Generate Docker artifacts\n  --compiler-ref <commit>   Set the 40-character compiler commit for Docker\n\nExample: zelyra module bundle main.zyl src/invoices.zyl --output dist/invoices --dry-run",
        "outbox" => "Commands: zelyra outbox setup|list|requeue|run\n\nInspect and process transactional outbox events. `run` requires at least one handler.\n\nDetails: zelyra outbox setup --help, zelyra outbox list --help, zelyra outbox requeue --help, zelyra outbox run --help",
        "outbox-setup" => "Command: zelyra outbox setup [file.zyl]\n\nSet up the transactional outbox tables.\n\nExample: zelyra outbox setup main.zyl",
        "outbox-list" => "Command: zelyra outbox list [file.zyl] [--limit <1..200>] [--format text|json]\n\nList pending, leased, and failed events.\n\nExample: zelyra outbox list main.zyl --limit 50 --format json",
        "outbox-requeue" => "Command: zelyra outbox requeue <event-id> [file.zyl]\n\nReset a failed outbox event for retry.\n\nExample: zelyra outbox requeue evt_123 main.zyl",
        "outbox-run" => "Command: zelyra outbox run [file.zyl] --handler <event.type=function_name> [--once] [--lease-seconds <5..3600>]\n\nStart a worker with explicitly mapped handlers.\n\nExample: zelyra outbox run main.zyl --handler invoice.created=send_invoice --once",
        "text" => "Commands: zelyra text edit <plan.zedit> [--apply --plan-id sha256:<id>]\n\nPreview safe regular-expression text replacements or apply a reviewed preview.\n\nDetails: zelyra text edit --help",
        "text-edit" => "Command: zelyra text edit <plan.zedit> [--apply --plan-id sha256:<id>]\n\nRead a structured change plan with regex rules. By default, print only a preview and plan digest; source files remain unchanged.\n\nApply a reviewed plan:\n  zelyra text edit change.zedit\n  zelyra text edit change.zedit --apply --plan-id sha256:<id-from-preview>\n\nEach rule checks its expected match count. Plans must stay inside the project; symlinks and overlapping rules are rejected.",
        "update" => "Command: zelyra update [--check]\n\nCheck for a newer published Zelyra release or install it.\n  --check  Check the version only; do not install\n\nExamples:\n  zelyra update --check\n  zelyra update",
        "tutorial" | "check" | "routes" | "fmt" | "init" | "new" | "setup" | "setup-web" | "editor" | "migrate-laravel" | "run" | "serve" | "db-plan" | "db-apply" | "db-map" | "db-inspect" | "db-create" | "db-setup" | "db-bootstrap" | "db-history" => unreachable!(),
        _ => "Detailed help is not available for this command yet. Run `zelyra --help` for the command overview.",
    }
}
