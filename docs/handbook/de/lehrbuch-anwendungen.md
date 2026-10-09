# TEIL VI – DATENBANKEN MIT ZELYRA

---

## Kapitel 23: Warum Zelyra die Datenbank direkt versteht

### 1. Was lerne ich in diesem Kapitel?
In diesem Kapitel lernst du:
- Warum die Verbindung zwischen Programmiersprache und relationaler Datenbank traditionell oft fehleranfällig ist.
- Was das Problem der „ORM-Kluft“ (*Object-Relational Impedance Mismatch*) ist.
- Wie Zelyra Datenbanken als Bürger erster Klasse (*First-Class Citizen*) in die Sprache integriert.
- Wie Zelyra SQL-Befehle bereits zur Compile-Zeit auf syntaktische und typbezogene Korrektheit prüft.

### 2. Warum ist das Thema wichtig?
In fast allen gängigen Web-Frameworks (PHP/Laravel, Python/Django, Node/TypeORM) existiert eine unsaubere Trennung: Entwickler schreiben SQL-Strings oder nutzen komplexe Abstraktionsschichten (ORMs). Tippfehler in Spaltennamen wie `user.emaiil` werden oft erst bemerkt, wenn ein Nutzer im laufenden Betrieb einen Fehler 500 erhält. Zelyra beendet dieses Risiko: Wenn eine SQL-Abfrage nicht zum definierten Tabellenschema passt, verweigert der Compiler den Build sofort.

### 3. Verständliche Erklärung
In Zelyra definierst du deine Datenbank-Konfiguration direkt im Quelltext mit dem Schlüsselwort `database`:

```zelyra
database main {
    engine: mariadb
    database: "aufgaben_db"
}

fn main() {
    print("Datenbank-Konfiguration initialisiert.")
}
```

Wenn du Daten aus der Datenbank abfragst, schreibst du echtes SQL – aber der Compiler weiß genau, welche Spalten existieren:
- Schreibt man `SELECT id, beschreibung FROM tasks`, ist das gültig.
- Schreibt man `SELECT gibts_nicht FROM tasks`, meldet Zelyra schon beim Prüfen: `unknown column gibts_nicht`.

### 4. Kleine, aufeinander aufbauende Beispiele

**Beispiel 1: Datenbank und Tabelle deklarieren**
```zelyra
database main {
    engine: mariadb
    database: "test_db"
}

table tasks {
    id: Id primary auto
    beschreibung: String(255) required
}

fn main() {
    print("Datenbank und Tabelle geprueft.")
}
```

**Beispiel 2: SQL-Prüfung zur Compile-Zeit**
```zelyra
database main {
    engine: mariadb
    database: "test_db"
}

table tasks {
    id: Id primary auto
    beschreibung: String(255) required
}

fn zeige_aufgaben() uses Database {
    daten = sql<Task[]> {
        SELECT id, beschreibung
        FROM tasks
    }
    print("SQL typgeprueft.")
}

fn main() uses Database {
    zeige_aufgaben()
}
```

### 5. Typische Fehler und deren Ursachen
- **Fehler:** SQL-Abfragen ohne Deklaration von `uses Database` in der Funktion ausführen.
  *Ursache:* Zelyras Capability-System schützt vor unerlaubten Datenbankzugriffen.
- **Fehler:** Den Datenbankblock `database main` vergessen.
  *Ursache:* Ohne Ziel-Engine kann Zelyra das SQL-Schema nicht verifizieren.

### 6. Merksätze
1. Zelyra schließt die Kluft zwischen Code und Datenbank.
2. SQL-Abfragen werden zur Compile-Zeit typgeprüft.
3. Datenbankzugriffe erfordern zwingend `uses Database`.

### 7. Übungsaufgaben
- **Stufe 1 (Leicht):** Erstelle einen `database`-Block für SQLite oder MariaDB.
- **Stufe 2 (Mittel):** Modelliere eine Tabelle `benutzer` und schreibe eine SQL-Abfrage, die alle Benutzer selektiert.
- **Stufe 3 (Anspruchsvoll):** Provoziere absichtlich einen Tippfehler in einem SQL-Spaltennamen und beobachte, wie Zelyra den Fehler exakt meldet.

### 8. Praxisaufgabe: Die Datenbank der Aufgabenverwaltung anbinden
Erstelle das Fundament unserer Aufgabenverwaltung:
```zelyra
database main {
    engine: mariadb
    database: "zelyra_tasks"
}

table tasks {
    id: Id primary auto
    name: String(100) required
    erledigt: Bool
}

fn status_bericht() uses Database {
    liste = sql<Task[]> {
        SELECT id, name, erledigt
        FROM tasks
    }
    print("Datenbank fuer Aufgabenverwaltung einsatzbereit.")
}

fn main() uses Database {
    status_bericht()
}
```

### 9. Zusammenfassung
- Datenbanken und Schemas sind in Zelyra integraler Bestandteil der Sprache.
- Tippfehler in SQL werden bereits zur Entwicklungszeit verhindert.

### 10. Kontrollfragen zur Selbstprüfung
1. Welches Problem lösen Zelyras typgeprüfte SQL-Blöcke gegenüber gewöhnlichen SQL-Strings?
2. Welche Fähigkeit muss eine Funktion deklarieren, die `sql` ausführt?
3. Wie leitet Zelyra den Typ `Aufgabe` aus der Tabelle `aufgaben` ab?

---

## Kapitel 24: Tabellen definieren und Daten modellieren

### 1. Was lerne ich in diesem Kapitel?
- Wie relationale Tabellen mit `table` deklariert werden.
- Die Syntax für Primärschlüssel: `id: Id primary auto`.
- Wie Spalteneigenschaften definiert werden: `required`, Längenbegrenzung `String(100)`, Standardwerte.
- Wie Beziehungen zwischen Tabellen modelliert werden.

### 2. Warum ist das Thema wichtig?
Das Datenmodell ist das Fundament jeder Anwendung. Wenn das Schema unsauber entworfen ist, schleppt man Datenmüll und Performanceprobleme über Jahre mit sich herum. Zelyra erzwingt von Anfang an klare Pflichtfelder, Typen und Integrität.

### 3. Verständliche Erklärung
Eine Tabelle (`table`) ist wie ein Aktenordner für gleichartige Datenblätter:
- Jedes Datenblatt hat eine eindeutige laufende Nummer: `id: Id primary auto`.
- Bestimmte Angaben dürfen niemals fehlen: `required`.
- Für Texte kannst du Längenbegrenzungen angeben: `String(100)`.

```zelyra
table kategorien {
    id: Id primary auto
    bezeichnung: String(50) required
}
```

Aus der Tabellendefinition `table kategorien` generiert Zelyra automatisch den Datentyp `Kategorie` mit den exakten Feldern.

### 4. Kleine, aufeinander aufbauende Beispiele

**Beispiel 1: Einfache Tabelle mit Pflichtfeldern**
```zelyra
database main {
    engine: mariadb
    database: "app_db"
}

table projekte {
    id: Id primary auto
    name: String(80) required
    aktiv: Bool
}

fn main() {
    print("Tabelle projekte deklariert.")
}
```

**Beispiel 2: Tabelle mit Datums- und Zahlenfeldern**
```zelyra
database main {
    engine: mariadb
    database: "app_db"
}

table zeiterfassungen {
    id: Id primary auto
    stunden: Float
    erfasst_am: Timestamp
}

fn main() {
    print("Tabelle zeiterfassungen deklariert.")
}
```

**Beispiel 3: Verknüpfung zweier Tabellen über IDs**
```zelyra
database main {
    engine: mariadb
    database: "app_db"
}

table tasks {
    id: Id primary auto
    beschreibung: String(200) required
    projekt_id: Id
}

fn main() {
    print("Beziehung aufgaben -> projekt_id angelegt.")
}
```

### 5. Typische Fehler und deren Ursachen
- **Fehler:** Ein Pflichtfeld beim Anlegen weglassen.
  *Ursache:* Felder mit `required` müssen in jedem Datensatz gültige Werte besitzen.
- **Fehler:** Reservierte Wörter wie `action`, `field`, `title` als Spaltennamen wählen.
  *Ursache:* Diese Bezeichner sind für Zelyra-Sprachkonstrukte reserviert.

### 6. Merksätze
1. Jede Tabelle benötigt einen Primärschlüssel `id: Id primary auto`.
2. Das Attribut `required` verbietet leere Einträge auf Datenbank- und Sprachebene.
3. Der Singularname der Tabelle (z. B. `Aufgabe` für `aufgaben`) wird zum automatischen Datentyp.

### 7. Übungsaufgaben
- **Stufe 1 (Leicht):** Definiere eine Tabelle `etiketten` mit einem Pflichtfeld `name: String(30) required`.
- **Stufe 2 (Mittel):** Ergänze eine Tabelle `kunden` um `email: Email` und `telefon: String(30)`.
- **Stufe 3 (Anspruchsvoll):** Modelliere eine Tabelle `kommentare`, die per `aufgabe_id: Id` mit einer Aufgabe verknüpft ist und einen `erstellt_am: Timestamp` besitzt.

### 8. Praxisaufgabe: Vollständiges Schema für die Aufgabenverwaltung
Erstelle das produktive Datenmodell unserer Aufgabenverwaltung:
```zelyra
database main {
    engine: mariadb
    database: "zelyra_tasks"
}

table tasks {
    id: Id primary auto
    name: String(120) required
    beschreibung: String(500)
    prioritaet: Int
    ist_erledigt: Bool
}

fn main() {
    print("Vollstaendiges Aufgaben-Schema aktiv.")
}
```

### 9. Zusammenfassung
- `table` definiert Struktur, Typen und Beschränkungen der Daten.
- Zelyra sorgt dafür, dass Datenbank-Struktur und Code-Typen immer synchron bleiben.

### 10. Kontrollfragen zur Selbstprüfung
1. Wozu dient die Kennzeichnung `auto` beim Primärschlüssel?
2. Was bewirkt das Schlüsselwort `required` an einer Tabellenspalte?
3. Welcher Typname entsteht automatisch aus der Tabelle `projekte`?

---

## Kapitel 25: Daten abfragen und verändern

### 1. Was lerne ich in diesem Kapitel?
- Wie du mit `sql<T[]>` Datensätze sicher aus der Datenbank liest.
- Wie SQL-Injection durch parametrisierte Abfragen (`:param`) unmöglich gemacht wird.
- Wie man Datensätze mit `INSERT`, `UPDATE` und `DELETE` verändert.
- Warum Änderungen in `transaction { ... }`-Blöcken zusammengefasst werden.
- Die CLI-Migrationswerkzeuge (`zelyra db setup`, `zelyra db apply`).

### 2. Warum ist das Thema wichtig?
SQL-Injection gehört seit über 20 Jahren zu den gefährlichsten Sicherheitslücken im Web: Ein Angreifer gibt in ein Suchfeld manipulierten Text ein und liest fremde Passwörter aus oder löscht Tabellen. Zelyra schützt deine Software konstruktiv: Parameter in SQL-Blöcken werden mit Doppelpunkt (`:name`) gebunden und von der Engine immer sicher escaped. Gleichzeitig sichern Transaktionen ab, dass bei Fehlern keine halben Buchungen stehenbleiben.

### 3. Verständliche Erklärung
- **Lesen mit sql<T[]>:**
  ```zelyra
  database main {
      engine: mariadb
      database: "tasks_db"
  }
  table tasks {
      id: Id primary auto
      name: String required
      ist_erledigt: Bool
  }
  fn lade(filter_wert: Bool) uses Database {
      meine_tasks = sql<Task[]> {
          SELECT id, name, ist_erledigt
          FROM tasks
          WHERE ist_erledigt = :filter_wert
      }
      print("Tasks geladen")
  }
  fn main() uses Database { lade(false) }
  ```
- **Schreiben in einer Transaktion:**
  ```zelyra
  database main {
      engine: mariadb
      database: "tasks_db"
  }
  table tasks {
      id: Id primary auto
      name: String required
      ist_erledigt: Bool
  }
  fn anlegen(neuer_name: String) uses Database {
      fertig_flag = false
      transaction {
          sql {
              INSERT INTO tasks (name, ist_erledigt)
              VALUES (:neuer_name, :fertig_flag)
          }
      }
  }
  fn main() uses Database { anlegen("Test") }
  ```
  Sollte während der Transaktion etwas schiefgehen, macht die Datenbank alle Änderungen ungeschehen (*Rollback*).

### 4. Kleine, aufeinander aufbauende Beispiele

**Beispiel 1: Einen neuen Datensatz einfügen**
```zelyra
database main {
    engine: mariadb
    database: "tasks_db"
}

table tasks {
    id: Id primary auto
    name: String(100) required
    fertig: Bool
}

fn fuege_aufgabe_ein(text: String) uses Database {
    fertig_status = false
    transaction {
        sql {
            INSERT INTO tasks (name, fertig)
            VALUES (:text, :fertig_status)
        }
    }
    print("Aufgabe gespeichert.")
}

fn main() uses Database {
    fuege_aufgabe_ein("E-Mail beantworten")
}
```

**Beispiel 2: Datensätze typisiert abfragen**
```zelyra
database main {
    engine: mariadb
    database: "tasks_db"
}

table tasks {
    id: Id primary auto
    name: String(100) required
    fertig: Bool
}

fn lade_alle() uses Database {
    liste = sql<Task[]> {
        SELECT id, name, fertig
        FROM tasks
    }
    print("Aufgabenliste geladen.")
}

fn main() uses Database {
    lade_alle()
}
```

**Beispiel 3: Datensatz aktualisieren**
```zelyra
database main {
    engine: mariadb
    database: "tasks_db"
}

table tasks {
    id: Id primary auto
    name: String(100) required
    fertig: Bool
}

fn markiere_als_fertig(aufgabe_id: Int) uses Database {
    transaction {
        sql {
            UPDATE tasks
            SET fertig = true
            WHERE id = :aufgabe_id
        }
    }
    print("Status aktualisiert.")
}

fn main() uses Database {
    markiere_als_fertig(1)
}
```

### 5. Typische Fehler und deren Ursachen
- **Fehler:** Werte mit String-Verkettung in SQL einfügen wollen (`"WHERE id = " + id`).
  *Ursache:* In Zelyra gibt es keine String-Zusammenstückelung in SQL. Nutze immer Parameter mit Doppelpunkt (`:id`).
- **Fehler:** Vergessen, dass Änderungen in `transaction { ... }` gekapselt sein müssen.
  *Ursache:* Zelyra verlangt für Schreiboperationen klare Transaktionsgrenzen.

### 6. Merksätze
1. Binde Eingabewerte in SQL stets mit `:parameter` – das schützt vor SQL-Injection.
2. Schreibende Operationen gehören in `transaction { ... }`.
3. Mit `zelyra db apply` wird das definierte Schema auf die Datenbank übertragen.

### 7. Übungsaufgaben
- **Stufe 1 (Leicht):** Schreibe eine SQL-Abfrage, die alle unfertigen Aufgaben (`fertig = false`) selektiert.
- **Stufe 2 (Mittel):** Schreibe eine Funktion zum Löschen einer Aufgabe anhand ihrer ID (`DELETE FROM tasks WHERE id = :id`).
- **Stufe 3 (Anspruchsvoll):** Implementiere eine Funktion, die innerhalb einer einzigen Transaktion eine alte Aufgabe archiviert und eine neue Nachfolge-Aufgabe anlegt.

### 8. Praxisaufgabe: Vollständige Datenbank-Operationen der Aufgabenverwaltung
Schreibe den Datenzugriff für unsere Aufgabenverwaltung:
```zelyra
database main {
    engine: mariadb
    database: "zelyra_tasks"
}

table tasks {
    id: Id primary auto
    name: String(100) required
    ist_erledigt: Bool
}

fn aufgabe_anlegen(aufgabe_name: String) uses Database {
    status_initial = false
    transaction {
        sql {
            INSERT INTO tasks (name, ist_erledigt)
            VALUES (:aufgabe_name, :status_initial)
        }
    }
    print("Aufgabe angelegt.")
}

fn aufgabe_abschliessen(ziel_id: Int) uses Database {
    transaction {
        sql {
            UPDATE tasks
            SET ist_erledigt = true
            WHERE id = :ziel_id
        }
    }
    print("Aufgabe abgeschlossen.")
}

fn main() uses Database {
    aufgabe_anlegen("Erstes Zelyra Projekt starten")
    aufgabe_abschliessen(1)
}
```

### 9. Zusammenfassung
- SQL in Zelyra ist nativ, typsicher und automatisch vor Angriffen geschützt.
- `transaction` schützt die Datenbankkonsistenz bei allen Änderungen.

### 10. Kontrollfragen zur Selbstprüfung
1. Wie schützt Zelyra vor bösartigen SQL-Injection-Angriffen?
2. Warum müssen schreibende SQL-Befehle in einem `transaction`-Block stehen?
3. Welcher CLI-Befehl richtet die Datenbanktabellen anhand des Codes ein?

# TEIL VII – WEBANWENDUNGEN UND FORMULARE

---

## Kapitel 26: Webseiten ausgeben

### 1. Was lerne ich in diesem Kapitel?
In diesem Kapitel lernst du:
- Wie du mit `page` blitzschnell Webseiten und Routen erstellst.
- Wie URL-Parameter (z. B. `/tasks/{id}`) dynamisch übergeben werden.
- Wie HTML-Vorlagen direkt im Zelyra-Code definiert werden (`html { ... }`).
- Wie Zelyra Cross-Site Scripting (XSS) durch automatisches HTML-Escaping verhindert.

### 2. Warum ist das Thema wichtig?
Im klassischen Web-Development muss man oft drei verschiedene Welten verbinden: Einen Webserver (wie Nginx oder Apache), einen Router, eine Template-Engine (Blade, Jinja, Twig) und den Anwendungscode. Wenn man an einer Stelle vergisst, HTML-Sonderzeichen zu maskieren, können Angreifer bösartiges JavaScript einschleusen (XSS). In Zelyra ist der Webserver direkt integriert (`zelyra serve`), und das HTML-Escaping geschieht automatisch und unumgänglich.

### 3. Verständliche Erklärung
Mit dem Schlüsselwort `page` definierst du eine Webroute und das dazugehörige HTML:

```zelyra
page "/willkommen" {
    html {
        <h1>Willkommen bei Zelyra</h1>
        <p>Deine moderne Webanwendung laeuft!</p>
    }
}
```

Wenn du dynamische Werte anzeigen möchtest, setzt du sie einfach in geschweifte Klammern: `{name}`. Zelyra ersetzt den Platzhalter sicher durch den echten Text.

### 4. Kleine, aufeinander aufbauende Beispiele

**Beispiel 1: Einfache statische Begrüßungsseite**
```zelyra
page "/hallo" {
    html {
        <html>
            <body>
                <h1>Hallo Zelyra-Welt!</h1>
            </body>
        </html>
    }
}
```

**Beispiel 2: Dynamische Route mit URL-Parameter**
```zelyra
page "/benutzer/{name}" {
    html {
        <html>
            <body>
                <h1>Profil von {name}</h1>
                <p>Willkommen zurueck im Dashboard.</p>
            </body>
        </html>
    }
}
```

**Beispiel 3: Sicheres Escaping gegen XSS-Angriffe**
Übergibt ein Nutzer als Namen `<script>alert('hack')</script>`, gibt Zelyra dies im Browser als harmlosen Text aus – das Skript wird niemals ausgeführt:
```zelyra
page "/sicher/{eingabe}" {
    html {
        <div>Eingabe: {eingabe}</div>
    }
}
```

**Beispiel 4: Wiederverwendbare View-Layouts und Slots (ab Zelyra 0.1.41)**
Statt auf jeder Seite `<html>`, `<head>`, Header und Footer neu zu schreiben, definierst du mit `view` ein Layout. Ein View besitzt genau einen Hauptslot `<slot />` sowie optionale benannte Slots mit sicherem Standardinhalt:
```zelyra
view AppShell {
    html {
        <html lang="de">
            <head><title>Zelyra Anwendung</title></head>
            <body>
                <header>
                    <slot name="header"><h1>Zelyra Portal</h1></slot>
                </header>
                <main>
                    <slot />
                </main>
                <footer>
                    <slot name="footer"><p>Erstellt mit Zelyra</p></slot>
                </footer>
            </body>
        </html>
    }
}

page "/dashboard" {
    view: AppShell
    html {
        <slot name="header"><h1>Mein Dashboard</h1></slot>
        <p>Der eigentliche Inhalt wird im Haupt-Slot der AppShell platziert.</p>
    }
}
```

**Beispiel 5: Deklarative Suche, Filterung und Pagination (ab Zelyra 0.1.40)**
Für datengetriebene Seiten erzeugt Zelyra semantische Steuerungen für Suche, Sortierung und Seitenaufteilung automatisch, inklusive URL-Zustandserhalt:
```zelyra
database main {
    engine: mariadb
}

table tasks {
    id: Id primary auto
    title: String(100) required
    done: Bool default false
}

page "/tasks" {
    search { title }
    filter { done }
    sort { title }
    paginated 25

    load tasks = sql<Task[]> {
        SELECT id, title, done
        FROM tasks
        ORDER BY title
    }

    html {
        <h1>Aufgaben ({total} gesamt, Seite {page} von {pages})</h1>
        <ul>
            for task in tasks {
                <li>{task.title}</li>
            }
        </ul>
    }
}
```
Zelyra führt im Hintergrund automatisch die optimierte Zählabfrage (`COUNT(*)`) aus, bindet `total` und `pages` als sichere `UInt`-Variablen und rendert semantische Filter-Fieldsets.

### 5. Typische Fehler und deren Ursachen
- **Fehler:** HTML-Tags nicht ordnungsgemäß schließen (z. B. `<h1>` ohne `</h1>`).
  *Ursache:* Zelyra prüft den HTML-Baum syntaktisch auf Wohlgeformtheit.
- **Fehler:** URL-Parameter in geschweiften Klammern falsch benennen.
  *Ursache:* Der Parameter in der Route (z. B. `{id}`) muss mit der Variablen im HTML übereinstimmen.
- **Fehler `E-VIEW-010` bis `E-VIEW-015`:** Unbekannte Variablen oder falsche Typen in der View-Interpolation.
  *Ursache:* Der Zelyra-Compiler prüft View-Bindungen und Component-Properties bereits zur Compile-Zeit strikt gegen deklarierte Routen, Typen und SQL-Loads.
- **Fehler:** Unbekannte oder doppelte Slots in `page` angeben.
  *Ursache:* Eine Seite darf nur benannte Slots befüllen, die der ausgewählte `view` auch tatsächlich deklariert.

### 6. Merksätze
1. `page "/pfad"` definiert eine Route und liefert geprüften HTML-Code aus.
2. Variablen im HTML werden mit `{variable}` sicher interpoliert und automatisch escaped.
3. `view Name { ... }` definiert wiederverwendbare Master-Layouts mit `<slot />` und benannten Slots (`<slot name="...">`).
4. `search`, `filter` und `paginated` erzeugen vollautomatische, semantische Query-Steuerungen mit Zustandsbewahrung in der URL.
5. Mit `zelyra serve` startest du den integrierten HTTP-Server ohne externe Webserver-Konfiguration.

### 7. Übungsaufgaben
- **Stufe 1 (Leicht):** Erstelle eine `page "/ueber-uns"`, die eine Firmenbeschreibung anzeigt.
- **Stufe 2 (Mittel):** Erstelle eine dynamische Route `/produkt/{nummer}`, die eine Produkt-Detailansicht darstellt.
- **Stufe 3 (Anspruchsvoll):** Gestalte eine Übersichtsseite mit Überschrift, Navigation und Aufzählungsliste im HTML-Block.

### 8. Praxisaufgabe: Startseite für die Aufgabenverwaltung
Erstelle die Web-Startseite unserer Aufgabenverwaltung:
```zelyra
page "/tasks" {
    html {
        <html>
            <head>
                <title>Zelyra Aufgabenverwaltung</title>
            </head>
            <body>
                <h1>Meine Aufgaben</h1>
                <p>Willkommen in deiner persoenlichen Aufgabenverwaltung.</p>
                <a href="/tasks/neu">Neue Aufgabe erstellen</a>
            </body>
        </html>
    }
}
```

Starte den Server mit `zelyra serve main.zyl` und öffne `http://localhost:8080/tasks` im Browser!

### 9. Zusammenfassung
- Webseiten werden mit `page` und `html` direkt deklariert.
- Automatisches Escaping schützt deine Nutzer vor Sicherheitsrisiken.

### 10. Kontrollfragen zur Selbstprüfung
1. Welches Schlüsselwort leitet eine Webseiten-Definition ein?
2. Wie bindet man dynamische Werte in den HTML-Quelltext ein?
3. Warum ist XSS bei der HTML-Ausgabe in Zelyra standardmäßig ausgeschlossen?

---

## Kapitel 27: Formulare und Benutzereingaben

### 1. Was lerne ich in diesem Kapitel?
- Wie du mit `form` sichere Eingabemasken für deine Datenbanktabellen definierst.
- Wie automatische CSRF-Schutzmechanismen funktionieren.
- Wie Zelyra Eingabedaten typisiert validiert (z. B. `Email`, Mindestlängen).
- Wie du Formulare mit dem CLI-Befehl `zelyra form validate` vorab testen kannst.

### 2. Warum ist das Thema wichtig?
Eingaben von Nutzern sind die Hauptursache für Sicherheitslücken im Web: Angreifer übermitteln leere Pflichtfelder, manipulierte IDs oder nutzen fremde Browser-Sitzungen aus (CSRF-Attacken). In anderen Frameworks muss man Formulare mühsam von Hand mit Validierungsregeln, Fehleranzeigen und CSRF-Tokens zusammenbauen. Zelyras `form`-Konstrukt leitet die Eingabemaske direkt aus der Datenbanktabelle ab und sichert alles automatisch ab.

### 3. Verständliche Erklärung
Ein Formular verknüpft eine Eingabemaske mit einer Zieltabelle:

```zelyra
database main {
    engine: mariadb
}

table tasks {
    id: Id primary auto
    name: String(100) required
    beschreibung: String(500)
}

form TaskCreate -> tasks {
    fields {
        name
        beschreibung
    }
}
```

Zelyra generiert daraus:
- Die HTML-Eingabefelder mit passenden Typen (`<input type="text">`, etc.).
- Ein unsichtbares, kryptografisches CSRF-Token, das Angriffe verhindert.
- Server-seitige Validierungsprüfungen (z. B. `name` darf maximal 100 Zeichen haben und nicht fehlen).

### 4. Kleine, aufeinander aufbauende Beispiele

**Beispiel 1: Basis-Formular für Kundendaten**
```zelyra
database main {
    engine: mariadb
}

table customers {
    id: Id primary auto
    name: String(80) required
    email: Email?
}

form CustomerForm -> customers {
    fields {
        name
        email
    }
}
```

**Beispiel 2: Formular mit benutzerdefinierten Aktionen**
```zelyra
database main {
    engine: mariadb
}

table tasks {
    id: Id primary auto
    name: String(100) required
}

form NewTaskForm -> tasks {
    fields {
        name
    }
}
```

### 5. Typische Fehler und deren Ursachen
- **Fehler:** Ein Feld im Formular aufführen, das in der Zieltabelle gar nicht existiert.
  *Ursache:* Zelyra prüft `fields` strikt gegen die Spalten der Tabelle.
- **Fehler:** CSRF-Schutz manuell deaktivieren wollen.
  *Ursache:* In Zelyra ist der CSRF-Schutz unverzichtbarer Sicherheitsstandard.

### 6. Merksätze
1. `form Name -> zieltabelle` generiert eine sichere Eingabemaske.
2. Alle Validierungsregeln der Tabelle (Länge, Pflichtfeld, Typ) gelten automatisch.
3. CSRF- und XSS-Schutz sind integral eingebaut.

### 7. Übungsaufgaben
- **Stufe 1 (Leicht):** Definiere ein Formular `KategorieErstellen` für eine Tabelle `kategorien`.
- **Stufe 2 (Mittel):** Teste das Formular auf der Kommandozeile mit `zelyra form validate`.
- **Stufe 3 (Anspruchsvoll):** Ergänze das Aufgaben-Formular um ein Prioritätsfeld und validiere fehlerhafte Eingaben.

### 8. Praxisaufgabe: Das Erstellungsformular für Aufgaben
Definiere das Eingabeformular für neue Aufgaben:
```zelyra
database main {
    engine: mariadb
}

table tasks {
    id: Id primary auto
    name: String(100) required
    prioritaet: Int
}

form TaskCreate -> tasks {
    fields {
        name
        prioritaet
    }
}

fn main() {
    print("Aufgaben-Formular bereit.")
}
```

### 9. Zusammenfassung
- Formulare verknüpfen Tabellen mit sicheren Web-Eingabemasken.
- Zelyra erledigt Validierung, CSRF-Schutz und Fehlerbehandlung automatisch.

### 10. Kontrollfragen zur Selbstprüfung
1. Wofür steht der Pfeil `->` in `form TaskCreate -> tasks`?
2. Warum müssen Entwickler in Zelyra keine manuellen CSRF-Tokens im HTML einfügen?
3. Welche Spaltenprüfungen werden automatisch auf das Formular angewendet?

---

## Kapitel 28: Das vollständige CRUD-Muster

### 1. Was lerne ich in diesem Kapitel?
- Was CRUD (Create, Read, Update, Delete) bedeutet und warum es das Herzstück von Business-Web-Apps ist.
- Wie Zelyra ein vollständiges Verwaltungsinterface mit nur einem `crud`-Block erzeugt.
- Wie du Listen-, Detail-, Formular- und Löschansichten konfigurierst.
- Wie benutzerdefinierte Aktionen mit `action` hinzugefügt werden.

### 2. Warum ist das Thema wichtig?
Über 80 % der Arbeit an Webanwendungen besteht aus dem immer gleichen Muster: Eine Tabelle anzeigen, Datensätze anlegen, bearbeiten und löschen. Entwickler verbringen Wochen damit, Controller, Routen, Formulare und Bestätigungsdialoge zu schreiben. In Zelyra erledigst du das in wenigen Zeilen deklarativem Code – absolut fehlerfrei, sicher und konsistent.

### 3. Verständliche Erklärung
Das Schlüsselwort `crud` fasst alle Operationen für eine Entität zusammen:
- **C**reate: Neue Datensätze anlegen.
- **R**ead: Liste durchsuchen und Details ansehen.
- **U**pdate: Vorhandene Daten ändern.
- **D**elete: Datensätze mit Sicherheitsabfrage entfernen.

```zelyra
database main {
    engine: mariadb
}

table tasks {
    id: Id primary auto
    name: String(100) required
    erledigt: Bool default false
}

crud Task -> tasks {
    title: "Aufgabenverwaltung"
    view {
        fields {
            name
            erledigt
        }
        list {
            mode: cards
            empty: "Keine Aufgaben vorhanden."
        }
    }
}
```

> **Automatische Detail-Verlinkung bei ausgeblendeter ID (ab Version 0.1.50):**
> Wenn die technische `id`-Spalte im Block `fields` nicht aufgeführt ist (wie hier, wo nur `name` und `erledigt` sichtbar sind), verlinkt Zelyra automatisch das erste angezeigte Feld (`name`) mit der Detailseite des Datensatzes. Dies gilt sowohl für Tabellen- (`table`) als auch für Kachel-Layouts (`cards`).

### 4. Kleine, aufeinander aufbauende Beispiele

**Beispiel 1: Ein vollständiges CRUD-Modul**
```zelyra
database main {
    engine: mariadb
}

table tasks {
    id: Id primary auto
    name: String(100) required
    aktiv: Bool default true
}

crud Task -> tasks {
    title: "Aufgaben"

    view {
        fields {
            name
            aktiv
        }

        list {
            mode: cards
            empty: "Keine Aufgaben vorhanden."
        }

        detail {
            mode: cards
            title: "Aufgabendetails"
        }

        form {
            mode: cards
            title: "Aufgabe bearbeiten"
            submit: "Speichern"
        }

        delete {
            title: "Aufgabe loeschen"
            message: "Diese Aktion kann nicht rueckgaengig gemacht werden."
            submit: "Jetzt loeschen"
        }
    }
}
```

**Beispiel 2: Benutzerdefinierte Aktionen im CRUD-Interface**
Du kannst individuelle Schaltflächen hinzufügen, z. B. um eine Aufgabe sofort als erledigt zu markieren:
```zelyra
database main {
    engine: mariadb
}

table tasks {
    id: Id primary auto
    name: String(100) required
    erledigt: Bool default false
}

crud Task -> tasks {
    title: "Aufgaben"
    view {
        fields {
            name
            erledigt
        }
    }

    action erledigen {
        label: "Als erledigt markieren"
        confirm: "Moechtest du diese Aufgabe abschliessen?"

        sql {
            UPDATE tasks
            SET erledigt = true
            WHERE id = :id
        }

        success "Aufgabe erfolgreich abgeschlossen."
        redirect "/tasks"
    }
}
```

**Beispiel 3: CRUD-Ressource mit wiederverwendbarem View-Layout (ab Zelyra 0.1.43)**
Mit `layout: ViewName` bettest du alle generierten CRUD-Ansichten (Listen-, Detail-, Formular- und Löschdialoge) automatisch in ein zuvor deklariertes Seitenlayout ein:
```zelyra
view AppShell {
    html {
        <html lang="de">
            <body>
                <nav><a href="/">Start</a> | <a href="/tasks">Aufgaben</a></nav>
                <main>
                    <slot />
                </main>
            </body>
        </html>
    }
}

crud Task -> tasks {
    title: "Aufgabenverwaltung"
    layout: AppShell

    view {
        fields {
            name
            erledigt
        }
    }
}
```
Die generierte CRUD-Ressource übernimmt das Navigationsgerüst von `AppShell`, während alle Sicherheitsprüfungen, Rollen und CSRF-Tokens voll aktiv bleiben.

### 5. Typische Fehler und deren Ursachen
- **Fehler:** Den `view`-Block oder die `fields`-Deklaration im CRUD weglassen.
  *Ursache:* Zelyra muss wissen, welche Spalten in den generierten Ansichten angezeigt werden sollen.
- **Fehler:** Ein `UPDATE` in einer `action` ohne `WHERE id = :id` ausführen.
  *Ursache:* Aktionen beziehen sich immer auf den aktuell ausgewählten Datensatz mit `:id`.

### 6. Merksätze
1. `crud Name -> tabelle` generiert eine vollständige, sichere Verwaltungsoberfläche.
2. Ansichten (`list`, `detail`, `form`, `delete`) lassen sich flexibel anpassen.
3. Benutzerdefinierte Aktionen (`action`) erweitern den Standard um individuelle Geschäftsregeln.

### 7. Übungsaufgaben
- **Stufe 1 (Leicht):** Erstelle ein CRUD-Interface für eine Tabelle `kategorien`.
- **Stufe 2 (Mittel):** Passe die Hinweistexte und Titel der Ansichten an.
- **Stufe 3 (Anspruchsvoll):** Implementiere eine benutzerdefinierte Aktion `duplizieren`, die eine Kopie der ausgewählten Aufgabe anlegt.

### 8. Praxisaufgabe: Das produktive CRUD-Interface unserer Aufgabenverwaltung
Kombiniere Datenbank, Tabelle und CRUD zu einem vollständigen System:
```zelyra
database main {
    engine: mariadb
    database: "tasks_app"
}

table tasks {
    id: Id primary auto
    name: String(120) required
    erledigt: Bool default false
}

crud Task -> tasks {
    title: "Meine Aufgabenverwaltung"

    view {
        fields {
            name
            erledigt
        }

        list {
            mode: cards
            empty: "Grossartig! Alle Aufgaben sind erledigt."
        }

        detail {
            mode: cards
            title: "Aufgabe ansehen"
        }

        form {
            mode: cards
            title: "Aufgabe erfassen oder bearbeiten"
            submit: "Aufgabe sichern"
        }

        delete {
            title: "Aufgabe loeschen"
            message: "Soll diese Aufgabe unwiderruflich entfernt werden?"
            submit: "Loeschen"
        }
    }
}
```

### 9. Zusammenfassung
- Das CRUD-Muster beschleunigt die Entwicklung von Datenbank-Web-Apps dramatisch.
- Zelyra generiert fehlerfreie Routen, HTML-Masken und Datenbankaufrufe vollautomatisch.

### 10. Kontrollfragen zur Selbstprüfung
1. Welche vier Grundfunktionen umfasst die Abkürzung CRUD?
2. Wie reagiert Zelyra, wenn in `fields` ein ungültiger Spaltenname steht?
3. Welche Aufgabe erfüllt das Schlüsselwort `action` in einem CRUD-Block?

---

## Kapitel 29: Benutzer, Passwörter und Sitzungen

### 1. Was lerne ich in diesem Kapitel?
- Wie Zelyra Benutzerauthentifizierung mit dem `auth`-Block nativ bereitstellt.
- Wie Passwörter mit dem modernen Argon2-Algorithmus gehasht werden.
- Wie Sitzungen (*Sessions*) und Rollenrechte abgesichert werden.
- Wie Seiten und Aktionen mit `requires auth` und `permits` geschützt werden.

### 2. Warum ist das Thema wichtig?
Sicherheit ist kein nachträgliches Add-on. Wer Passwörter im Klartext oder mit veralteten Algorithmen (wie MD5 oder SHA1) speichert, riskiert Datenschutz-Katastrophen. In Zelyra ist Authentifizierung fest in der Spracharchitektur verankert: Passwörter werden standardmäßig mit Argon2 gehasht, Sitzungs-Tokens werden kryptografisch geschützt und Routenrechte werden deklarativ geprüft.

### 3. Verständliche Erklärung
Ein Authentifizierungsblock verbindet Benutzer, Sitzungen und Berechtigungen:

```zelyra
auth users {
    table: users
    sessions: auth_sessions
    permissions: user_permissions
}

table users {
    id: Id primary auto
    email: Email required unique
    password_hash: String(255) required
}

table auth_sessions {
    id: Id primary auto
    user: User required
    token_hash: String(64) required unique
    expires_at: Timestamp required
    device_label: String(255)
}

table user_permissions {
    id: Id primary auto
    user: User required
    permission: String(100) required
}
```

Um eine Webroute nur für angemeldete Nutzer freizugeben, schreibst du einfach:
```zelyra
auth users {
    table: users
    sessions: auth_sessions
    permissions: user_permissions
}

table users {
    id: Id primary auto
    email: Email required unique
    password_hash: String(255) required
}

table auth_sessions {
    id: Id primary auto
    user: User required
    token_hash: String(64) required unique
    expires_at: Timestamp required
    device_label: String(255)
}

table user_permissions {
    id: Id primary auto
    user: User required
    permission: String(100) required
}

page "/geheim" {
    requires auth
    html {
        <h1>Nur fuer angemeldete Nutzer sichtbar!</h1>
    }
}
```

### 4. Kleine, aufeinander aufbauende Beispiele

**Beispiel 1: Die Standardtabellen für Authentifizierung**
```zelyra
auth users {
    table: users
    sessions: auth_sessions
    permissions: user_permissions
}

table users {
    id: Id primary auto
    email: Email required unique
    password_hash: String(255) required
    active: Bool default true
}

table auth_sessions {
    id: Id primary auto
    user: User required
    token_hash: String(64) required unique
    expires_at: Timestamp required
    device_label: String(255)
}

table user_permissions {
    id: Id primary auto
    user: User required
    permission: String(100) required
}
```

**Beispiel 2: Geschützte Seite mit Rechteprüfung**
```zelyra
auth users {
    table: users
    sessions: auth_sessions
    permissions: user_permissions
}

table users {
    id: Id primary auto
    email: Email required unique
    password_hash: String(255) required
    active: Bool default true
}

table auth_sessions {
    id: Id primary auto
    user: User required
    token_hash: String(64) required unique
    expires_at: Timestamp required
    device_label: String(255)
}

table user_permissions {
    id: Id primary auto
    user: User required
    permission: String(100) required
}

page "/dashboard" {
    requires auth
    permits "tasks.view"

    html {
        <h1>Aufgaben-Dashboard</h1>
        <p>Du bist autorisiert.</p>
    }
}
```

### 5. Typische Fehler und deren Ursachen
- **Fehler:** Passwörter im Klartext in der Tabelle ablegen.
  *Ursache:* Zelyra verlangt ein Spaltenfeld `password_hash` und stellt mit `zelyra auth hash-password` ein Hashing-Tool bereit.
- **Fehler:** Ohne Sitzungstabelle einen Neustart überdauernde Sitzungen erwarten.
  *Ursache:* Ohne `sessions: auth_sessions` liegen Sitzungen im Prozessspeicher.
  Für dauerhafte Speicherung und administrative Sitzungssperren die Tabelle deklarieren.

#### Sitzungsverwaltung im 0.4-Entwicklungszweig

Angemeldete Nutzer können unter `/account/sessions` ihre eigenen aktiven
Sitzungen anzeigen und beenden. Seiten mit generierter Navigation verlinken
auf diese Seite. Für Änderungen sind eine gültige Sitzung,
ein CSRF-Token und ein Herkunftsnachweis derselben Origin erforderlich. Beim
Beenden der aktuellen Sitzung wird das Cookie gelöscht und zur Anmeldung
weitergeleitet. Bei dauerhaften Sitzungen funktionieren Sitzungstabellen mit
und ohne `id`-Spalte. Flüchtige Speichersitzungen werden nur während der
Laufzeit des Prozesses angezeigt. Die Seite zeigt weder Bearer-Tokens noch
Bearer-Tokens werden nie ausgegeben. Bei älteren Sitzungstabellen ohne
`id`-Spalte dient der gespeicherte Token-Hash als verborgenes Kennzeichen zum
Beenden der Sitzung. Eine optionale Spalte `device_label: String(255)` speichert
eine begrenzte Browser-Angabe für neue Sitzungen; der Wert stammt vom Client,
kann irreführend sein und wird HTML-escaped. IP-Adressen und Browserverläufe
werden nicht gespeichert. Ohne diese Spalte erscheinen keine Geräteangaben.
Der Pfad `/account/sessions` ist bei konfigurierter Authentifizierung reserviert.

Sind dauerhafte Sitzungen mit einer `id`-Spalte und die Rollenverwaltung konfiguriert, sehen
berechtigte Administratoren bis zu 100 noch nicht abgelaufene Sitzungen,
nach Ablauf sortiert. Jede Zeile zeigt die Datenbank-ID, die E-Mail-Adresse
und den Ablaufzeitpunkt in Datenbankzeit. Weder Browser-Token noch dessen
Hash werden angezeigt. Die optionale Browser-Angabe wird nur gezeigt, wenn die
Sitzungstabelle die entsprechende Spalte deklariert. Sie ist keine bestätigte
Geräteidentität; IP-Verläufe werden nicht gespeichert.

Mit **Sitzung sperren** beendest du eine einzelne Sitzung. Der POST benötigt
das konfigurierte Verwaltungsrecht, ein gültiges CSRF-Token und einen
Herkunftsnachweis derselben Origin. Sitzungs-ID und Benutzer-ID müssen
zusammenpassen. Wiederholte Anfragen für bereits entfernte Sitzungen sind
unschädlich. Bei konfiguriertem Audit protokolliert die Transaktion
`auth.session_revoke_requested` einschließlich numerischer Sitzungs-ID.
Das Ereignis belegt die Anfrage, nicht das vorherige Vorhandensein einer Zeile.

Sperrst du die eigene Sitzung, scheitern folgende geschützte Anfragen mit
401. Melde dich erneut an: Das Konto und die Rolle des letzten Administrators
bleiben erhalten. Dauerhafte Sitzungen laufen nach einem Tag ab. Im
Entwicklungszweig sind auch Speichersitzungen auf 24 Stunden begrenzt; sie
verschwinden beim Neustart und erscheinen nicht in dieser Verwaltung
dauerhafter Sitzungen. Der Ablauf wird beim Zugriff geprüft. Eine Sperre
bricht bereits laufende Anfragen nicht ab. Passwortwiederherstellung und
Geräteerkennung sind damit nicht implementiert.

### 6. Merksätze
1. `auth` deklariert Benutzer, Sitzungen und Berechtigungen an einer zentralen Stelle.
2. Geschützte Seiten erfordern `requires auth` und optional `permits "recht"`.
3. Passwörter werden ausschließlich als sichere Argon2-Hashes gespeichert.

### 7. Übungsaufgaben
- **Stufe 1 (Leicht):** Erzeuge ein Passwort-Hash über den CLI-Befehl `zelyra auth hash-password`.
- **Stufe 2 (Mittel):** Schütze eine Seite `/einstellungen` mit `requires auth`.
- **Stufe 3 (Anspruchsvoll):** Vergib einer Rolle ein Berechtigungsrecht über `zelyra auth role-permission`.

### 8. Praxisaufgabe: Geschützte Aufgabenverwaltung
Schütze die Aufgabenverwaltung vor unbefugtem Zugriff:
```zelyra
auth users {
    table: users
    sessions: auth_sessions
    permissions: user_permissions
}

table users {
    id: Id primary auto
    email: Email required unique
    password_hash: String(255) required
    active: Bool default true
}

table auth_sessions {
    id: Id primary auto
    user: User required
    token_hash: String(64) required unique
    expires_at: Timestamp required
    device_label: String(255)
}

table user_permissions {
    id: Id primary auto
    user: User required
    permission: String(100) required
}

page "/meine-aufgaben" {
    requires auth

    html {
        <h1>Geschuetzter Aufgabenbereich</h1>
        <p>Nur fuer authentifizierte Benutzer zugaenglich.</p>
    }
}
```

### 9. Zusammenfassung
- Authentifizierung und Autorisierung sind vollwertige Bestandteile von Zelyra.
- Moderne Sicherheitsstandards (Argon2, Session-Tokens, RBAC) sind ohne Drittanbieter-Bibliotheken einsatzbereit.

### 10. Kontrollfragen zur Selbstprüfung
1. Welcher moderne Hash-Algorithmus wird von Zelyra für Passwörter verwendet?
2. Mit welchem Befehl wird eine Webroute für unbefugte Besucher gesperrt?
3. Wozu dient die Tabelle `auth_sessions`?

---

## Kapitel 30: APIs und Datenaustausch

### 1. Was lerne ich in diesem Kapitel?
- Wie du typisierte REST-Schnittstellen mit dem Schlüsselwort `api` definierst.
- Wie HTTP-Methoden (`GET`, `POST`, `PUT`, `DELETE`) und Eingabe/Ausgabe typisiert werden.
- Wie automatische Fehler-Codes (`404 NotFound`, `400 ValidationError`) deklariert werden.
- Wie Zelyra vollständige OpenAPI/Swagger-Dokumentationen auf Knopfdruck generiert (`zelyra doc --openapi`).

### 2. Warum ist das Thema wichtig?
Moderne Software lebt nicht isoliert: Mobile Apps (iOS/Android), Frontend-Frameworks (Vue, React) oder Partnersysteme müssen mit deinem Backend kommunizieren. Bei herkömmlichen APIs veraltet die Dokumentation oft schon am Tag nach dem Release. In Zelyra ist die API-Definition der Code selbst: Jede Route, jedes Eingabefeld und jeder Fehlercode ist exakt typisiert – und die OpenAPI-Spezifikation wird daraus vollautomatisch abgeleitet.

### 3. Verständliche Erklärung
Mit dem Schlüsselwort `api` deklarierst du einen Web-Endpunkt:
- **Methode und Pfad:** z. B. `GET "/tasks/{id}"`
- **Input:** Welche Parameter erwartet die API?
- **Output:** Welcher Typ wird als JSON zurückgegeben?
- **Errors:** Welche HTTP-Statuscodes können im Fehlerfall auftreten?

```zelyra
type TaskId = Id

table tasks {
    id: TaskId primary auto
    name: String required
}

api GET "/api/tasks/{id}" {
    input {
        id: TaskId
    }
    output Task
    errors {
        404 NotFound
    }
}

fn main() {
    print("API-Endpunkt definiert.")
}
```

### 4. Kleine, aufeinander aufbauende Beispiele

**Beispiel 1: Ein GET-Endpunkt mit Rückgabetyp**
```zelyra
type TaskId = Id

table tasks {
    id: TaskId primary auto
    name: String required
}

api GET "/tasks/{id}" {
    input {
        id: TaskId
    }
    output Task
    errors {
        404 NotFound
    }
}

fn main() {
    print("GET API geprueft.")
}
```

**Beispiel 2: Ein POST-Endpunkt zum Anlegen von Daten**
```zelyra
type TaskId = Id

table tasks {
    id: TaskId primary auto
    name: String(100) required
}

api POST "/tasks" {
    input {
        name: String
    }
    output Task
    errors {
        400 ValidationError
    }
}

fn main() {
    print("POST API geprueft.")
}
```

**Beispiel 3: OpenAPI-Dokumentation erzeugen**
Mit dem Befehl:
```bash
zelyra doc main.zyl --openapi
```
generiert Zelyra eine normgerechte `openapi.json`, die du direkt in Swagger-UI oder Postman importieren kannst.

### 5. Typische Fehler und deren Ursachen
- **Fehler:** Den Ausgabetyp `output` mit einem unbekannten Typen belegen.
  *Ursache:* Zelyra prüft, ob der Datentyp (z. B. `Task`) als Tabelle oder Record existiert.
- **Fehler:** Nicht deklarierte HTTP-Fehlercodes zurücksenden wollen.
  *Ursache:* Deklarierte Schnittstellen fordern vollständige Spezifikation aller Statuscodes.

### 6. Merksätze
1. `api METHOD "/pfad"` deklariert eine typsichere REST-Schnittstelle.
2. `input`, `output` und `errors` beschreiben den Datenvertrag der Schnittstelle lückenlos.
3. `zelyra doc --openapi` erzeugt OpenAPI-Spezifikationen direkt aus dem Quelltext.

### 7. Übungsaufgaben
- **Stufe 1 (Leicht):** Definiere einen Endpunkt `GET "/api/version"`, der einen Versions-String liefert.
- **Stufe 2 (Mittel):** Erstelle einen Endpunkt `DELETE "/api/tasks/{id}"` mit Fehlercode `404 NotFound`.
- **Stufe 3 (Anspruchsvoll):** Generiere mit `zelyra doc --openapi` eine API-Dokumentation und untersuche die JSON-Ausgabe.

### 8. Praxisaufgabe: Die REST-API für unsere Aufgabenverwaltung
Definiere die öffentliche Programmierschnittstelle unserer Aufgabenverwaltung:
```zelyra
type TaskId = Id

table tasks {
    id: TaskId primary auto
    name: String(120) required
    erledigt: Bool
}

api GET "/api/tasks/{id}" {
    input {
        id: TaskId
    }
    output Task
    errors {
        404 NotFound
    }
}

api POST "/api/tasks" {
    input {
        name: String
    }
    output Task
    errors {
        400 ValidationError
    }
}

fn main() {
    print("Aufgaben REST-API einsatzbereit.")
}
```

### 9. Zusammenfassung
- APIs in Zelyra sind typsicher, selbstdokumentierend und standardkonform.
- Mit minimalem Aufwand entstehen robuste Endpunkte für Web- und Mobilanwendungen.

### 10. Kontrollfragen zur Selbstprüfung
1. Welche vier Abschnitte umfasst eine vollständige `api`-Deklaration?
2. Warum ist die automatische OpenAPI-Generierung gegenüber manuellen Dokumentationen überlegen?
3. Was geschieht, wenn ein Client ungültige Daten an einen `input`-Block sendet?

# TEIL VIII – DIE BESONDERHEITEN VON ZELYRA

---

## Kapitel 31: Lesbarkeit als oberstes Gebot

### 1. Was lerne ich in diesem Kapitel?
In diesem Kapitel lernst du:
- Warum Code in der Realität bis zu zehnmal öfter gelesen als geschrieben wird.
- Welche Designentscheidungen Zelyra bewusst getroffen hat, um maximale Klarheit zu schaffen.
- Warum Zelyra auf unlesbare Syntaxakrobatik und kryptische Symbole verzichtet.
- Wie der integrierte Code-Formatierer `zelyra fmt` für einen einheitlichen Standard sorgt.

### 2. Warum ist das Thema wichtig?
Viele Programmiersprachen erlauben es, denselben Sachverhalt auf zehn verschiedene Arten auszudrücken – oft in ultrakompakten Einzeilern mit Sonderzeichen, die nach drei Monaten niemand mehr versteht. In großen Teams und langfristigen Projekten führt das zu enormen Wartungskosten. Zelyra folgt dem Grundsatz: Es gibt genau einen offensichtlichen, klaren Weg, eine Aufgabe zu lösen.

### 3. Verständliche Erklärung
Lesbarkeit in Zelyra bedeutet:
- **Ausdrückliche Namen statt Abkürzungen:** Funktionen und Variablen sprechen Klartext (`prioritaet` statt `prio_lvl_fn()`).
- **Klare Blöcke:** Jede Bedingung, Schleife und Funktion besitzt eindeutige geschweifte Klammern.
- **Automatische Formatierung:** Niemand im Team muss über Einrückungen oder Leerzeichen diskutieren. Der Befehl `zelyra fmt` rückt jede Zeile exakt nach dem einheitlichen Zelyra-Standard ein.

```zelyra
fn berechne_gesamtzeit(aufgaben_dauern: Int[]) -> Int {
    mutable summe = 0
    for dauer in aufgaben_dauern {
        summe = summe + dauer
    }
    return summe
}

fn main() {
    zeiten: Int[] = [15, 30, 45]
    print(berechne_gesamtzeit(zeiten))
}
```

### 4. Kleine, aufeinander aufbauende Beispiele

**Beispiel 1: Selbsterklärende Funktionssignaturen**
```zelyra
fn ist_aufgabe_ueberfaellig(frist_tage: Int) -> Bool {
    return frist_tage < 0
}

fn main() {
    print(ist_aufgabe_ueberfaellig(-2))
}
```

**Beispiel 2: Verständliche Kontrollstrukturen**
```zelyra
fn status_anzeige(status_code: Int) -> String {
    match status_code {
        1 => {
            return "Neu"
        }
        2 => {
            return "In Bearbeitung"
        }
        3 => {
            return "Erledigt"
        }
        _ => {
            return "Unbekannt"
        }
    }
}

fn main() {
    print(status_anzeige(2))
}
```

### 5. Typische Fehler und deren Ursachen
- **Fehler:** Variablen mit einzelnen Buchstaben (`a`, `x`, `tmp`) benennen, deren Bedeutung unklar ist.
  *Ursache:* Zelyra-Code soll wie verständliche Prosa gelesen werden können.
- **Fehler:** Uneinheitliche Einrückungen manuell korrigieren.
  *Ursache:* Führe einfach `zelyra fmt main.zyl` aus – der Compiler formatiert alles automatisch.

### 6. Merksätze
1. Schreibe Code für den Menschen, der ihn in sechs Monaten warten muss.
2. `zelyra fmt` garantiert einen einheitlichen, lesbaren Programmierstil im gesamten Projekt.
3. Klare, sprechende Bezeichner sind die beste Dokumentation.

### 7. Übungsaufgaben
- **Stufe 1 (Leicht):** Formatiere eine unordentliche Quellcodedatei mit `zelyra fmt`.
- **Stufe 2 (Mittel):** Refaktoriere eine Funktion mit unklaren Variablennamen zu sprechendem Zelyra-Code.
- **Stufe 3 (Anspruchsvoll):** Schreibe eine komplexe Berechnungsfunktion so sauber gegliedert, dass keine einzige Zeile mehr als 80 Zeichen benötigt.

### 8. Praxisaufgabe: Lesbare Status- und Filterlogik
Gestalte die Filterlogik unserer Aufgabenverwaltung maximal verständlich:
```zelyra
fn ist_dringend_und_offen(prioritaet: Int, erledigt: Bool) -> Bool {
    ist_prioritaet_eins = prioritaet == 1
    ist_noch_nicht_erledigt = !erledigt
    return ist_prioritaet_eins && ist_noch_nicht_erledigt
}

fn main() {
    print(ist_dringend_und_offen(1, false))
    print(ist_dringend_und_offen(2, false))
}
```

### 9. Zusammenfassung
- Lesbarkeit ist der wichtigste Schutzfaktor gegen schleichende Software-Fäulnis.
- Zelyra erzwingt Klarheit durch Sprachdesign und Werkzeuge.

### 10. Kontrollfragen zur Selbstprüfung
1. Warum spart gut lesbarer Code auf lange Sicht Zeit und Geld?
2. Welcher Zelyra-CLI-Befehl formatiert Quelldateien automatisch?
3. Warum verzichtet Zelyra auf übermäßig viele alternative Schreibweisen für dieselbe Logik?

---

## Kapitel 32: KI-Nativität – Warum Zelyra perfekt für KI-Assistenten ist

### 1. Was lerne ich in diesem Kapitel?
- Was eine „KI-native Programmiersprache“ bedeutet.
- Warum moderne LLMs (wie Claude, GPT, Gemini) bei Zelyra weniger halluzinieren.
- Wie typisierte Löcher (*Typed Holes* `_`) Entwicklern und KIs bei der Codegenerierung helfen.
- Wie CLI-Befehle mit `--format json` strukturierte Schnittstellen für Werkzeuge bieten.
- Wie `zelyra impact` und `zelyra edit` automatisierte Code-Änderungen absichern.

### 2. Warum ist das Thema wichtig?
Die meisten Programmiersprachen wurden vor Jahrzehnten entwickelt – ausschließlich für menschliche Tastatureingaben. Wenn moderne KI-Assistenten Code in Python oder JavaScript schreiben, erfinden sie oft Methoden, verwechseln Typen oder übersehen Seiteneffekte. Zelyra wurde von Grund auf so entworfen, dass menschliche Entwickler und KI-Assistenten optimal zusammenarbeiten können.

### 3. Verständliche Erklärung
Zelyra unterstützt KI-Entwicklung durch vier Schlüsselmerkmale:
1. **Eindeutige, kontextfreie Grammatik:** Die Sprache hat keine Mehrdeutigkeiten.
2. **Maschinenlesbare JSON-Ausgabe:** Fast alle Befehle bieten `--format json` (z. B. `zelyra check --format json`), sodass KIs Fehlermeldungen direkt als strukturierte Daten erfassen.
3. **Typed Holes (`_`):** Wenn du oder die KI nicht genau wissen, wie ein Wert berechnet wird, setzt man einen Unterstrich `_` ein. Der Compiler meldet sofort exakt: Welcher Typ wird erwartet? Welche Variablen sind verfügbar? Welche Verträge gelten?
4. **Auswirkungsanalyse (`zelyra impact`):** Zelyra berechnet vorab genau, welche Programmteile von einer Änderung betroffen sind.

### 4. Kleine, aufeinander aufbauende Beispiele

**Beispiel 1: Typisierte Verträge leiten die KI fehlerfrei**
Durch Verträge weiß die KI ohne Raten, welche Randbedingungen gelten:
```zelyra
fn normalisiere_skala(wert: Int) -> Int
    requires { wert >= 0 && wert <= 100 }
    ensures { result >= 0 && result <= 10 }
{
    return wert / 10
}

fn main() {
    print(normalisiere_skala(85))
}
```

**Beispiel 2: Maschinenlesbare Fehleranalyse**
Führt ein KI-Tool den Befehl:
```bash
zelyra check main.zyl --format json
```
aus, erhält es präzise JSON-Objekte mit exaktem Fehlercode, Zeile, Spalte und Lösungshinweis.

### 5. Typische Fehler und deren Ursachen
- **Fehler:** Typed Holes (`_`) in Produktionscode belassen.
  *Ursache:* Typed Holes sind Entwicklungshilfen. Vor dem finalen Kompilieren müssen alle `_` durch gültigen Code ersetzt werden.
- **Fehler:** KI-Code ohne `zelyra check` ungeprüft übernehmen.
  *Ursache:* Nutze immer den Zelyra-Compiler als unbestechlichen Richter.

### 6. Merksätze
1. Zelyra ist die erste KI-native Sprache: Eindeutig, strukturiert und werkzeugfreundlich.
2. Typed Holes `_` dienen als präzise Arbeitsaufträge an den Compiler und KI-Assistenten.
3. `--format json` ermöglicht nahtlose Integration in moderne KI-Agenten und IDEs.

### 7. Übungsaufgaben
- **Stufe 1 (Leicht):** Führe `zelyra check` mit der Option `--format json` aus und studiere die Ausgabe.
- **Stufe 2 (Mittel):** Analysiere eine Datei mit `zelyra impact`.
- **Stufe 3 (Anspruchsvoll):** Setze in einer Funktion ein Typed Hole `_` ein und beobachte die detaillierten Kontextinformationen des Compilers.

### 8. Praxisaufgabe: KI-unterstützte Erweiterung der Aufgabenverwaltung
Schreibe eine wohlstrukturierte Funktionssignatur mit Vertrag, die sich perfekt von einer KI vervollständigen lässt:
```zelyra
fn berechne_restzeit(ziel_stunde: Int, aktuelle_stunde: Int) -> Int
    requires { ziel_stunde >= aktuelle_stunde }
    ensures { result >= 0 }
{
    return ziel_stunde - aktuelle_stunde
}

fn main() {
    rest = berechne_restzeit(18, 14)
    print(rest)
}
```

### 9. Zusammenfassung
- Zelyra beseitigt Sprach-Mehrdeutigkeiten, die KI-Systeme traditionell verwirren.
- Typed Holes und strukturierte JSON-Ausgaben machen Pair-Programming mit KIs extrem zuverlässig.

### 10. Kontrollfragen zur Selbstprüfung
1. Wofür steht das Konzept der „Typed Holes“ in Zelyra?
2. Warum profitieren KI-Systeme von der `--format json`-Option des Compilers?
3. Wie helfen Verträge (`requires`, `ensures`) einer KI beim Erzeugen von korrektem Code?

---

## Kapitel 33: Sicherheit durch Fähigkeiten (Capabilities)

### 1. Was lerne ich in diesem Kapitel?
- Was das Capability-Sicherheitsmodell ist und warum es herkömmlichen Berechtigungskonzepten überlegen ist.
- System-Capabilities wie `Console`, `Database`, `Network`, `FileSystem`, `Process`, `Environment`, `Clock` und `Random`.
- Wie Capabilities deklariert, vererbt und in `zelyra.toml` beschränkt werden.
- Warum Zelyra gegen Supply-Chain-Angriffe (bösartige Pakete) immun ist.

### 2. Warum ist das Thema wichtig?
In heutigen Ökosystemen wie npm (JavaScript) oder PyPI (Python) bindet man oft hunderte Bibliotheken von Drittanbietern ein. Wenn ein Paket manipuliert wird, kann es unbemerkt Passwörter auslesen, Dateien verschlüsseln oder Daten ins Internet funken. In Zelyra ist das unmöglich: Eine Funktion kann ohne ausdrückliche Deklaration von `uses Network` kein einziges Byte übers Netz senden – der Compiler verweigert den Dienst!

### 3. Verständliche Erklärung
Stell dir Zelyras Capability-System wie ein Sicherheitsschloss vor:
- Wenn eine Funktion auf die Festplatte schreiben will, muss sie den Schlüssel `uses FileSystem` am Revers tragen.
- Hat sie diesen Schlüssel nicht, kann sie keine Dateien anrühren – selbst wenn sie es versucht.
- Und das Beste: Wenn Funktion A die Funktion B aufruft, muss auch A die Berechtigung deklarieren. So siehst du auf den ersten Blick in `main()`, was das gesamte Programm überhaupt darf!

```zelyra
fn sichere_operation() {
    // Diese Funktion hat KEINE Capabilities.
    // Sie kann unmoeglich Schaden auf der Festplatte oder im Netzwerk anrichten!
    print("Garantiert seiteneffektfrei.")
}

fn main() {
    sichere_operation()
}
```

### 4. Kleine, aufeinander aufbauende Beispiele

**Beispiel 1: Saubere Deklaration von Umgebungsvariablen**
```zelyra
fn lese_konfiguration(schluessel: String) -> Option<String>
    uses Environment
{
    return env(schluessel)
}

fn main() uses Environment {
    print("Konfigurationszugriff erlaubt.")
}
```

**Beispiel 2: Datenbankzugriff mit Capability**
```zelyra
database main {
    engine: mariadb
    database: "tasks_db"
}

table tasks {
    id: Id primary auto
    name: String required
}

fn lade_daten() uses Database {
    daten = sql<Task[]> {
        SELECT id, name
        FROM tasks
    }
    print("Datenbankzugriff gewaehrt.")
}

fn main() uses Database {
    lade_daten()
}
```

### 5. Typische Fehler und deren Ursachen
- **Fehler:** Vergessen, Capabilities an den Aufrufer weiterzureichen.
  *Ursache:* Wenn Funktion `a()` die Funktion `b() uses FileSystem` aufruft, muss auch `a()` mit `uses FileSystem` versehen sein.
- **Fehler:** Fehlende Freigabe in `zelyra.toml`.
  *Ursache:* Das Projekt manifestiert seine maximalen Rechte in der Konfiguration.

### 6. Merksätze
1. Keine Funktion kann heimlich auf Dateien, Datenbanken oder das Netzwerk zugreifen.
2. Alle Seiteneffekte sind transparent in der Funktionssignatur dokumentiert.
3. Reine Funktionen ohne Capabilities sind garantiert manipulationssicher.

### 7. Übungsaufgaben
- **Stufe 1 (Leicht):** Identifiziere in einem bestehenden Code alle Funktionen mit Capabilities.
- **Stufe 2 (Mittel):** Schreibe eine Funktion, die `uses Clock` und `uses Environment` kombiniert.
- **Stufe 3 (Anspruchsvoll):** Entwirf ein Programm so, dass alle Geschäftsrechenlogik in Funktionen völlig ohne Capabilities ausgelagert wird.

### 8. Praxisaufgabe: Berechtigungs-Architektur der Aufgabenverwaltung
Isoliere die Berechtigungen in unserer Aufgabenverwaltung:
```zelyra
// 1. Reine Logik: KEINE Berechtigungen noetig
fn ist_bereit_fuer_export(anzahl_aufgaben: Int) -> Bool {
    return anzahl_aufgaben > 0
}

// 2. I/O-Logik: Explizite Dateisystem-Berechtigung
fn fuehre_export_durch(datei: String, inhalt: String) uses FileSystem {
    write_text(datei, inhalt)
    print("Export vollzogen.")
}

fn main() uses FileSystem {
    if ist_bereit_fuer_export(5) {
        fuehre_export_durch("aufgaben.txt", "Aufgabe 1")
    }
}
```

### 9. Zusammenfassung
- Das Capability-Modell von Zelyra schützt vor bösartigen Bibliotheken und unkontrollierten Seiteneffekten.
- Software wird durch explizite Rechtevergabe von Grund auf sicher (*Secure by Design*).

### 10. Kontrollfragen zur Selbstprüfung
1. Nenne drei System-Capabilities von Zelyra.
2. Warum müssen auch übergeordnete Aufrufer-Funktionen Capabilities deklarieren?
3. Wie schützt Zelyra vor schadhaftem Fremdcode aus Paketmanagern?

---

## Kapitel 34: Zelyra im Vergleich

### 1. Was lerne ich in diesem Kapitel?
- Wie sich Zelyra im direkten Vergleich zu Python, PHP/Laravel, Rust und TypeScript schlägt.
- Welche Stärken die jeweiligen Sprachen haben und warum Zelyra für moderne Web- & KI-Systeme maßgeschneidert wurde.
- Warum Zelyra die Typsicherheit von Rust mit der Entwicklungsgeschwindigkeit von Python verbindet.

### 2. Warum ist das Thema wichtig?
Keine Programmiersprache ist für jeden Zweck perfekt: C und Rust sind unschlagbar für Betriebssystemkerne, Python dominiert die Datenwissenschaft, JavaScript das Frontend. Wer aber moderne, datenbankgestützte Geschäftsanwendungen und Web-Backends bauen will, kämpft in diesen Sprachen oft mit historischem Ballast. Zelyra vereint das Beste aus diesen Welten.

### 3. Verständliche Erklärung: Der Sprachvergleich

| Merkmal | Python | PHP / Laravel | TypeScript / Node | Rust | Zelyra |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **Typisierung** | Dynamisch | Dynamisch/Optional | Statisch (wird zu JS) | Statisch (sehr streng) | **Statisch & Eindeutig** |
| **Null-Sicherheit** | `None`-Crashes möglich | `null`-Crashes möglich | `undefined`-Crashes | Absolut (`Option`) | **Absolut (`Option`)** |
| **SQL-Integration** | ORM (Strings) | ORM (Eloquent) | ORM (Prisma/TypeORM) | Diesel/SQLx | **Nativ & Typgeprüft** |
| **Verträge (Contracts)**| Nein (nur `assert`) | Nein | Nein | Dritt-Bibliotheken | **Eingebaut (`requires`)** |
| **Sicherheits-Capabilities**| Nein (Vollzugriff) | Nein (Vollzugriff) | Nein (Vollzugriff) | Nein | **Eingebaut (`uses ...`)** |
| **KI-Werkzeugunterstützung**| Mittel (Mehrdeutig) | Mittel | Mittel | Schwer für KIs | **Nativ (JSON, Typed Holes)** |

### 4. Kleine, aufeinander aufbauende Beispiele

**Vergleich: Wie Zelyra Fehler verhindert, die in anderen Sprachen passieren**

*In Python/JS (potenzieller Laufzeitabsturz bei `null`):*
Ein unbemerkter fehlender Wert führt zum Servercrash: `AttributeError: 'NoneType' object has no attribute 'title'`.

*In Zelyra (garantiert abgefangen zur Compilezeit):*
```zelyra
fn zeige_titel(opt_titel: Option<String>) {
    match opt_titel {
        Some(t) => {
            print("Titel: " + t)
        }
        None => {
            print("Kein Titel vorhanden.")
        }
    }
}

fn main() {
    zeige_titel(Some("Projekt X"))
    zeige_titel(None)
}
```

### 5. Typische Fehler und deren Ursachen
- **Fehler:** Zelyra wie Python schreiben wollen (Einrückung statt Klammern, dynamische Typänderung).
  *Ursache:* Zelyra nutzt geschweifte Klammern und verlangt statische Typstabilität.
- **Fehler:** Zelyra mit Low-Level-Rust verwechseln (komplexe Lifetime-Annotationen suchen).
  *Ursache:* Zelyra nimmt Entwicklern die Speicherverwaltung vollautomatisch ab.

### 6. Merksätze
1. Zelyra vereint die Einfachheit von Skriptsprachen mit der Unbestechlichkeit statischer Typsysteme.
2. Datenbanken, Webformulare und Schnittstellen sind native Sprachbausteine statt externer Bibliotheken.
3. Sicherheit und Korrektheit werden nicht nachträglich hineingetestet, sondern von Anfang an erzwungen.

### 7. Übungsaufgaben
- **Stufe 1 (Leicht):** Übersetze eine einfache Python-Berechnungsfunktion in sauberen Zelyra-Code.
- **Stufe 2 (Mittel):** Vergleiche eine Datenbankabfrage in PHP/Laravel Eloquent mit Zelyras `sql<T[]>`.
- **Stufe 3 (Anspruchsvoll):** Diskutiere anhand eines Praxisbeispiels, warum Zelyras Capabilities Supply-Chain-Angriffe verhindern.

### 8. Praxisaufgabe: Die Aufgabenverwaltung als Zelyra-Vorzeigeprojekt
Führe alle Kernstärken in einem prägnanten Ausschnitt zusammen:
```zelyra
database main {
    engine: mariadb
    database: "tasks_demo"
}

table tasks {
    id: Id primary auto
    name: String(100) required
    erledigt: Bool default false
}

fn zaehle_offene() -> Int uses Database {
    offene = sql<Task[]> {
        SELECT id, name, erledigt
        FROM tasks
        WHERE erledigt = false
    }
    return len(offene)
}

fn main() uses Database {
    anzahl = zaehle_offene()
    print("Offene Aufgaben ermittelt.")
}
```

### 9. Zusammenfassung
- Zelyra schließt die Lücke zwischen zu komplexen System-Sprachen und zu fehleranfälligen Skriptsprachen.
- Für moderne Web-, Daten- und KI-Anwendungen bietet Zelyra eine unübertroffen robuste Plattform.

### 10. Kontrollfragen zur Selbstprüfung
1. Welchen Vorteil bietet Zelyras `Option`-Typ gegenüber Pythons `None` oder JavaScripts `null`?
2. Warum ist die native Datenbankintegration in Zelyra sicherer als traditionelle ORM-Bibliotheken?
3. Welche Rolle spielen Capabilities beim Schutz vor bösartigen Fremdpaketen?

# TEIL IX – VOM ENTWURF ZUR FERTIGEN ANWENDUNG

---

## Kapitel 35: Software planen – Von der Idee zum Entwurf

### 1. Was lerne ich in diesem Kapitel?
In diesem Kapitel lernst du:
- Wie du von einer vagen Idee zu einem präzisen, umsetzbaren Software-Entwurf gelangst.
- Wie man Entitäten und deren Beziehungen auf Papier oder im Editor skizziert.
- Warum die frühe Definition des Schemas in Zelyra den gesamten weiteren Entwicklungsverlauf vereinfacht.
- Wie man Anforderungen in kleine, testbare Meilensteine zerlegt.

### 2. Warum ist das Thema wichtig?
Der größte Fehler von Anfängern (und unvorsichtigen Profis) ist es, sofort loszutippen, ohne den Datenfluss zu planen. Wer während des Programmierens merkt, dass ein zentrales Tabellenfeld oder eine Beziehung fehlt, muss oft Tage damit verbringen, bestehenden Code mühsam umzuschreiben. Zelyra belohnt gründliche Planung: Sobald dein `table`-Schema steht, leiten sich Formulare, Validierungen und APIs fast wie von selbst ab.

### 3. Verständliche Erklärung
Jedes gute Softwareprojekt durchläuft vier Planungsphasen:
1. **Zweck und Zielgruppe klären:** Wer nutzt das System? Welche Kernaufgabe muss gelöst werden? (z. B. „Ein Teamleiter möchte Aufgaben anlegen, zuweisen und als erledigt markieren“).
2. **Datenmodell entwerfen:** Welche Objekte gibt es? Welche Felder sind Pflicht? (z. B. `tasks` mit `name`, `prioritaet`, `ist_erledigt`).
3. **Sicherheits- und Zugriffsregeln:** Wer darf was tun? Benötigen wir Authentifizierung?
4. **Schrittweise Implementierung:** Erst das Schema (`table`), dann die Logik (`fn`), dann die Webansichten (`crud`/`page`).

### 4. Kleine, aufeinander aufbauende Beispiele

**Beispiel 1: Der erste Meilenstein – Das Datenmodell**
```zelyra
database main {
    engine: mariadb
    database: "aufgaben_planer"
}

table tasks {
    id: Id primary auto
    name: String(100) required
    prioritaet: Int default 2
    erledigt: Bool default false
}

fn main() {
    print("Planungsschritt 1: Schema steht.")
}
```

**Beispiel 2: Planung der Geschäftslogik als reine Funktionen**
```zelyra
fn validiere_frist(tage: Int) -> Bool {
    return tage >= 0
}

fn main() {
    print(validiere_frist(3))
}
```

### 5. Typische Fehler und deren Ursachen
- **Fehler:** Alles auf einmal programmieren wollen, bevor die Grundlagen getestet sind.
  *Ursache:* Baue Software schrittweise: Teste jede Funktion sofort mit `zelyra check`.
- **Fehler:** Unklare Pflichtfelder im Datenmodell.
  *Ursache:* Lege von Anfang an fest, welche Felder `required` sind und welche leer sein dürfen (`Option`).

### 6. Merksätze
1. Wer die Planung vernachlässigt, plant das Scheitern.
2. Ein klares Datenmodell ist das Rückgrat jeder erfolgreichen Anwendung.
3. Zerlege große Probleme in kleine, unabhängig überprüfbare Funktionen.

### 7. Übungsaufgaben
- **Stufe 1 (Leicht):** Skizziere den Funktionsumfang einer einfachen Notiz-App in Stichpunkten.
- **Stufe 2 (Mittel):** Entwirf ein Tabellenschema für Benutzer, Aufgaben und Kategorien mit passenden Datentypen.
- **Stufe 3 (Anspruchsvoll):** Formuliere für alle Kernfunktionen deiner geplanten App Vor- und Nachbedingungen (`requires`, `ensures`).

### 8. Praxisaufgabe: Der vollständige Architekturplan unserer Aufgabenverwaltung
Führe alle Planungselemente der Aufgabenverwaltung zusammen:
```zelyra
database main {
    engine: mariadb
    database: "tasks_pro"
}

table tasks {
    id: Id primary auto
    name: String(120) required
    prioritaet: Int default 1
    erledigt: Bool default false
}

fn berechne_dringlichkeit(prioritaet: Int, verbleibende_tage: Int) -> String {
    if prioritaet == 1 {
        return "HOECHSTE PRIORITAET"
    }
    if verbleibende_tage <= 1 {
        return "DRINGEND WEGEN FRIST"
    }
    return "NORMAL"
}

fn main() {
    status = berechne_dringlichkeit(1, 5)
    print("Architektur-Plan verifiziert: " + status)
}
```

### 9. Zusammenfassung
- Strukturierte Planung spart Entwicklungszeit und verhindert Architekturfehler.
- Zelyras deklarative Sprachstruktur passt sich nahtlos an agile Planungsphasen an.

### 10. Kontrollfragen zur Selbstprüfung
1. Welche vier Phasen durchläuft ein professioneller Software-Entwurf?
2. Warum sollte das Datenmodell vor der Benutzeroberfläche entworfen werden?
3. Wie helfen Vorbedingungen bei der präzisen Anforderungsdefinition?

---

## Kapitel 36: Architektur und saubere Codestruktur

### 1. Was lerne ich in diesem Kapitel?
- Wie du deinen Zelyra-Code nach dem bewährten Schichtenmodell strukturierst.
- Die klare Trennung von Persistenz (`table`), Geschäftslogik (`fn`) und Darstellung (`page`, `crud`).
- Wie du Kopplungen vermeidest und Module wartungsfreundlich hältst.
- Warum saubere Architektur vor bösen Überraschungen bei späteren Erweiterungen schützt.

### 2. Warum ist das Thema wichtig?
Wenn Datenzugriff, Geschäftsregeln und HTML-Ausgabe wild durcheinandergewürfelt werden, entsteht unwartbarer Code. Wenn sich später das Datenbanklayout ändert, zerbricht plötzlich die Weboberfläche. Eine saubere Architektur zieht klare Trennlinien: Jede Schicht hat eine einzige Verantwortlichkeit.

### 3. Verständliche Erklärung
Eine saubere Zelyra-Anwendung gliedert sich in drei klare Schichten:
1. **Daten- und Persistenzschicht:** Tabellendefinitionen (`table`) und typisierte SQL-Abfragen.
2. **Geschäftslogikschicht:** Reine Rechen- und Validierungsfunktionen mit Verträgen (`requires`, `ensures`).
3. **Präsentations- und Schnittstellenschicht:** Weboberflächen (`crud`, `page`) und REST-Endpunkte (`api`).

```zelyra
// 1. Datenmodell
table tasks {
    id: Id primary auto
    name: String required
}

// 2. Geschäftslogik
fn formatiere_name(rohtext: String) -> String {
    return "[AUFGABE] " + rohtext
}

// 3. Einstieg / Ablauf
fn main() {
    print(formatiere_name("Server pruefen"))
}
```

### 4. Kleine, aufeinander aufbauende Beispiele

**Beispiel 1: Trennung von Logik und I/O**
```zelyra
// Reine Rechenfunktion: Keine Capabilities noetig
fn berechne_prozent(wert: Int, max_wert: Int) -> Int
    requires { max_wert > 0 && wert >= 0 }
{
    return (wert * 100) / max_wert
}

// I/O-Funktion: Nutzt die Logik und gibt sie aus
fn main() {
    prozent = berechne_prozent(45, 50)
    print(prozent)
}
```

**Beispiel 2: Strukturierung durch aussagekräftige Namen**
```zelyra
table settings {
    id: Id primary auto
    app_name: String(60) required
}

fn zeige_systeminfo(name: String) {
    print("System laeuft: " + name)
}

fn main() {
    zeige_systeminfo("Zelyra Task Suite")
}
```

### 5. Typische Fehler und deren Ursachen
- **Fehler:** Geschäftslogik direkt in SQL-Strings oder HTML-Blöcke stopfen.
  *Ursache:* Trenne Berechnungen in eigene Hilfsfunktionen aus, damit sie unabhängig testbar bleiben.
- **Fehler:** Zirkuläre Abhängigkeiten erzeugen.
  *Ursache:* Der Datenfluss sollte immer von oben nach unten verlaufen (Präsentation -> Logik -> Daten).

### 6. Merksätze
1. Trenne Datenmodell, Geschäftsregeln und Darstellung strikt voneinander.
2. Geschäftslogik sollte möglichst frei von Seiteneffekten und Capabilities sein.
3. Saubere Schichten machen Anwendungen zukunftssicher und einfach erweiterbar.

### 7. Übungsaufgaben
- **Stufe 1 (Leicht):** Identifiziere in einem bestehenden Codebeispiel die drei Schichten.
- **Stufe 2 (Mittel):** Lagere alle Berechnungen aus einer Web-Route in separate reine Funktionen aus.
- **Stufe 3 (Anspruchsvoll):** Entwirf ein Schichtenmodell für ein Zeiterfassungssystem.

### 8. Praxisaufgabe: Schichtenarchitektur für die Aufgabenverwaltung
Implementiere das Schichtenmodell für unsere Aufgabenverwaltung:
```zelyra
database main {
    engine: mariadb
    database: "tasks_architecture"
}

// Schicht 1: Persistenz
table tasks {
    id: Id primary auto
    name: String required
    erledigt: Bool default false
}

// Schicht 2: Geschaeftslogik
fn ist_aufgabe_wichtig(name: String, dringend: Bool) -> Bool {
    return dringend
}

// Schicht 3: Anwendung / Ausfuehrung
fn main() uses Database {
    wichtig = ist_aufgabe_wichtig("Steuern einreichen", true)
    print("Aufgaben-Architektur geprueft.")
}
```

### 9. Zusammenfassung
- Das Drei-Schichten-Modell garantiert Übersichtlichkeit und langfristige Wartbarkeit.
- Zelyra unterstützt diese Struktur auf natürliche Weise durch sein klares Typsystem.

### 10. Kontrollfragen zur Selbstprüfung
1. Welche drei Schichten bilden das Fundament einer sauberen Zelyra-Anwendung?
2. Warum sollte Kern-Geschäftslogik möglichst ohne Capabilities auskommen?
3. Welche Vorteile bietet die Trennung von Präsentation und Datenzugriff bei Re-Designs?

---

## Kapitel 37: Konfiguration und Umgebungsvariablen

### 1. Was lerne ich in diesem Kapitel?
- Wie Konfigurationswerte sicher über Umgebungsvariablen (`.env`) verwaltet werden.
- Die Bibliotheksfunktion `env(schluessel)` und die benötigte Fähigkeit `uses Environment`.
- Wie du mit dem `Option<String>`-Rückgabewert von `env()` sicher umgehst.
- Warum Passwörter und API-Schlüssel niemals im Quelltext stehen dürfen.

### 2. Warum ist das Thema wichtig?
Einer der schwersten Sicherheitsverstöße ist das Versehentliche Einchecken von Datenbank-Passwörtern oder geheimen API-Keys in öffentliche Git-Repositories. Zudem muss sich eine Anwendung in Entwicklung, Test und Produktion unterschiedlich verhalten (z. B. andere Datenbank-Hosts). Umgebungsvariablen trennen Code und geheime Konfiguration sauber voneinander.

### 3. Verständliche Erklärung
In Zelyra greifst du über die Funktion `env()` auf Umgebungsvariablen zu.
Weil eine Variable in der Umgebung existieren kann oder fehlen kann, liefert `env()` immer ein `Option<String>` zurück:

```zelyra
fn lese_port() -> String uses Environment {
    opt_port = env("APP_PORT")
    match opt_port {
        Some(p) => {
            return p
        }
        None => {
            return "8080"
        }
    }
}

fn main() uses Environment {
    port = lese_port()
    print("Server lauscht auf Port: " + port)
}
```

### 4. Kleine, aufeinander aufbauende Beispiele

**Beispiel 1: Datenbank-Host konfigurieren**
```zelyra
fn hole_db_host() -> String uses Environment {
    match env("DB_HOST") {
        Some(host) => {
            return host
        }
        None => {
            return "127.0.0.1"
        }
    }
}

fn main() uses Environment {
    print("Verbinde mit: " + hole_db_host())
}
```

**Beispiel 2: Debug-Modus dynamisch abfragen**
```zelyra
fn ist_debug_aktiv() -> Bool uses Environment {
    match env("APP_DEBUG") {
        Some(wert) => {
            return wert == "true"
        }
        None => {
            return false
        }
    }
}

fn main() uses Environment {
    if ist_debug_aktiv() {
        print("Debug-Modus ist AN")
    } else {
        print("Debug-Modus ist AUS")
    }
}
```

### 5. Typische Fehler und deren Ursachen
- **Fehler:** Geheime Passwörter fest in `.zyl`-Dateien hineinschreiben.
  *Ursache:* Nutze immer eine `.env`-Datei und lese sensible Werte mit `env()`.
- **Fehler:** `env()` ohne `uses Environment` aufrufen.
  *Ursache:* Zelyras Sicherheitssystem schützt Systemumgebungen vor unbefugtem Zugriff.

### 6. Merksätze
1. Sensible Zugangsdaten gehören niemals in den Versionskontroll-Quelltext.
2. `env(name)` liefert ein `Option<String>` und erfordert `uses Environment`.
3. Stelle für fehlende Umgebungsvariablen immer sichere Standardwerte bereit.

### 7. Übungsaufgaben
- **Stufe 1 (Leicht):** Lese eine Umgebungsvariable `USER_NAME` aus und gib eine persönliche Begrüßung aus.
- **Stufe 2 (Mittel):** Schreibe eine Hilfsfunktion `hole_env_oder_standard(schluessel: String, standard: String) -> String`.
- **Stufe 3 (Anspruchsvoll):** Konfiguriere ein Programm so, dass es zwischen Entwicklungs- und Produktions-Modus umschaltet.

### 8. Praxisaufgabe: Konfigurationszentrale für die Aufgabenverwaltung
Schreibe den Konfigurations-Loader für unsere Aufgabenverwaltung:
```zelyra
fn lade_app_titel() -> String uses Environment {
    match env("APP_TITLE") {
        Some(titel) => {
            return titel
        }
        None => {
            return "Zelyra Aufgabenverwaltung 0.1"
        }
    }
}

fn main() uses Environment {
    print("System gestartet: " + lade_app_titel())
}
```

### 9. Zusammenfassung
- Umgebungsvariablen ermöglichen flexible, sichere Konfiguration für verschiedene Serverumgebungen.
- Zelyras `Option`-Typ zwingt dich, das Fehlen von Einstellungen elegant zu behandeln.

### 10. Kontrollfragen zur Selbstprüfung
1. Warum dürfen geheime API-Schlüssel niemals im Quelltext fest hinterlegt sein?
2. Welchen Datentyp liefert die Funktion `env()` zurück?
3. Welche Capability wird für den Zugriff auf Umgebungsvariablen benötigt?

---

## Kapitel 38: Fehlersuche und Optimierung

### 1. Was lerne ich in diesem Kapitel?
- Wie du mit dem CLI-Befehl `zelyra doctor` dein Projekt auf Herz und Nieren prüfst.
- Wie man Compilerhinweise und Diagnosen effektiv nutzt.
- Strategien zum systematischen Aufspüren von Fehlern (*Debugging*).
- Wie du Performance-Engpässe erkennst und eliminierst.

### 2. Warum ist das Thema wichtig?
Selbst bei sorgfältigster Programmierung läuft nicht immer alles auf Anhieb glatt. Vielleicht ist der Datenbankport blockiert, eine Konfiguration unvollständig oder eine Schleife berechnet unnötige Schritte. Wer planlos herumprobiert, verliert Stunden. Systematisches Debugging mit den passenden Zelyra-Werkzeugen führt dagegen in wenigen Minuten zur Lösung.

### 3. Verständliche Erklärung
Zelyra gibt dir ein Schweizer Taschenmesser für die Fehlerdiagnose an die Hand:
- `zelyra check`: Prüft Syntax, Typen, Berechtigungen und Verträge.
- `zelyra doctor`: Prüft die Systemumgebung, Datenbankverbindungen, Ports und Richtlinien.
- `zelyra impact`: Zeigt an, welche Funktionen durch eine geplante Änderung beeinflusst werden.

```bash
zelyra doctor main.zyl
```
Wenn die MariaDB-Verbindung nicht erreichbar ist, meldet `doctor` sofort die Ursache, anstatt dich im Unklaren zu lassen.

### 4. Kleine, aufeinander aufbauende Beispiele

**Beispiel 1: Systematische Kontrollausgaben**
```zelyra
fn berechne_summe(zahlen: Int[]) -> Int {
    mutable summe = 0
    for z in zahlen {
        summe = summe + z
    }
    return summe
}

fn main() {
    werte: Int[] = [10, 20, 30]
    ergebnis = berechne_summe(werte)
    print("Berechnetes Ergebnis:")
    print(ergebnis)
}
```

**Beispiel 2: Absicherung vor Endlosschleifen durch Invarianten**
```zelyra
fn sichere_zaehlung(grenze: Int) -> Int
    requires { grenze > 0 }
{
    mutable i = 0
    while i < grenze
        invariant { i >= 0 }
    {
        i = i + 1
    }
    return i
}

fn main() {
    print(sichere_zaehlung(10))
}
```

### 5. Typische Fehler und deren Ursachen
- **Fehler:** Bei unerwartetem Verhalten wahllos Zeilen im Code ändern.
  *Ursache:* Lokalisiere das Problem erst exakt mit `zelyra check` und Ausgaben.
- **Fehler:** Datenbankfehler vermuten, wenn lediglich Berechtigungen in `zelyra.toml` fehlen.
  *Ursache:* Führe `zelyra doctor` aus, um Umgebungsfehler sofort zu erkennen.

### 6. Merksätze
1. `zelyra doctor` ist der erste Schritt bei Verbindungsproblemen und Umgebungsfehlern.
2. Invarianten und Verträge verhindern logische Fehlberechnungen.
3. Systematische Fehlersuche ist schneller und sicherer als blindes Ausprobieren.

### 7. Übungsaufgaben
- **Stufe 1 (Leicht):** Führe `zelyra doctor` für dein Aufgaben-Projekt aus.
- **Stufe 2 (Mittel):** Isoliere eine bewusst fehlerhafte Berechnung in einer Testfunktion.
- **Stufe 3 (Anspruchsvoll):** Schreibe ein Programm mit detaillierten Statusmeldungen für jeden Einzelschritt.

### 8. Praxisaufgabe: Selbstdiagnose-Routine der Aufgabenverwaltung
Baue einen internen Gesundheits-Check für die Aufgabenverwaltung:
```zelyra
fn fuehre_selbsttest_durch() -> Bool {
    test_ok = 1 + 1 == 2
    return test_ok
}

fn main() {
    print("Starte System-Selbsttest...")
    if fuehre_selbsttest_durch() {
        print("[OK] System arbeitet einwandfrei.")
    } else {
        print("[FEHLER] Interner Systemfehler.")
    }
}
```

### 9. Zusammenfassung
- Zelyras Werkzeugkette (`doctor`, `check`, `impact`) liefert schnelle Klarheit bei Fehlern.
- Verträge und Invarianten fangen Probleme ab, bevor sie zu schwer auffindbaren Bugs werden.

### 10. Kontrollfragen zur Selbstprüfung
1. Welche Aspekte überprüft der CLI-Befehl `zelyra doctor`?
2. Wie grenzt man einen Fehler in einer längeren Berechnung am besten ein?
3. Welche Rolle spielen Schleifeninvarianten bei der Fehlersuche?

---

## Kapitel 39: Bereitstellung und Betrieb

### 1. Was lerne ich in diesem Kapitel?
- Wie Zelyra-Anwendungen für den Produktivbetrieb bereitgestellt (*deployed*) werden.
- Wie du deine Webanwendung im Docker-Container betreibst.
- Wie Zelyra mit einer Produktions-MariaDB verbunden wird.
- Wie du den Server-Prozess mit `zelyra serve` zuverlässig am Laufen hältst.

### 2. Warum ist das Thema wichtig?
Eine Software nützt niemandem, wenn sie nur auf dem Entwickler-Laptop läuft. Sie muss auf einem Server oder in der Cloud rund um die Uhr stabil erreichbar sein. In anderen Umgebungen erfordert das Deployment oft komplizierte Anleitungen mit PHP-FPM, Webserver-Vhosts und Prozessmanagern. Zelyra vereinfacht den Betrieb radikal: Eine einzelne Konfiguration und ein schlanker Container genügen.

### 3. Verständliche Erklärung
Zelyra bringt seinen eigenen Hochleistungs-Webserver direkt mit:
```bash
zelyra serve src/main.zyl 0.0.0.0:8080
```
Für den professionellen Betrieb verpackst du dein Projekt in einen Docker-Container:
- Der Container enthält das Zelyra-Binary, deine Quelldateien und `zelyra.toml`.
- Beim Start führt der Container automatisch `zelyra db apply` aus und startet den Webserver.

### 4. Kleine, aufeinander aufbauende Beispiele

**Beispiel 1: Produktionsreife Projektstruktur**
```toml
# zelyra.toml
[package]
name = "aufgaben_produktion"
version = "1.0.0"

[capabilities]
database = true
filesystem = false
network = true
```

**Beispiel 2: Saubere Hauptdatei für den Webbetrieb**
```zelyra
database main {
    engine: mariadb
    database: "tasks_prod"
}

table tasks {
    id: Id primary auto
    name: String(120) required
    fertig: Bool default false
}

page "/" {
    html {
        <h1>Zelyra Aufgabenverwaltung live</h1>
        <p>Produktivsystem aktiv und sicher.</p>
    }
}
```

**Beispiel 3: Startbefehl für den Server**
Auf dem Produktionsserver genügt:
```bash
zelyra db apply src/main.zyl
zelyra serve src/main.zyl 0.0.0.0:80
```

### 5. Typische Fehler und deren Ursachen
- **Fehler:** Vergessen, die Datenbankmigrationen (`zelyra db apply`) vor dem Serverstart auszuführen.
  *Ursache:* Neue Tabellen und Spalten müssen in der Datenbank existieren, bevor Anfragen eingehen.
- **Fehler:** In Docker den Port `8080` nicht nach außen freigeben.
  *Ursache:* Nutze im `docker run`-Befehl das Port-Mapping `-p 8080:8080`.

### 6. Merksätze
1. `zelyra serve` startet den integrierten HTTP-Server ohne externe Webserver-Abhängigkeiten.
2. `zelyra db apply` bringt das Produktivschema sicher auf den neuesten Stand.
3. Klare Capability-Beschränkungen in `zelyra.toml` sichern den Server vor Angriffen ab.

### 7. Übungsaufgaben
- **Stufe 1 (Leicht):** Starte deine Webanwendung lokal auf Port 3000 mit `zelyra serve main.zyl 127.0.0.1:3000`.
- **Stufe 2 (Mittel):** Erstelle eine `docker-compose.yml`, die MariaDB und deine Zelyra-App verbindet.
- **Stufe 3 (Anspruchsvoll):** Simuliere ein Update mit einer Schema-Änderung und führe `zelyra db plan` und `zelyra db apply` aus.

### 8. Praxisaufgabe: Das produktionsfertige Aufgaben-Paket
Führe alle Einstellungen für das finale Deployment zusammen:
```zelyra
database main {
    engine: mariadb
    database: "tasks_production"
}

table tasks {
    id: Id primary auto
    name: String(100) required
    erledigt: Bool default false
}

page "/" {
    html {
        <html>
            <body>
                <h1>Aufgabenverwaltung - Produktivsystem</h1>
                <p>System bereit fuer Benutzeranfragen.</p>
            </body>
        </html>
    }
}
```

### 9. Zusammenfassung
- Zelyra-Anwendungen lassen sich ohne komplizierte Server-Stacks direkt und performant betreiben.
- Datenbank-Migrationen und Webbetrieb greifen nahtlos ineinander.

### 10. Kontrollfragen zur Selbstprüfung
1. Welcher Befehl startet die Webanwendung auf einem Server?
2. Warum sollte `zelyra db apply` vor dem Starten des Webservers ausgeführt werden?
3. Welche Vorteile bietet der integrierte HTTP-Server gegenüber externen Server-Setups?

# TEIL X – ABSCHLUSSPROJEKT UND WEITERFÜHRUNG

---

## Kapitel 40: Das große Abschlussprojekt: Vollständige Aufgabenverwaltung

### 1. Was lerne ich in diesem Kapitel?
In diesem großen Finale lernst du:
- Wie alle gelernten Bausteine zu einer vollständigen, produktionsreifen Anwendung verschmelzen.
- Wie Datenbank, Authentifizierung, Rollen, CRUD-Interface, REST-API und Geschäftslogik ineinandergreifen.
- Wie du den kompletten Code liest, verstehst und auf deinem Server in Betrieb nimmst.

### 2. Warum ist das Thema wichtig?
Einzelne Code-Snippets zu verstehen ist eine Sache – eine echte, zusammenhängende Anwendung aus einem Guss zu bauen, ist die eigentliche Kunst der Softwareentwicklung. Dieses Abschlussprojekt beweist die Eleganz von Zelyra: In einer einzigen, übersichtlichen Datei entsteht eine datenbankgestützte, authentifizierte Webanwendung mit UI und REST-API, für die man in herkömmlichen Frameworks dutzende Dateien anlegen müsste.

### 3. Verständliche Erklärung des Gesamtprojekts
Unsere Aufgabenverwaltung umfasst:
1. **Datenbank & Schemas:** Ziel-Engine MariaDB mit Tabellen für Aufgaben (`tasks`), Benutzer (`users`), Sitzungen (`auth_sessions`) und Berechtigungen (`user_permissions`).
2. **Authentifizierung:** `auth users` mit geschützten Routen und Argon2-Passwortschutz.
3. **Geschäftslogik:** Reine Funktionen mit Verträgen (`requires`, `ensures`) zur Prioritätsprüfung und Fortschrittsberechnung.
4. **CRUD-Interface:** Das administrative Web-Dashboard mit Listen, Formularen und Löschdialogen.
5. **REST-API:** JSON-Endpunkte für den programmatischen Zugriff.

### 4. Das vollständige Projekt: Der finale Quelltext

```zelyra
database main {
    engine: mariadb
    database: "zelyra_tasks_app"
}

// ==========================================
// 1. AUTHENTIFIZIERUNG & BENUTZERVERWALTUNG
// ==========================================

auth users {
    table: users
    sessions: auth_sessions
    permissions: user_permissions
}

table users {
    id: Id primary auto
    email: Email required unique
    password_hash: String(255) required
    active: Bool default true
}

table auth_sessions {
    id: Id primary auto
    user: User required
    token_hash: String(64) required unique
    expires_at: Timestamp required
    device_label: String(255)
}

table user_permissions {
    id: Id primary auto
    user: User required
    permission: String(100) required
}

// ==========================================
// 2. AUFGABEN-DATENMODELL
// ==========================================

type TaskId = Id

table tasks {
    id: TaskId primary auto
    name: String(120) required
    beschreibung: String(500)
    prioritaet: Int default 2
    erledigt: Bool default false
}

// ==========================================
// 3. GESCHÄFTSLOGIK MIT VERTRÄGEN
// ==========================================

fn berechne_erfolgsquote(erledigte: Int, gesamt: Int) -> Int
    requires { gesamt > 0 && erledigte >= 0 && erledigte <= gesamt }
    ensures { result >= 0 && result <= 100 }
{
    return (erledigte * 100) / gesamt
}

fn prioritaet_label(stufe: Int) -> String {
    match stufe {
        1 => {
            return "HOCH"
        }
        2 => {
            return "MITTEL"
        }
        3 => {
            return "NIEDRIG"
        }
        _ => {
            return "NORMAL"
        }
    }
}

// ==========================================
// 4. WEBOBERFLÄCHE & CRUD-SCHNITTSTELLE
// ==========================================

crud Task -> tasks {
    title: "Zelyra Aufgabenverwaltung"

    view {
        fields {
            name
            prioritaet
            erledigt
        }

        list {
            mode: cards
            empty: "Keine Aufgaben vorhanden. Erstelle deine erste Aufgabe!"
        }

        detail {
            mode: cards
            title: "Aufgabendetails"
        }

        form {
            mode: cards
            title: "Aufgabe bearbeiten"
            submit: "Aufgabe sichern"
        }

        delete {
            title: "Aufgabe entfernen"
            message: "Moechtest du diese Aufgabe wirklich loeschen?"
            submit: "Jetzt loeschen"
        }
    }

    action abschliessen {
        label: "Als erledigt markieren"
        confirm: "Aufgabe abschliessen?"

        sql {
            UPDATE tasks
            SET erledigt = true
            WHERE id = :id
        }

        success "Aufgabe erfolgreich abgeschlossen."
        redirect "/tasks"
    }
}

// ==========================================
// 5. REST-API ENDPUNKTE
// ==========================================

api GET "/api/tasks/{id}" {
    input {
        id: TaskId
    }
    output Task
    errors {
        404 NotFound
    }
}

api POST "/api/tasks" {
    input {
        name: String
        prioritaet: Int
    }
    output Task
    errors {
        400 ValidationError
    }
}

// ==========================================
// 6. STARTSEITE
// ==========================================

page "/" {
    html {
        <html>
            <head>
                <title>Zelyra Aufgaben-System</title>
            </head>
            <body>
                <h1>Zelyra Aufgabenverwaltung</h1>
                <p>Das vollstaendige Abschlussprojekt ist einsatzbereit.</p>
                <a href="/tasks">Zur Aufgabenuebersicht</a>
            </body>
        </html>
    }
}

// ==========================================
// 7. EINSTIEGSPUNKT
// ==========================================

fn main() {
    print("Zelyra Aufgabenverwaltung vollstaendig initialisiert.")
}
```

### 5. Typische Fehler und deren Ursachen
- **Fehler:** Den Code starten, ohne vorher `zelyra db apply` ausgeführt zu haben.
  *Ursache:* Die Tabellen in MariaDB müssen angelegt sein, bevor der Webserver Anfragen verarbeitet.
- **Fehler:** Fehlende MariaDB-Zugangsdaten in `.env`.
  *Ursache:* Hinterlege `DB_HOST`, `DB_USER` und `DB_PASSWORD` in deiner Umgebungsdatei.

### 6. Merksätze
1. In Zelyra entsteht eine vollständige, sichere Web-App in einer einzigen, harmonischen Datei.
2. Typsicherheit, Verträge, Authentifizierung und APIs greifen lückenlos ineinander.
3. Dieser Code ist sofort mit `zelyra check` prüfbar und mit `zelyra serve` startbar.

### 7. Übungsaufgaben
- **Stufe 1 (Leicht):** Kompiliere das Gesamtprojekt mit `zelyra check` und überprüfe, dass 0 Fehler auftreten.
- **Stufe 2 (Mittel):** Ergänze die Tabelle `tasks` um ein Feld `faellig_am: Date` und passe Formular und Ansichten an.
- **Stufe 3 (Anspruchsvoll):** Richte MariaDB lokal ein, spiele das Schema mit `zelyra db apply` ein und lege die ersten echten Aufgaben über den Webbrowser an.

### 8. Praxisaufgabe: Dein eigener produktiver Server-Start
Initialisiere das Projekt und starte es:
```bash
zelyra new task_manager --template minimal
cd task_manager
# Quellcode in src/main.zyl einfuegen
zelyra check src/main.zyl
zelyra serve src/main.zyl 0.0.0.0:8080
```
Öffne `http://localhost:8080` – deine eigene Zelyra-Anwendung ist live!

### 9. Zusammenfassung
- Das Abschlussprojekt vereint alle 9 vorangegangenen Teile dieses Lehrbuchs.
- Du hast gelernt, wie man eine moderne, fehlertolerante Webanwendung von Grund auf baut.

### 10. Kontrollfragen zur Selbstprüfung
1. Welche Komponenten wurden im Abschlussprojekt kombiniert?
2. Warum genügen in Zelyra so wenige Zeilen für ein vollwertiges CRUD-System?
3. Welche Schritte sind erforderlich, um das Projekt auf einem neuen Server zu deployen?

---

<a id="kapitel-41-die-zelyra-roadmap-von-030-bis-10"></a>

## Kapitel 41: Die Zelyra-Roadmap (Von 0.3.0 bis 1.0)

### 1. Was lerne ich in diesem Kapitel?
- Die Entwicklung von Zelyra: Was das experimentelle Release 0.3.0 liefert und was als Nächstes kommt.
- Geplante nächste Meilensteine: vollständige Module, Package-Management und WebAssembly.
- Wie Abwärtskompatibilität und Stabilitätsgarantien bis Version 1.0 gewährleistet werden.

### 2. Warum ist das Thema wichtig?
Eine Programmiersprache ist ein lebendiges Ökosystem. Wer heute Zeit investiert, um Zelyra zu lernen, möchte sicher sein, dass die Sprache eine klare Zukunft hat, professionell weiterentwickelt wird und bestehender Code auch in kommenden Versionen lauffähig bleibt.

### 3. Verständliche Erklärung: Die Roadmap im Überblick
Die Entwicklung von Zelyra gliedert sich in Implementierungsphasen und
Release-Meilensteine:
- **Phase 1 bis 3 (Fundament):** Lexer, Parser, AST, Typsystem, Kontrollstrukturen, Funktionen und Verträge (`requires`, `ensures`). *(Abgeschlossen)*
- **Phase 4 bis 6 (Datenbank & Daten):** MariaDB/SQLite-Engine, typisiertes `sql<T>`, Migrationen, Transaktionen, Dateisystem- und Zeit-Capabilities. *(Abgeschlossen)*
- **Phase 7 bis 9 (Web & Sicherheit):** `page`, `html`, `form` mit CSRF/XSS-Schutz, `crud`-Views, `auth` mit Argon2, `api` mit OpenAPI-Generierung, Typed Holes, JSON-Compilerdiagnostik. *(Abgeschlossen)*
- **0.3.0 (aktuelles experimentelles Release):** geprüfte Compiler- und
  Datenbankpfade, Installer und Update-Check, generierte Businessanwendungen
  sowie Linux- und Windows-x86_64-Release-Artefakte. Die menschliche
  Einsteigerabnahme wurde auf das verpflichtende 0.4.0-Gate verschoben.
- **0.2.0 (vorheriges Release):** Typisierte Maps, deklarative Suche,
  Filterung und Pagination, wiederverwendbare Views und Slots, erzeugtes CRUD,
  Authentifizierung und Berechtigungen, Audit-Unterstützung, Setup-/Doctor-
  Werkzeuge sowie maschinenlesbare Compiler-Schnittstellen.
- **0.4.0 (vorgeschlagen):** Module, Datenbank-Lebenszyklus, sichere
  Konto-/API-Abläufe und unabhängige menschliche Einsteigerabnahme. Siehe den
  [Releaseplan](../../release-plans/0.4.0.de.md).
- **Spätere Meilensteine:** Paketmanager, WebAssembly und jede LTS-Zusage
  bleiben zukünftige Arbeiten. Der aktuelle Entwicklungszweig enthält
  experimentelle Imports für Funktionen, Typen, Records, Tabellen, Views,
  Komponenten und projektweite Datenbankkonfiguration. Sie sind nicht in
  0.3.0 enthalten und bilden noch kein vollständiges, stabiles Modulmodell.

### 4. Zelyras Versprechen an Entwickler
- **Keine Breaking Changes ohne Deprecation:** Änderungen an der Syntax werden mit klaren Übergangsfristen und Compiler-Hinweisen eingeführt.
- **Verlässliche Spezifikation:** Jedes Sprachmerkmal ist in der formalen Grammatik festgeschrieben.

### 5. Typische Missverständnisse
- **Missverständnis:** „Zelyra 0.3.0 ist produktionsreif, weil die Kernpfade funktionieren.“
  *Richtigstellung:* Zelyra 0.3.0 ist ein experimenteller, getesteter Umfang.
  Unterstützte Pfade und Restrisiken sind dokumentiert; eine Freigabe für
  Produktion wird nicht behauptet.
- **Missverständnis:** Annehmen, dass Imports sich in allen Zelyra-Versionen wie in anderen Sprachen verhalten.
  *Richtigstellung:* Das veröffentlichte 0.3.0 hat keine Modul-Imports. Der aktuelle Entwicklungszweig unterstützt statische, projektlokale Imports einer begrenzten Auswahl von Deklarationen über `check`, `build`, `run`, `serve`, `context`, `verify` und `impact`. Vollständiges Packaging, Sichtbarkeit und die Integration aller Werkzeuge bleiben geplant.

### 6. Merksätze
1. Zelyra besitzt einen klaren, transparenten Entwicklungsplan vom aktuellen
   Release 0.3.0 zu späteren Meilensteinen.
2. Datenbank, Web, Typsicherheit und KI-native Werkzeuge besitzen getestete
   experimentelle Pfade, sind aber heute nicht für Produktion freigegeben.
3. Ein vollständiges Modulmodell, Paketverteilung und WebAssembly bleiben
   zukünftige Arbeiten; der aktuelle Entwicklungszweig enthält einen
   begrenzten, experimentellen Mehrdatei-Compilerpfad.

### 7. Übungsaufgaben
- **Stufe 1 (Leicht):** Lies das offizielle `CHANGELOG.md` im Zelyra-Repository.
- **Stufe 2 (Mittel):** Vergleiche die Features von Phase 9 mit den Planungen für Phase 11.
- **Stufe 3 (Anspruchsvoll):** Schreibe ein kurzes Konzept für ein zukünftiges Zelyra-Paket, das du in Phase 12 veröffentlichen möchtest.

### 8. Praxisaufgabe: Zelyra-Version und Umgebung auditieren
Überprüfe die installierte Version und den Zustand deines Systems:
```bash
zelyra --version
zelyra doctor
```

### 9. Zusammenfassung
- Zelyra schreitet zielstrebig auf Version 1.0 zu.
- Das modulare Phasenkonzept garantiert kontinuierliche, stabile Weiterentwicklung.

### 10. Kontrollfragen zur Selbstprüfung
1. Welche Kernbausteine sind im getesteten Umfang von 0.3.0 implementiert?
2. Für welchen zukünftigen Meilenstein sind feingliedrige `import`-Anweisungen geplant?
3. Warum ist der experimentelle Status vor einem Produktiveinsatz wichtig?

---

## Kapitel 42: Dein Weg als Zelyra-Entwickler

### 1. Was lerne ich in diesem Kapitel?
- Wie du dein erworbenes Wissen vertiefst und in eigenen Projekten anwendest.
- Die wichtigsten Prinzipien für professionelle, langlebige Softwarearchitektur.
- Wie du Teil der Zelyra-Community wirst und zum Open-Source-Ökosystem beiträgst.

### 2. Warum ist das Thema wichtig?
Programmieren lernt man nicht durch bloßes Lesen, sondern durch Machen. Dieses Buch hat dir das Fundament vermittelt – jetzt beginnt deine persönliche Reise als Entwickler. Mit Zelyra besitzt du eine moderne, sichere und zukunftsfähige Sprache, die dich bei jedem Schritt unterstützt.

### 3. Die fünf goldenen Regeln für Zelyra-Entwickler
1. **Model First:** Beginne jedes Projekt mit dem `table`-Schema. Ein klares Datenmodell löst die Hälfte aller späteren Probleme.
2. **Definiere Verträge:** Sichere wichtige Funktionen mit `requires` und `ensures` ab. Sie sind Dokumentation und Test in einem.
3. **Rechte bewusst vergeben:** Halte den Großteil deines Codes rein (ohne Capabilities) und deklariere Seiteneffekte gezielt.
4. **Fehler als Werte behandeln:** Nutze `Result` und `Option`. Verbanne Ausreden für unkontrollierte Abstürze.
5. **Gemeinsam mit KI arbeiten:** Nutze Zelyras `--format json` und Typed Holes `_`, um KI-Assistenten als produktive Co-Piloten einzusetzen.

### 4. Dein nächstes Projekt: Ideen zum Weiterprogrammieren
- **Persönliches Haushaltsbuch:** Einnahmen, Ausgaben, Kategorien und Monatsberichte mit Zelyra CRUD.
- **Support-Ticket-System:** Kundenanfragen, Prioritäten, Zuweisungen und E-Mail-Benachrichtigungen.
- **Kunden- und Projektzeiterfassung:** Stundenerfassung mit `uses Clock` und Rechnungs-Export als JSON.

### 5. Zusammenfassung des Lehrbuchs
Herzlichen Glückwunsch! Du hast alle 10 Teile und 42 Kapitel von *Zelyra lernen* erfolgreich gemeistert. Du beherrschst die Grundlagen, das Typsystem, Fehlerbehandlung, Datenbanken, Webanwendungen, APIs und sichere Softwarearchitektur. Du bist jetzt bereit, eigene, robuste Anwendungen mit Zelyra zu erschaffen!

### 6. Kontrollfragen zum Abschluss
1. Welche Zelyra-Besonderheit schätzt du nach diesem Lehrgang am meisten?
2. Warum ist das Zusammenspiel von Sprache und Datenbank in Zelyra so revolutionär?
3. Welches Projekt wirst du als Nächstes mit Zelyra umsetzen?
