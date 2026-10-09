# PART VI – DATABASES WITH ZELYRA

---

## Chapter 23: Why Zelyra Understands Databases Directly

### 1. What will I learn in this chapter?
In this chapter, you will learn:
- Why the integration between programming languages and relational databases has traditionally been prone to subtle bugs.
- What the object-relational impedance mismatch problem is and why ORMs struggle with it.
- How Zelyra integrates relational databases as first-class citizens directly into the language.
- How Zelyra verifies SQL statements for syntax and type correctness at compile time.

### 2. Why is this topic important?
In nearly all popular web frameworks (such as PHP/Laravel, Python/Django, or Node/TypeORM), an awkward divide exists: developers compose SQL strings or rely on heavyweight object-relational mapping (ORM) abstractions. Typos in column names—such as `user.emaiil`—often remain undetected until an end user encounters an HTTP 500 error in production. Zelyra eliminates this hazard at the root: if an SQL query does not match the declared table schema, the compiler rejects the build immediately.

### 3. Understandable explanation without unnecessary jargon
In Zelyra, you define your database configuration directly in your source code using the `database` keyword:

```zelyra
database main {
    engine: mariadb
    database: "tasks_db"
}

fn main() {
    print("Database configuration initialized.")
}
```

When querying data, you write authentic SQL—yet the compiler knows every existing table and column:
- Writing `SELECT id, description FROM tasks` is completely valid.
- Writing `SELECT non_existent FROM tasks` causes Zelyra to report a compile-time error immediately: `unknown column non_existent`.

### 4. Small, progressive examples

**Example 1: Declaring database and table**
```zelyra
database main {
    engine: mariadb
    database: "test_db"
}

table tasks {
    id: Id primary auto
    description: String(255) required
}

fn main() {
    print("Database and table checked.")
}
```

**Example 2: Compile-time SQL validation**
```zelyra
database main {
    engine: mariadb
    database: "test_db"
}

table tasks {
    id: Id primary auto
    description: String(255) required
}

fn show_tasks() uses Database {
    records = sql<Task[]> {
        SELECT id, description
        FROM tasks
    }
    print("SQL type-checked.")
}

fn main() uses Database {
    show_tasks()
}
```

### 5. Typical errors and their causes
- **Error:** Executing SQL queries without declaring `uses Database` on the enclosing function.
  *Cause:* Zelyra's capability system protects against unauthorized database queries.
- **Error:** Omitting the `database main` block.
  *Cause:* Without a target engine declaration, Zelyra cannot verify SQL dialect semantics or the schema.

### 6. Key takeaways
1. Zelyra bridges the gap between application code and relational databases.
2. SQL queries are type-checked at compile time.
3. Database operations strictly require `uses Database`.

### 7. Exercises (Level 1 Easy, Level 2 Medium, Level 3 Challenging)
- **Level 1 (Easy):** Create a `database` block for SQLite or MariaDB.
- **Level 2 (Medium):** Model a `users` table and write an SQL query that selects all users.
- **Level 3 (Challenging):** Deliberately introduce a typo into an SQL column name and observe how Zelyra pinpoints the error.

### 8. Practical project task: Task Management
Set up the database foundation for our task management system:
```zelyra
database main {
    engine: mariadb
    database: "zelyra_tasks"
}

table tasks {
    id: Id primary auto
    name: String(100) required
    done: Bool
}

fn status_report() uses Database {
    records = sql<Task[]> {
        SELECT id, name, done
        FROM tasks
    }
    print("Database for task management ready.")
}

fn main() uses Database {
    status_report()
}
```

### 9. Summary
- Databases and schemas are native, integral components of Zelyra.
- Typos in SQL statements are caught and prevented at compile time.

### 10. Self-check review questions
1. What problem do Zelyra's type-checked SQL blocks solve compared to ordinary SQL strings?
2. Which capability must be declared by a function executing `sql`?
3. How does Zelyra derive the type `Task` from the table `tasks`?

---

## Chapter 24: Defining Tables and Data Modeling

### 1. What will I learn in this chapter?
In this chapter, you will learn:
- How relational tables are declared with `table`.
- The syntax for primary keys: `id: Id primary auto`.
- How column modifiers are defined: `required`, length constraints like `String(100)`, and default values.
- How relationships between tables are modeled.

### 2. Why is this topic important?
The data model is the bedrock of any application. If schema design is neglected early on, technical debt, corrupt data, and performance bottlenecks will linger for years. Zelyra enforces mandatory constraints, explicit types, and referential integrity right from day one.

### 3. Understandable explanation without unnecessary jargon
A table (`table`) is like a structured filing cabinet for uniform record sheets:
- Every sheet possesses a unique serial number: `id: Id primary auto`.
- Certain attributes must never be omitted: `required`.
- Text columns can have strict length bounds: `String(100)`.

```zelyra
table categories {
    id: Id primary auto
    name: String(50) required
}
```

From the definition `table categories`, Zelyra automatically generates the strongly typed struct `Category` with corresponding fields.

### 4. Small, progressive examples

**Example 1: Simple table with required fields**
```zelyra
database main {
    engine: mariadb
    database: "app_db"
}

table projects {
    id: Id primary auto
    name: String(80) required
    active: Bool
}

fn main() {
    print("Table projects declared.")
}
```

**Example 2: Table with date and numeric fields**
```zelyra
database main {
    engine: mariadb
    database: "app_db"
}

table time_entries {
    id: Id primary auto
    hours: Float
    recorded_at: Timestamp
}

fn main() {
    print("Table time_entries declared.")
}
```

**Example 3: Linking two tables via IDs**
```zelyra
database main {
    engine: mariadb
    database: "app_db"
}

table tasks {
    id: Id primary auto
    description: String(200) required
    project_id: Id
}

fn main() {
    print("Relationship tasks -> project_id created.")
}
```

### 5. Typical errors and their causes
- **Error:** Omitting a required field during record creation.
  *Cause:* Columns flagged with `required` must hold valid values in every record.
- **Error:** Using reserved keywords such as `action`, `field`, `title`, or `list` as column names.
  *Cause:* These identifiers are reserved language syntax elements in Zelyra.

### 6. Key takeaways
1. Every table must define a primary key `id: Id primary auto`.
2. The `required` modifier prohibits null or absent values at both database and type levels.
3. The singular name of a plural table (e.g., `Task` for `tasks`) becomes an automatic Zelyra data type.

### 7. Exercises (Level 1 Easy, Level 2 Medium, Level 3 Challenging)
- **Level 1 (Easy):** Define a table `tags` with a mandatory column `name: String(30) required`.
- **Level 2 (Medium):** Create a table `customers` with `email: Email` and `phone: String(30)`.
- **Level 3 (Challenging):** Model a table `comments` linked via `task_id: Id` to a task and having a `created_at: Timestamp` column.

### 8. Practical project task: Task Management
Create the complete production data model for our task management system:
```zelyra
database main {
    engine: mariadb
    database: "zelyra_tasks"
}

table tasks {
    id: Id primary auto
    name: String(120) required
    description: String(500)
    priority: Int
    is_done: Bool
}

fn main() {
    print("Complete task schema active.")
}
```

### 9. Summary
- `table` defines the structure, types, and constraints of data records.
- Zelyra guarantees that database schemas and application types stay strictly in sync.

### 10. Self-check review questions
1. What is the purpose of the `auto` attribute on primary keys?
2. What does the `required` keyword enforce on a table column?
3. What type name is automatically synthesized from the table `projects`?

---

## Chapter 25: Querying and Modifying Data

### 1. What will I learn in this chapter?
In this chapter, you will learn:
- How to safely query records from the database using `sql<T[]>`.
- How SQL injection is made impossible through parameterized queries (`:param`).
- How to mutate data using `INSERT`, `UPDATE`, and `DELETE`.
- Why modifications must be grouped into `transaction { ... }` blocks.
- The CLI database migration commands (`zelyra db setup`, `zelyra db apply`).

### 2. Why is this topic important?
SQL injection has remained one of the top web security threats for over two decades: an attacker inputs malicious payloads into a form, extracting password hashes or destroying tables. Zelyra protects your software by construction: SQL parameters bound with a colon (`:name`) are strictly treated as data and securely escaped by the engine. Furthermore, transactions ensure that failures never leave half-committed, corrupted state in your database.

### 3. Understandable explanation without unnecessary jargon
- **Reading with sql<T[]>:**
  ```zelyra
  database main {
      engine: mariadb
      database: "tasks_db"
  }
  table tasks {
      id: Id primary auto
      name: String required
      is_done: Bool
  }
  fn load_tasks(filter_val: Bool) uses Database {
      my_tasks = sql<Task[]> {
          SELECT id, name, is_done
          FROM tasks
          WHERE is_done = :filter_val
      }
      print("Tasks loaded")
  }
  fn main() uses Database { load_tasks(false) }
  ```
- **Writing inside a transaction:**
  ```zelyra
  database main {
      engine: mariadb
      database: "tasks_db"
  }
  table tasks {
      id: Id primary auto
      name: String required
      is_done: Bool
  }
  fn create_task(new_name: String) uses Database {
      done_flag = false
      transaction {
          sql {
              INSERT INTO tasks (name, is_done)
              VALUES (:new_name, :done_flag)
          }
      }
  }
  fn main() uses Database { create_task("Test") }
  ```
  If an error occurs during execution, the database automatically rolls back all intermediate operations.

### 4. Small, progressive examples

**Example 1: Inserting a new record**
```zelyra
database main {
    engine: mariadb
    database: "tasks_db"
}

table tasks {
    id: Id primary auto
    name: String(100) required
    done: Bool
}

fn add_task(task_text: String) uses Database {
    done_status = false
    transaction {
        sql {
            INSERT INTO tasks (name, done)
            VALUES (:task_text, :done_status)
        }
    }
    print("Task saved.")
}

fn main() uses Database {
    add_task("Answer email")
}
```

**Example 2: Querying typed records**
```zelyra
database main {
    engine: mariadb
    database: "tasks_db"
}

table tasks {
    id: Id primary auto
    name: String(100) required
    done: Bool
}

fn load_all() uses Database {
    records = sql<Task[]> {
        SELECT id, name, done
        FROM tasks
    }
    print("Task list loaded.")
}

fn main() uses Database {
    load_all()
}
```

**Example 3: Updating a record**
```zelyra
database main {
    engine: mariadb
    database: "tasks_db"
}

table tasks {
    id: Id primary auto
    name: String(100) required
    done: Bool
}

fn mark_as_done(task_id: Int) uses Database {
    transaction {
        sql {
            UPDATE tasks
            SET done = true
            WHERE id = :task_id
        }
    }
    print("Status updated.")
}

fn main() uses Database {
    mark_as_done(1)
}
```

### 5. Typical errors and their causes
- **Error:** Attempting string concatenation in SQL queries (`"WHERE id = " + id`).
  *Cause:* Zelyra forbids raw string concatenation in SQL blocks. Always use named parameters with a colon (`:id`).
- **Error:** Forgetting that mutating operations must reside inside a `transaction { ... }` block.
  *Cause:* Zelyra requires explicit transaction boundaries for any write operations.

### 6. Key takeaways
1. Always bind input values in SQL using `:parameter` to prevent SQL injection vulnerabilities.
2. All write operations must be enclosed within `transaction { ... }`.
3. `zelyra db apply` migrates the declared schema to the live database.

### 7. Exercises (Level 1 Easy, Level 2 Medium, Level 3 Challenging)
- **Level 1 (Easy):** Write an SQL query selecting all uncompleted tasks (`done = false`).
- **Level 2 (Medium):** Write a function to delete a task by its ID (`DELETE FROM tasks WHERE id = :id`).
- **Level 3 (Challenging):** Implement a function that archives an old task and creates a successor task within a single transaction.

### 8. Practical project task: Task Management
Write complete database access routines for our task management application:
```zelyra
database main {
    engine: mariadb
    database: "zelyra_tasks"
}

table tasks {
    id: Id primary auto
    name: String(100) required
    is_done: Bool
}

fn create_task(task_name: String) uses Database {
    initial_status = false
    transaction {
        sql {
            INSERT INTO tasks (name, is_done)
            VALUES (:task_name, :initial_status)
        }
    }
    print("Task created.")
}

fn complete_task(target_id: Int) uses Database {
    transaction {
        sql {
            UPDATE tasks
            SET is_done = true
            WHERE id = :target_id
        }
    }
    print("Task completed.")
}

fn main() uses Database {
    create_task("Launch first Zelyra project")
    complete_task(1)
}
```

### 9. Summary
- SQL in Zelyra is native, type-safe, and automatically defended against injection attacks.
- The `transaction` block guarantees database consistency across all mutations.

### 10. Self-check review questions
1. How does Zelyra prevent malicious SQL injection attacks?
2. Why must mutating SQL statements be placed inside a `transaction` block?
3. Which CLI command provisions and migrates database tables according to your code?

# PART VII – WEB APPLICATIONS AND FORMS

---

## Chapter 26: Rendering Web Pages

### 1. What will I learn in this chapter?
In this chapter, you will learn:
- How to create web pages and routes rapidly using the `page` keyword.
- How to pass dynamic URL parameters (such as `/tasks/{id}`).
- How HTML templates are declared directly within Zelyra code (`html { ... }`).
- How Zelyra completely prevents Cross-Site Scripting (XSS) through automatic HTML escaping.

### 2. Why is this topic important?
Traditional web development forces you to coordinate multiple disjoint tools: web servers (like Nginx or Apache), URL routers, template engines (Blade, Jinja, Twig), and backend business logic. If a developer forgets to escape HTML special characters in just one place, attackers can inject malicious JavaScript (XSS) into users' browsers. In Zelyra, the HTTP server is built-in (`zelyra serve`), and context-aware HTML escaping is automatic and unavoidable.

### 3. Understandable explanation without unnecessary jargon
With the `page` keyword, you define both an HTTP route and its corresponding HTML markup in one unified block:

```zelyra
page "/welcome" {
    html {
        <h1>Welcome to Zelyra</h1>
        <p>Your modern web application is running!</p>
    }
}
```

Whenever you want to render dynamic values, place them inside curly braces: `{name}`. Zelyra replaces the placeholder securely with properly escaped text.

### 4. Small, progressive examples

**Example 1: Simple static welcome page**
```zelyra
page "/hello" {
    html {
        <html>
            <body>
                <h1>Hello Zelyra World!</h1>
            </body>
        </html>
    }
}
```

**Example 2: Dynamic route with URL parameter**
```zelyra
page "/user/{name}" {
    html {
        <html>
            <body>
                <h1>Profile of {name}</h1>
                <p>Welcome back to the dashboard.</p>
            </body>
        </html>
    }
}
```

**Example 3: Safe escaping against XSS attacks**
If a malicious user submits `<script>alert('hack')</script>` as their name, Zelyra renders it in the browser as benign text—the script will never execute:
```zelyra
page "/secure/{user_input}" {
    html {
        <div>Input: {user_input}</div>
    }
}
```

**Example 4: Reusable view layouts and named slots (from Zelyra 0.1.41)**
Instead of repeating `<html>`, `<head>`, headers, and footers on every page, you declare reusable layout shells with `view`. A view defines exactly one default slot `<slot />` and optional named slots with safe fallback content:
```zelyra
view AppShell {
    html {
        <html lang="en">
            <head><title>Zelyra Application</title></head>
            <body>
                <header>
                    <slot name="header"><h1>Zelyra Portal</h1></slot>
                </header>
                <main>
                    <slot />
                </main>
                <footer>
                    <slot name="footer"><p>Built with Zelyra</p></slot>
                </footer>
            </body>
        </html>
    }
}

page "/dashboard" {
    view: AppShell
    html {
        <slot name="header"><h1>My Dashboard</h1></slot>
        <p>Specific page content is inserted into the default slot of the AppShell.</p>
    }
}
```

**Example 5: Declarative search, filtering, and pagination (from Zelyra 0.1.40)**
For data-driven collection pages, Zelyra automatically generates semantic form controls for search, sorting, and pagination with full URL state preservation:
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
        <h1>Tasks ({total} total, page {page} of {pages})</h1>
        <ul>
            for task in tasks {
                <li>{task.title}</li>
            }
        </ul>
    }
}
```
Zelyra automatically runs the optimized `COUNT(*)` query in the background, binds `total` and `pages` as typed `UInt` variables, and renders semantic filter fieldsets.

### 5. Typical errors and their causes
- **Error:** Failing to properly close HTML tags (e.g., `<h1>` without `</h1>`).
  *Cause:* Zelyra strictly parses the HTML tree for well-formedness at compile time.
- **Error:** Inconsistent parameter names in curly braces.
  *Cause:* The route placeholder (e.g., `{id}`) must match the variable identifier used in the HTML template.
- **Errors `E-VIEW-010` to `E-VIEW-015`:** Undeclared variables or type mismatches in view interpolation.
  *Cause:* The Zelyra compiler checks view bindings and component properties at compile time against declared routes, types, and SQL loads.
- **Error:** Supplying undeclared or duplicate slots in a `page`.
  *Cause:* A page may only provide content for named slots that its declared `view` explicitly defines.

### 6. Key takeaways
1. `page "/path"` defines a web route and serves verified, well-formed HTML.
2. Variables in HTML templates are interpolated via `{variable}` and escaped automatically.
3. `view Name { ... }` defines reusable master layout shells with `<slot />` and named slots (`<slot name="...">`).
4. `search`, `filter`, and `paginated` automatically generate semantic query controls with URL state preservation.
5. The command `zelyra serve` starts the built-in HTTP server without external web server configurations.

### 7. Exercises (Level 1 Easy, Level 2 Medium, Level 3 Challenging)
- **Level 1 (Easy):** Create a `page "/about"` that displays an informative description of a company.
- **Level 2 (Medium):** Create a dynamic route `/product/{item_id}` displaying a product detail view.
- **Level 3 (Challenging):** Design an overview page containing headings, navigation links, and an ordered list inside the HTML block.

### 8. Practical project task: Task Management
Build the main landing page for our task management application:
```zelyra
page "/tasks" {
    html {
        <html>
            <head>
                <title>Zelyra Task Management</title>
            </head>
            <body>
                <h1>My Tasks</h1>
                <p>Welcome to your personal task manager.</p>
                <a href="/tasks/new">Create New Task</a>
            </body>
        </html>
    }
}
```

Start the development server with `zelyra serve main.zyl` and navigate to `http://localhost:8080/tasks` in your browser!

### 9. Summary
- Web pages are declared directly and concisely using `page` and `html`.
- Automatic context-aware escaping protects your users against web security threats.

### 10. Self-check review questions
1. Which keyword introduces a web page definition in Zelyra?
2. How are dynamic values bound into HTML markup?
3. Why are XSS vulnerabilities precluded by default in Zelyra HTML rendering?

---

## Chapter 27: Forms and User Inputs

### 1. What will I learn in this chapter?
In this chapter, you will learn:
- How to define secure input forms bound to database tables using `form`.
- How automatic Cross-Site Request Forgery (CSRF) protection works behind the scenes.
- How Zelyra validates input data against strong types (such as `Email` or length limits).
- How to test and inspect forms prior to runtime using `zelyra form validate`.

### 2. Why is this topic important?
Untrusted user inputs represent the single largest vector for web vulnerabilities: attackers submit empty required values, spoofed foreign IDs, or trick authenticated users into unauthorized requests (CSRF attacks). In other frameworks, developers must manually stitch together input forms, validation rules, error feedback, and CSRF tokens. Zelyra's `form` construct derives the input mask directly from your database table schema, safeguarding all interactions automatically.

### 3. Understandable explanation without unnecessary jargon
A form directly connects an input mask with a target database table:

```zelyra
database main {
    engine: mariadb
}

table tasks {
    id: Id primary auto
    name: String(100) required
    description: String(500)
}

form TaskCreate -> tasks {
    fields {
        name
        description
    }
}
```

From this concise definition, Zelyra automatically produces:
- HTML input elements matching the schema types (`<input type="text">`, etc.).
- An invisible cryptographic CSRF token preventing unauthorized form submissions.
- Server-side validation rules (e.g., `name` cannot exceed 100 characters and cannot be omitted).

### 4. Small, progressive examples

**Example 1: Basic form for customer data**
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

**Example 2: Minimal form with single field**
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

### 5. Typical errors and their causes
- **Error:** Listing a field inside `fields` that does not exist in the target table.
  *Cause:* Zelyra strictly validates `fields` against columns declared on the underlying table.
- **Error:** Attempting to disable CSRF protection.
  *Cause:* In Zelyra, CSRF defense is an uncompromised, mandatory architectural standard.

### 6. Key takeaways
1. `form Name -> target_table` generates a strongly validated, secure input form.
2. All table column constraints (lengths, required flags, types) are automatically enforced on input.
3. CSRF and XSS protections are integral and active by default.

### 7. Exercises (Level 1 Easy, Level 2 Medium, Level 3 Challenging)
- **Level 1 (Easy):** Define a form `CategoryCreate` for a `categories` table.
- **Level 2 (Medium):** Test form validation rules on the command line using `zelyra form validate`.
- **Level 3 (Challenging):** Add a priority column to the task form and test server-side validation against erroneous submissions.

### 8. Practical project task: Task Management
Define the creation form for new tasks:
```zelyra
database main {
    engine: mariadb
}

table tasks {
    id: Id primary auto
    name: String(100) required
    priority: Int
}

form TaskCreate -> tasks {
    fields {
        name
        priority
    }
}

fn main() {
    print("Task form ready.")
}
```

### 9. Summary
- Forms bridge database tables directly with secure web input masks.
- Zelyra manages validation, CSRF tokens, and error handling with zero boilerplate.

### 10. Self-check review questions
1. What does the arrow `->` signify in `form TaskCreate -> tasks`?
2. Why don't Zelyra developers need to manually insert CSRF tokens into templates?
3. Which column validations are automatically enforced on the form?

---

## Chapter 28: The Complete CRUD Pattern

### 1. What will I learn in this chapter?
In this chapter, you will learn:
- What CRUD (Create, Read, Update, Delete) means and why it forms the backbone of business software.
- How Zelyra synthesizes a complete administrative web interface with a single `crud` block.
- How to configure list, detail, form, and delete views.
- How to attach custom operations using the `action` keyword.

### 2. Why is this topic important?
Over 80% of routine web development consists of the exact same repetitive pattern: listing records in a table, displaying details, editing fields, and deleting entries. Developers frequently spend weeks implementing controllers, routes, views, and confirmation dialogs. In Zelyra, you achieve all of this in a few declarative lines of code—bulletproof, secure, and uniform.

### 3. Understandable explanation without unnecessary jargon
The `crud` keyword encapsulates all foundational operations for an entity:
- **C**reate: Add new records.
- **R**ead: Browse lists and inspect detail views.
- **U**pdate: Edit and persist existing records.
- **D**elete: Remove records safely with an explicit confirmation step.

```zelyra
database main {
    engine: mariadb
}

table tasks {
    id: Id primary auto
    name: String(100) required
    done: Bool default false
}

crud Task -> tasks {
    title: "Task Management"
    view {
        fields {
            name
            done
        }
        list {
            mode: cards
            empty: "No tasks available."
        }
    }
}
```

> **Automatic Detail Linking with Hidden ID (from version 0.1.50):**
> When the technical `id` column is omitted from the `fields` block (as shown here, where only `name` and `done` are visible), Zelyra automatically links the first displayed field (`name`) to the record detail page. This applies to both table (`table`) and card (`cards`) layouts.

### 4. Small, progressive examples

**Example 1: A complete CRUD module**
```zelyra
database main {
    engine: mariadb
}

table tasks {
    id: Id primary auto
    name: String(100) required
    active: Bool default true
}

crud Task -> tasks {
    title: "Tasks"

    view {
        fields {
            name
            active
        }

        list {
            mode: cards
            empty: "No tasks available."
        }

        detail {
            mode: cards
            title: "Task Details"
        }

        form {
            mode: cards
            title: "Edit Task"
            submit: "Save"
        }

        delete {
            title: "Delete Task"
            message: "This action cannot be undone."
            submit: "Delete Now"
        }
    }
}
```

**Example 2: Custom actions in the CRUD interface**
You can add custom action buttons—for example, to mark a task as completed immediately:
```zelyra
database main {
    engine: mariadb
}

table tasks {
    id: Id primary auto
    name: String(100) required
    done: Bool default false
}

crud Task -> tasks {
    title: "Tasks"
    view {
        fields {
            name
            done
        }
    }

    action complete {
        label: "Mark as completed"
        confirm: "Do you want to complete this task?"

        sql {
            UPDATE tasks
            SET done = true
            WHERE id = :id
        }

        success "Task completed successfully."
        redirect "/tasks"
    }
}
```

**Example 3: CRUD resources with reusable view layouts (from Zelyra 0.1.43)**
With `layout: ViewName`, you automatically embed all generated CRUD screens (list, details, form, and delete dialogs) into an existing layout shell:
```zelyra
view AppShell {
    html {
        <html lang="en">
            <body>
                <nav><a href="/">Home</a> | <a href="/tasks">Tasks</a></nav>
                <main>
                    <slot />
                </main>
            </body>
        </html>
    }
}

crud Task -> tasks {
    title: "Task Management"
    layout: AppShell

    view {
        fields {
            name
            done
        }
    }
}
```
The generated CRUD resource adopts the navigation and container structure of `AppShell`, while security checks, permissions, and CSRF protection remain fully active.

### 5. Typical errors and their causes
- **Error:** Omitting the `view` block or `fields` declaration inside a `crud` block.
  *Cause:* Zelyra needs an explicit list of columns to display across the generated UI views.
- **Error:** Writing an `UPDATE` statement in an `action` without `WHERE id = :id`.
  *Cause:* CRUD actions always target the specific selected record identified by `:id`.

### 6. Key takeaways
1. `crud Name -> table` generates a complete, secure administration interface.
2. Views (`list`, `detail`, `form`, `delete`) can be individually styled and tailored.
3. Custom actions (`action`) extend basic CRUD functionality with specific business logic.

### 7. Exercises (Level 1 Easy, Level 2 Medium, Level 3 Challenging)
- **Level 1 (Easy):** Create a CRUD interface for a `categories` table.
- **Level 2 (Medium):** Customize the titles, button labels, and empty messages in a CRUD block.
- **Level 3 (Challenging):** Implement a custom action `duplicate` that clones the selected task.

### 8. Practical project task: Task Management
Construct the full production CRUD interface for our task manager:
```zelyra
database main {
    engine: mariadb
    database: "tasks_app"
}

table tasks {
    id: Id primary auto
    name: String(120) required
    done: Bool default false
}

crud Task -> tasks {
    title: "My Task Manager"

    view {
        fields {
            name
            done
        }

        list {
            mode: cards
            empty: "Great job! All tasks are completed."
        }

        detail {
            mode: cards
            title: "View Task"
        }

        form {
            mode: cards
            title: "Create or Edit Task"
            submit: "Save Task"
        }

        delete {
            title: "Delete Task"
            message: "Are you sure you want to permanently delete this task?"
            submit: "Delete"
        }
    }
}
```

### 9. Summary
- The CRUD pattern dramatically accelerates web development for data-driven applications.
- Zelyra automatically generates routes, forms, validation, and database operations.

### 10. Self-check review questions
1. What four core operations make up the acronym CRUD?
2. How does Zelyra react if an invalid column name appears inside `fields`?
3. What is the role of the `action` keyword in a CRUD declaration?

---

## Chapter 29: Users, Passwords, and Sessions

### 1. What will I learn in this chapter?
In this chapter, you will learn:
- How Zelyra provides native authentication with the `auth` block.
- How passwords are hashed using state-of-the-art Argon2.
- How user sessions and role-based permissions are structured and secured.
- How pages and actions are protected using `requires auth` and `permits`.

### 2. Why is this topic important?
Security cannot be treated as an optional plugin. Storing plaintext passwords or using outdated hashing algorithms (like MD5 or SHA1) leads to catastrophic security failures. In Zelyra, authentication is baked directly into the language design: passwords default to Argon2 hashing, session tokens are securely managed, and route permissions are verified declaratively.

### 3. Understandable explanation without unnecessary jargon
An authentication block ties together users, sessions, and permissions:

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
}

table user_permissions {
    id: Id primary auto
    user: User required
    permission: String(100) required
}
```

To restrict a web page exclusively to authenticated users, annotate the route:
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
}

table user_permissions {
    id: Id primary auto
    user: User required
    permission: String(100) required
}

page "/secret" {
    requires auth
    html {
        <h1>Visible only to authenticated users!</h1>
    }
}
```

### 4. Small, progressive examples

**Example 1: The standard tables for authentication**
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
}

table user_permissions {
    id: Id primary auto
    user: User required
    permission: String(100) required
}
```

**Example 2: Protected page with permissions check**
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
        <h1>Task Dashboard</h1>
        <p>You are authorized.</p>
    }
}
```

### 5. Typical errors and their causes
- **Error:** Storing plaintext passwords inside the user table.
  *Cause:* Zelyra expects a `password_hash` column and provides `zelyra auth hash-password` for hashing.
- **Error:** Expecting sessions to survive a restart without a session table.
  *Cause:* Without `sessions: auth_sessions`, sessions are held in process memory.
  Declare the table for persistence and administrative session revocation.

#### Session administration in the 0.4 development branch

Signed-in users can open `/account/sessions` to review and revoke their own
active sessions. Pages using the generated navigation link to it.
It requires a valid session, CSRF token and same-origin evidence for changes.
Revoking the current session clears its cookie and redirects to sign-in. For
persistent sessions, the page supports both session tables with and without an
`id` column. In-memory sessions are listed only while the process is running.
Bearer tokens are never emitted. For legacy session tables without an `id`
column, the stored token hash is used as a hidden revocation selector. The page
does not record or show device names, IP addresses or browser history. The
`/account/sessions` path is reserved when authentication is configured.

When persistent sessions with an `id` column and the role administration page are configured,
authorized administrators see up to 100 unexpired sessions, ordered by expiry.
Each row contains its database ID, user email and expiry in database time.
Neither the browser token nor its hash is displayed. Device names and IP
history are not recorded or inferred.

Use **Revoke session** to end an individual session. The POST requires the
configured administration permission, a valid CSRF token and same-origin
evidence. The session ID and user ID must match. Repeating a request for an
already removed session is harmless. If audit is configured, the transaction
records `auth.session_revoke_requested`, including the numeric session ID;
this event records a request, not proof that a row existed.

Revoking your current session makes subsequent protected requests fail with
401. Sign in again: the operation does not disable the account or remove the
last administrator's role. Persistent sessions expire after one day. The
development branch also bounds in-memory sessions to 24 hours; they disappear
on restart and are not listed by this persistent-session administration page.
Expiry is checked on access. Revocation does not cancel requests already in
progress. These controls do not implement password recovery or device tracking.

### 6. Key takeaways
1. `auth` declares user, session, and permission schemas at a single centralized location.
2. Protected routes require `requires auth` and optional `permits "permission"`.
3. Passwords are exclusively preserved as secure Argon2 hashes.

### 7. Exercises (Level 1 Easy, Level 2 Medium, Level 3 Challenging)
- **Level 1 (Easy):** Generate a password hash using the CLI command `zelyra auth hash-password`.
- **Level 2 (Medium):** Protect a `/settings` route using `requires auth`.
- **Level 3 (Challenging):** Assign a specific permission to a user role using `zelyra auth role-permission`.

### 8. Practical project task: Task Management
Lock down the task management application against unauthorized access:
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
}

table user_permissions {
    id: Id primary auto
    user: User required
    permission: String(100) required
}

page "/my-tasks" {
    requires auth

    html {
        <h1>Protected Task Area</h1>
        <p>Accessible only to authenticated users.</p>
    }
}
```

### 9. Summary
- Authentication and authorization are native, first-class features in Zelyra.
- Modern security standards (Argon2, session tokens, RBAC) work out of the box with no third-party libraries.

### 10. Self-check review questions
1. Which state-of-the-art algorithm does Zelyra use for hashing passwords?
2. Which declaration restricts a web page to authenticated users?
3. What is the responsibility of the `auth_sessions` table?

---

## Chapter 30: APIs and Data Exchange

### 1. What will I learn in this chapter?
In this chapter, you will learn:
- How to define typed REST API endpoints using the `api` keyword.
- How to type HTTP methods (`GET`, `POST`, `PUT`, `DELETE`) alongside inputs and outputs.
- How to declare standard error status codes (`404 NotFound`, `400 ValidationError`).
- How Zelyra generates complete OpenAPI/Swagger documentation automatically (`zelyra doc --openapi`).

### 2. Why is this topic important?
Modern applications do not operate in a vacuum: mobile clients (iOS/Android), frontend frameworks (React, Vue), and external partner services must communicate with your backend. With conventional frameworks, API documentation falls out of sync almost immediately. In Zelyra, the API definition is the executable code itself: every route, payload parameter, and response code is strictly typed—and the OpenAPI specification is derived directly from the code.

### 3. Understandable explanation without unnecessary jargon
With the `api` keyword, you declare an unambiguous API contract:
- **Method and Path:** e.g., `GET "/tasks/{id}"`
- **Input:** What parameters does the request require?
- **Output:** What data type is returned as JSON?
- **Errors:** What HTTP status codes can occur in error cases?

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
    print("API endpoint defined.")
}
```

### 4. Small, progressive examples

**Example 1: A GET endpoint with return type**
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
    print("GET API checked.")
}
```

**Example 2: A POST endpoint for creating resources**
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
    print("POST API checked.")
}
```

**Example 3: Generating OpenAPI documentation**
Execute:
```bash
zelyra doc main.zyl --openapi
```
Zelyra generates a standards-compliant `openapi.json` file ready for import into Swagger UI, Postman, or API gateway catalogs.

### 5. Typical errors and their causes
- **Error:** Specifying an undefined type for `output`.
  *Cause:* Zelyra verifies that the return type (e.g., `Task`) exists as a table or record declaration.
- **Error:** Returning undeclared HTTP error codes.
  *Cause:* Typed API contracts require all possible error responses to be explicitly declared.

### 6. Key takeaways
1. `api METHOD "/path"` declares a strongly typed REST endpoint.
2. `input`, `output`, and `errors` define a comprehensive API contract.
3. `zelyra doc --openapi` outputs OpenAPI specifications directly from source code.

### 7. Exercises (Level 1 Easy, Level 2 Medium, Level 3 Challenging)
- **Level 1 (Easy):** Define an endpoint `GET "/api/version"` that returns a version string.
- **Level 2 (Medium):** Create an endpoint `DELETE "/api/tasks/{id}"` declaring the `404 NotFound` error code.
- **Level 3 (Challenging):** Generate an OpenAPI schema using `zelyra doc --openapi` and inspect the output JSON structure.

### 8. Practical project task: Task Management
Define the official REST API for our task management application:
```zelyra
type TaskId = Id

table tasks {
    id: TaskId primary auto
    name: String(120) required
    done: Bool
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
    print("Task REST API ready.")
}
```

### 9. Summary
- APIs in Zelyra are type-safe, self-documenting, and standards-compliant.
- With minimal declarative code, you create robust interfaces for web and mobile frontends.

### 10. Self-check review questions
1. Which four clauses make up a complete `api` declaration?
2. Why is automated OpenAPI generation superior to manual documentation?
3. What occurs when a client sends invalid data to an endpoint's `input` block?

# PART VIII – THE DISTINCTIVE FEATURES OF ZELYRA

---

## Chapter 31: Readability as the Highest Priority

### 1. What will I learn in this chapter?
In this chapter, you will learn:
- Why in real-world engineering code is read up to ten times more often than it is written.
- Which deliberate design decisions Zelyra made to achieve maximum clarity.
- Why Zelyra intentionally avoids unreadable syntax acrobatics and cryptic symbols.
- How the built-in code formatter `zelyra fmt` enforces a uniform code standard across entire teams.

### 2. Why is this topic important?
Many programming languages allow the exact same logic to be expressed in ten disparate ways—frequently in ultra-compact one-liners saturated with symbols that no one understands three months later. In large development teams and long-lived projects, this results in staggering maintenance overhead. Zelyra follows a core philosophy: There is exactly one obvious, readable way to solve any given problem.

### 3. Understandable explanation without unnecessary jargon
Readability in Zelyra means:
- **Expressive names over obscure abbreviations:** Functions and variables speak clear language (`priority` instead of `prio_lvl_fn()`).
- **Unambiguous blocks:** Every conditional branch, loop, and function declaration uses explicit curly braces `{}`.
- **Automated formatting:** No team member ever needs to debate indentation, whitespace, or bracket placement. The command `zelyra fmt` aligns every single line strictly to the official Zelyra style guide.

```zelyra
fn calculate_total_duration(task_durations: Int[]) -> Int {
    mutable sum = 0
    for duration in task_durations {
        sum = sum + duration
    }
    return sum
}

fn main() {
    durations: Int[] = [15, 30, 45]
    print(calculate_total_duration(durations))
}
```

### 4. Small, progressive examples

**Example 1: Self-documenting function signatures**
```zelyra
fn is_task_overdue(deadline_days: Int) -> Bool {
    return deadline_days < 0
}

fn main() {
    print(is_task_overdue(-2))
}
```

**Example 2: Clear, readable control flow**
```zelyra
fn status_display(status_code: Int) -> String {
    match status_code {
        1 => {
            return "New"
        }
        2 => {
            return "In Progress"
        }
        3 => {
            return "Completed"
        }
        _ => {
            return "Unknown"
        }
    }
}

fn main() {
    print(status_display(2))
}
```

### 5. Typical errors and their causes
- **Error:** Naming variables with arbitrary single letters (`a`, `x`, `tmp`) whose intent is obscure.
  *Cause:* Zelyra code is intended to read like clear prose. Use descriptive names such as `task_index` or `elapsed_seconds`.
- **Error:** Manually trying to fix inconsistent indentation.
  *Cause:* Simply run `zelyra fmt main.zyl`—the compiler formats everything automatically and deterministically.

### 6. Key takeaways
1. Always write code for the developer who must maintain it six months from now.
2. `zelyra fmt` guarantees a clean, uniform programming style across the entire codebase.
3. Descriptive, expressive identifiers are the highest-value documentation you can write.

### 7. Exercises (Level 1 Easy, Level 2 Medium, Level 3 Challenging)
- **Level 1 (Easy):** Format an unformatted or messy source file using `zelyra fmt`.
- **Level 2 (Medium):** Refactor a function with cryptic variable names into clean, self-explanatory Zelyra code.
- **Level 3 (Challenging):** Structure a multi-stage computation function so cleanly that no single line exceeds 80 characters, while retaining total clarity.

### 8. Practical project task: Task Management
Design the filter logic of our Task Management system with maximum readability:
```zelyra
fn is_urgent_and_open(priority: Int, completed: Bool) -> Bool {
    is_priority_one = priority == 1
    is_not_yet_completed = !completed
    return is_priority_one && is_not_yet_completed
}

fn main() {
    print(is_urgent_and_open(1, false))
    print(is_urgent_and_open(2, false))
}
```

### 9. Summary
- Readability is your primary safeguard against software rot and maintenance friction.
- Zelyra enforces clarity both by language design and through standard tooling.

### 10. Self-check review questions
1. Why does highly readable code save substantial time and money over the lifespan of a software project?
2. Which Zelyra CLI command automatically reformats source files to the canonical standard?
3. Why does Zelyra deliberately avoid having multiple redundant syntax options for the same logical operation?

---

## Chapter 32: AI-Nativity – Why Zelyra Is Built for AI Assistants

### 1. What will I learn in this chapter?
In this chapter, you will learn:
- What it means for a programming language to be truly "AI-native".
- Why modern Large Language Models (such as Claude, GPT, and Gemini) hallucinate significantly less when generating Zelyra code.
- How Typed Holes (`_`) assist both human developers and AI coding agents during iterative synthesis.
- How CLI commands equipped with `--format json` expose rich, machine-readable interfaces for automated tooling.
- How `zelyra impact` and `zelyra edit` safeguard automated refactoring workflows.

### 2. Why is this topic important?
Most existing programming languages were designed decades ago exclusively for human keyboard input. When modern AI assistants generate code in Python or JavaScript, they frequently invent nonexistent APIs, misjudge types, or fail to account for hidden side effects. Zelyra was engineered from inception so that human developers and AI assistants can collaborate with unprecedented precision and zero ambiguity.

### 3. Understandable explanation without unnecessary jargon
Zelyra accelerates and safeguards AI-assisted development through four key architectural strengths:
1. **Unambiguous, context-free grammar:** The language contains no syntactic ambiguities, eliminating parse guesswork.
2. **Machine-readable JSON diagnostics:** Nearly all compiler commands support `--format json` (for example, `zelyra check --format json`), enabling AI agents to ingest diagnostics directly as structured objects.
3. **Typed Holes (`_`):** When you or an AI model are unsure how a specific value should be derived, you place an underscore `_`. The compiler immediately provides exact feedback: What type is expected? What variables are in scope? What contracts apply?
4. **Impact Analysis (`zelyra impact`):** Zelyra computes ahead of time exactly which downstream components of a program are affected by a proposed code edit.

### 4. Small, progressive examples

**Example 1: Typed contracts guide AI generation without guesswork**
Because contracts define explicit boundaries, AI agents can generate provably correct implementations:
```zelyra
fn normalize_scale(value: Int) -> Int
    requires { value >= 0 && value <= 100 }
    ensures { result >= 0 && result <= 10 }
{
    return value / 10
}

fn main() {
    print(normalize_scale(85))
}
```

**Example 2: Machine-readable error analysis**
When an AI tool invokes:
```bash
zelyra check main.zyl --format json
```
it receives structured JSON with exact error codes, file paths, line numbers, column offsets, and actionable remediation hints.

### 5. Typical errors and their causes
- **Error:** Leaving Typed Holes (`_`) in production code.
  *Cause:* Typed Holes are development scratchpads. Before final compilation, every `_` must be replaced with concrete code.
- **Error:** Accepting AI-generated code blindly without running `zelyra check`.
  *Cause:* Always use the Zelyra compiler as an incorruptible arbiter of correctness.

### 6. Key takeaways
1. Zelyra is an AI-native language: unambiguous, structured, and tool-friendly.
2. Typed Holes `_` serve as precise design prompts for both the compiler and AI assistants.
3. `--format json` enables seamless, zero-overhead integration into autonomous AI coding agents and modern IDEs.

### 7. Exercises (Level 1 Easy, Level 2 Medium, Level 3 Challenging)
- **Level 1 (Easy):** Execute `zelyra check` with the `--format json` flag and inspect the structured JSON schema of the output.
- **Level 2 (Medium):** Run `zelyra impact` against a file and analyze which dependents are flagged for recompilation.
- **Level 3 (Challenging):** Insert a Typed Hole `_` into a calculation function and examine the compiler diagnostics to see all inferred type and scope data.

### 8. Practical project task: Task Management
Write a well-formed function signature with contracts, providing an optimal specification for an AI assistant to complete:
```zelyra
fn calculate_remaining_time(target_hour: Int, current_hour: Int) -> Int
    requires { target_hour >= current_hour }
    ensures { result >= 0 }
{
    return target_hour - current_hour
}

fn main() {
    remaining = calculate_remaining_time(18, 14)
    print(remaining)
}
```

### 9. Summary
- Zelyra eliminates lexical and semantic ambiguities that historically confuse language models.
- Typed Holes and structured JSON outputs make pair programming with AI agents exceptionally reliable and deterministic.

### 10. Self-check review questions
1. What does the concept of "Typed Holes" signify in Zelyra?
2. Why do AI coding systems benefit substantially from the compiler's `--format json` option?
3. How do formal contracts (`requires`, `ensures`) guide an AI assistant in generating correct implementations?

---

## Chapter 33: Safety through Capabilities

### 1. What will I learn in this chapter?
In this chapter, you will learn:
- What the capability security model is and why it is superior to conventional permission architectures.
- System capabilities such as `Console`, `Database`, `Network`, `FileSystem`, `Process`, `Environment`, `Clock`, and `Random`.
- How capabilities are declared on functions, propagated through call graphs, and constrained in `zelyra.toml`.
- Why Zelyra is naturally immune to supply chain attacks and rogue dependencies.

### 2. Why is this topic important?
In contemporary package ecosystems such as npm (JavaScript) or PyPI (Python), projects routinely incorporate hundreds of third-party dependencies. If any single transitive dependency is compromised, it can silently read environment secrets, exfiltrate data across the internet, or encrypt local files. In Zelyra, this attack vector is eliminated at compile time: a function cannot send a single byte over the network without explicitly declaring `uses Network`—otherwise the compiler rejects the code outright!

### 3. Understandable explanation without unnecessary jargon
Think of Zelyra's capability system as a physical security clearance:
- If a function needs to write data to disk, it must explicitly hold the `uses FileSystem` badge on its signature.
- If it lacks that capability, it cannot touch the file system—even if malicious code tries to execute a write.
- Crucially, if function `A` calls function `B`, and `B` requires a capability, `A` must also declare that capability. A quick glance at `main()` immediately reveals the total security boundary of the entire program!

```zelyra
fn safe_operation() {
    // This function has NO capabilities.
    // It is mathematically impossible for it to damage the disk or access the network!
    print("Guaranteed side-effect free.")
}

fn main() {
    safe_operation()
}
```

### 4. Small, progressive examples

**Example 1: Safe declaration of environment access**
```zelyra
fn read_configuration(key: String) -> Option<String>
    uses Environment
{
    return env(key)
}

fn main() uses Environment {
    print("Configuration access allowed.")
}
```

**Example 2: Database access guarded by capabilities**
```zelyra
database main {
    engine: mariadb
    database: "tasks_db"
}

table tasks {
    id: Id primary auto
    name: String required
}

fn load_data() uses Database {
    records = sql<Task[]> {
        SELECT id, name
        FROM tasks
    }
    print("Database access granted.")
}

fn main() uses Database {
    load_data()
}
```

### 5. Typical errors and their causes
- **Error:** Forgetting to declare capabilities on upstream caller functions.
  *Cause:* If function `a()` calls function `b() uses FileSystem`, then `a()` must also be annotated with `uses FileSystem`.
- **Error:** Capability denied by project configuration in `zelyra.toml`.
  *Cause:* The project configuration file sets the ceiling of permitted capabilities for the application.

### 6. Key takeaways
1. No function can secretly access files, databases, or the network.
2. All side effects are explicitly documented in function signatures.
3. Pure functions without capabilities are guaranteed to be side-effect free and immune to external tampering.

### 7. Exercises (Level 1 Easy, Level 2 Medium, Level 3 Challenging)
- **Level 1 (Easy):** Inspect an existing codebase and list all functions that require system capabilities.
- **Level 2 (Medium):** Write a utility function that combines `uses Clock` and `uses Environment`.
- **Level 3 (Challenging):** Architect an application such that 100% of business domain logic resides in pure functions that require zero capabilities.

### 8. Practical project task: Task Management
Isolate capability boundaries in our Task Management system:
```zelyra
// 1. Pure logic: NO capabilities required
fn is_ready_for_export(task_count: Int) -> Bool {
    return task_count > 0
}

// 2. I/O logic: Explicit FileSystem capability
fn execute_export(filename: String, content: String) uses FileSystem {
    write_text(filename, content)
    print("Export completed.")
}

fn main() uses FileSystem {
    if is_ready_for_export(5) {
        execute_export("tasks.txt", "Task 1")
    }
}
```

### 9. Summary
- Zelyra's capability model protects against malicious packages and unintended side effects.
- Applications become secure by design through explicit permission declaration and static enforcement.

### 10. Self-check review questions
1. Name three system capabilities provided by Zelyra.
2. Why must caller functions declare the capabilities required by the sub-functions they invoke?
3. How does Zelyra prevent supply chain attacks originating from third-party libraries?

---

## Chapter 34: Zelyra in Comparison

### 1. What will I learn in this chapter?
In this chapter, you will learn:
- How Zelyra compares directly with Python, PHP/Laravel, TypeScript/Node, and Rust.
- Where each language excels and why Zelyra was custom-tailored for modern data-driven web and AI applications.
- How Zelyra marries the rigorous type safety of Rust with the development velocity of Python.

### 2. Why is this topic important?
No single programming language is optimal for every possible domain: C and Rust are unmatched for operating system kernels; Python reigns in data science; JavaScript dominates the browser. However, when building database-backed business applications and web backends, developers in those languages often battle significant legacy baggage. Zelyra unifies the greatest strengths of these ecosystems.

### 3. Understandable explanation without unnecessary jargon: Language Comparison

| Feature | Python | PHP / Laravel | TypeScript / Node | Rust | Zelyra |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **Typing System** | Dynamic | Dynamic/Optional | Static (erased to JS) | Static (very strict) | **Static & Unambiguous** |
| **Null Safety** | `None` crashes at runtime | `null` crashes at runtime | `undefined` crashes | Absolute (`Option`) | **Absolute (`Option`)** |
| **SQL Integration** | ORM (String-based) | ORM (Eloquent) | ORM (Prisma/TypeORM) | Diesel / SQLx | **Native & Type-Checked** |
| **Contracts** | No (only `assert`) | No | No | External libraries | **Built-in (`requires`, `ensures`)** |
| **Security Capabilities** | No (Full OS access) | No (Full OS access) | No (Full OS access) | No | **Built-in (`uses ...`)** |
| **AI Tooling Support** | Moderate (Ambiguous) | Moderate | Moderate | Difficult for LLMs | **Native (JSON, Typed Holes)** |

### 4. Small, progressive examples

**Comparison: How Zelyra catches bugs that slip into production in other languages**

*In Python / JavaScript (potential runtime crash on missing value):*
An unhandled missing field triggers a production server crash: `AttributeError: 'NoneType' object has no attribute 'title'`.

*In Zelyra (guaranteed safe handling enforced at compile time):*
```zelyra
fn show_title(opt_title: Option<String>) {
    match opt_title {
        Some(t) => {
            print("Title: " + t)
        }
        None => {
            print("No title provided.")
        }
    }
}

fn main() {
    show_title(Some("Project X"))
    show_title(None)
}
```

### 5. Typical errors and their causes
- **Error:** Attempting to write Zelyra like Python (using indentation instead of curly braces, or attempting dynamic type reassignment).
  *Cause:* Zelyra utilizes curly braces and enforces strict, static type stability.
- **Error:** Confusing Zelyra with low-level systems languages like Rust (searching for manual memory management or complex lifetime annotations).
  *Cause:* Zelyra manages memory completely automatically, freeing the developer to focus on business logic.

### 6. Key takeaways
1. Zelyra unifies the simplicity and developer velocity of scripting languages with the strict correctness of modern static type systems.
2. Databases, web endpoints, and interfaces are native language constructs rather than disparate third-party libraries.
3. Reliability and security are enforced structurally by the compiler rather than added as an afterthought.

### 7. Exercises (Level 1 Easy, Level 2 Medium, Level 3 Challenging)
- **Level 1 (Easy):** Port a simple calculation script from Python into idiomatic, statically typed Zelyra.
- **Level 2 (Medium):** Compare an Eloquent database query in PHP/Laravel with Zelyra's type-safe `sql<T[]>`.
- **Level 3 (Challenging):** Explain, using a real-world supply chain vulnerability example, how Zelyra's capability system completely neutralizes malicious third-party dependencies.

### 8. Practical project task: Task Management
Consolidate key strengths of Zelyra within a concise demonstration of our Task Management system:
```zelyra
database main {
    engine: mariadb
    database: "tasks_demo"
}

table tasks {
    id: Id primary auto
    name: String(100) required
    completed: Bool default false
}

fn count_open_tasks() -> Int uses Database {
    open_tasks = sql<Task[]> {
        SELECT id, name, completed
        FROM tasks
        WHERE completed = false
    }
    return len(open_tasks)
}

fn main() uses Database {
    count = count_open_tasks()
    print("Open tasks determined.")
}
```

### 9. Summary
- Zelyra bridges the gap between overly complex systems languages and error-prone dynamic scripting languages.
- For modern web backends, data processing, and AI workflows, Zelyra offers an exceptionally dependable, developer-friendly platform.

### 10. Self-check review questions
1. What concrete advantage does Zelyra's `Option` type offer compared to Python's `None` or JavaScript's `null`?
2. Why is native database integration in Zelyra safer than traditional ORM libraries?
3. What vital role do capabilities play in safeguarding software from malicious third-party packages?

# PART IX – FROM DESIGN TO FINISHED APPLICATION

---

## Chapter 35: Planning Software – From Idea to Design

### 1. What will I learn in this chapter?
In this chapter, you will learn:
- How to transform a vague product idea into a precise, implementable software architecture.
- How to sketch domain entities and their relationships on paper or in your editor.
- Why defining your schema early in Zelyra simplifies every subsequent development step.
- How to decompose requirements into small, independently testable milestones.

### 2. Why is this topic important?
The single most common mistake made by beginners (and careless veterans) is rushing to write code before planning the underlying data flow. If you realize midway through implementation that a crucial table column or entity relation is missing, you often spend days refactoring existing code. Zelyra rewards disciplined planning: once your `table` schema is established, forms, validations, and APIs emerge naturally and predictably from that single source of truth.

### 3. Understandable explanation without unnecessary jargon
Every successful software project progresses through four planning phases:
1. **Clarify purpose and target audience:** Who uses the system? What primary problem must it solve? (e.g., "A team lead wants to create, assign, and mark tasks as completed").
2. **Design the data model:** What entities exist? Which fields are mandatory? (e.g., `tasks` with `name`, `priority`, and `completed`).
3. **Security and access rules:** Who is permitted to perform which operations? Do we require authentication and permissions?
4. **Iterative implementation:** First the schema (`table`), then domain logic (`fn`), then web views (`crud`/`page`), and finally APIs (`api`).

### 4. Small, progressive examples

**Example 1: The first milestone – The data model**
```zelyra
database main {
    engine: mariadb
    database: "task_planner"
}

table tasks {
    id: Id primary auto
    name: String(100) required
    priority: Int default 2
    completed: Bool default false
}

fn main() {
    print("Planning Step 1: Schema established.")
}
```

**Example 2: Planning business logic as pure functions**
```zelyra
fn validate_deadline(days: Int) -> Bool {
    return days >= 0
}

fn main() {
    print(validate_deadline(3))
}
```

### 5. Typical errors and their causes
- **Error:** Attempting to program all features simultaneously before validating the core foundation.
  *Cause:* Build software incrementally: verify each milestone immediately with `zelyra check`.
- **Error:** Ambiguous mandatory fields in the data model.
  *Cause:* Decide from the beginning which fields are strictly `required` and which may be empty (`Option<T>`).

### 6. Key takeaways
1. Failing to plan is planning to fail in software architecture.
2. A clean, unambiguous data model is the backbone of any robust application.
3. Decompose complex problems into small, independently verifiable functions.

### 7. Exercises (Level 1 Easy, Level 2 Medium, Level 3 Challenging)
- **Level 1 (Easy):** Outline the feature set and data fields of a simple note-taking application as bullet points.
- **Level 2 (Medium):** Design a relational table schema for users, tasks, and categories with appropriate Zelyra types and constraints.
- **Level 3 (Challenging):** Formulate pre- and postconditions (`requires`, `ensures`) for all core domain functions of your planned application.

### 8. Practical project task: Task Management
Consolidate the architectural blueprint for our Task Management system:
```zelyra
database main {
    engine: mariadb
    database: "tasks_pro"
}

table tasks {
    id: Id primary auto
    name: String(120) required
    priority: Int default 1
    completed: Bool default false
}

fn calculate_urgency(priority: Int, remaining_days: Int) -> String {
    if priority == 1 {
        return "HIGHEST PRIORITY"
    }
    if remaining_days <= 1 {
        return "URGENT DUE TO DEADLINE"
    }
    return "NORMAL"
}

fn main() {
    status = calculate_urgency(1, 5)
    print("Architecture plan verified: " + status)
}
```

### 9. Summary
- Structured planning saves development time and prevents costly architectural dead-ends.
- Zelyra's declarative language structure integrates seamlessly into agile development workflows.

### 10. Self-check review questions
1. What four phases constitute professional software design?
2. Why should the data model be defined before implementing user interfaces?
3. How do preconditions assist in formulating unambiguous software specifications?

---

## Chapter 36: Architecture and Clean Code Structure

### 1. What will I learn in this chapter?
In this chapter, you will learn:
- How to structure your Zelyra application using the established three-layer architectural model.
- The clean separation of persistence (`table`), business logic (`fn`), and presentation (`page`, `crud`, `api`).
- How to minimize coupling and keep modules maintainable over long lifecycles.
- Why a clean architecture protects you from unexpected regressions when adding new features.

### 2. Why is this topic important?
When database queries, business calculations, and HTML markup are tangled together in the same file or function, the result is fragile, unmaintainable "spaghetti code." If the database schema changes later, the frontend unexpectedly breaks. A clean architecture establishes clear boundaries: each layer has a single, well-defined responsibility.

### 3. Understandable explanation without unnecessary jargon
A well-architected Zelyra application is divided into three distinct layers:
1. **Data and Persistence Layer:** Table declarations (`table`) and strongly typed SQL queries (`sql<T[]>`).
2. **Business Logic Layer:** Pure calculation and validation functions protected by formal contracts (`requires`, `ensures`).
3. **Presentation and Interface Layer:** Web interfaces (`crud`, `page`) and REST endpoints (`api`).

```zelyra
// 1. Data Model
table tasks {
    id: Id primary auto
    name: String required
}

// 2. Business Logic
fn format_name(raw_text: String) -> String {
    return "[TASK] " + raw_text
}

// 3. Entry point / Execution
fn main() {
    print(format_name("Check server"))
}
```

### 4. Small, progressive examples

**Example 1: Decoupling domain logic from I/O**
```zelyra
// Pure calculation function: No capabilities needed
fn calculate_percentage(value: Int, max_value: Int) -> Int
    requires { max_value > 0 && value >= 0 }
{
    return (value * 100) / max_value
}

// I/O function: Uses the logic and prints output
fn main() {
    percentage = calculate_percentage(45, 50)
    print(percentage)
}
```

**Example 2: Clear structure through expressive component names**
```zelyra
table settings {
    id: Id primary auto
    app_name: String(60) required
}

fn show_system_info(name: String) {
    print("System running: " + name)
}

fn main() {
    show_system_info("Zelyra Task Suite")
}
```

### 5. Typical errors and their causes
- **Error:** Embedding complex business calculations directly into SQL queries or HTML template blocks.
  *Cause:* Extract calculations into dedicated helper functions so they can be independently unit-tested.
- **Error:** Introducing circular dependencies between modules.
  *Cause:* Ensure data flow travels cleanly downward (Presentation -> Logic -> Data).

### 6. Key takeaways
1. Strictly separate data models, domain business rules, and UI presentation.
2. Core business logic should remain free of side effects and capabilities whenever possible.
3. Clean architectural layers ensure applications remain maintainable and straightforward to extend.

### 7. Exercises (Level 1 Easy, Level 2 Medium, Level 3 Challenging)
- **Level 1 (Easy):** Identify the three distinct layers in an existing code snippet.
- **Level 2 (Medium):** Refactor calculations embedded in a web route into pure standalone helper functions.
- **Level 3 (Challenging):** Design a full three-layer architectural blueprint for an employee time-tracking system.

### 8. Practical project task: Task Management
Implement the layered architectural structure for our Task Management system:
```zelyra
database main {
    engine: mariadb
    database: "tasks_architecture"
}

// Layer 1: Persistence
table tasks {
    id: Id primary auto
    name: String required
    completed: Bool default false
}

// Layer 2: Business Logic
fn is_task_important(name: String, urgent: Bool) -> Bool {
    return urgent
}

// Layer 3: Application / Execution
fn main() uses Database {
    important = is_task_important("Submit tax return", true)
    print("Task architecture verified.")
}
```

### 9. Summary
- The three-layer model provides long-term maintainability, clarity, and structural safety.
- Zelyra's type system and declarative constructs enforce these boundaries naturally.

### 10. Self-check review questions
1. What three layers form the foundation of a clean Zelyra application?
2. Why should core business logic avoid requiring capabilities whenever feasible?
3. How does separating presentation from data persistence simplify future UI redesigns?

---

## Chapter 37: Configuration and Environment Variables

### 1. What will I learn in this chapter?
In this chapter, you will learn:
- How to securely manage configuration values via environment variables (`.env`).
- How to use the standard library function `env(key)` and its required capability `uses Environment`.
- How to safely handle the `Option<String>` return value from `env()`.
- Why passwords, database secrets, and API tokens must never be hard-coded into source code.

### 2. Why is this topic important?
Accidentally committing database credentials or API keys into public Git repositories is one of the most common and disastrous security vulnerabilities in modern software. Furthermore, applications must adapt dynamically across development, testing, and production environments (e.g., using different database hosts). Environment variables cleanly decouple source code from confidential, environment-specific configuration.

### 3. Understandable explanation without unnecessary jargon
In Zelyra, you read environment variables using the built-in function `env()`. Because an environment variable may or may not be defined on the host system, `env()` always returns an `Option<String>`:

```zelyra
fn read_port() -> String uses Environment {
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
    port = read_port()
    print("Server listening on port: " + port)
}
```

### 4. Small, progressive examples

**Example 1: Configuring database host dynamically**
```zelyra
fn get_db_host() -> String uses Environment {
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
    print("Connecting to: " + get_db_host())
}
```

**Example 2: Dynamically checking debug mode**
```zelyra
fn is_debug_active() -> Bool uses Environment {
    match env("APP_DEBUG") {
        Some(val) => {
            return val == "true"
        }
        None => {
            return false
        }
    }
}

fn main() uses Environment {
    if is_debug_active() {
        print("Debug mode is ON")
    } else {
        print("Debug mode is OFF")
    }
}
```

### 5. Typical errors and their causes
- **Error:** Hardcoding sensitive database passwords directly in `.zyl` files.
  *Cause:* Always store secrets in a local `.env` file and read them via `env()`.
- **Error:** Calling `env()` without declaring `uses Environment`.
  *Cause:* Zelyra's capability security model prevents unauthorized access to host environment state.

### 6. Key takeaways
1. Never commit confidential secrets or credentials into version control.
2. `env(name)` returns an `Option<String>` and requires the `uses Environment` capability.
3. Always supply safe, predictable default fallback values for missing environment variables.

### 7. Exercises (Level 1 Easy, Level 2 Medium, Level 3 Challenging)
- **Level 1 (Easy):** Read an environment variable `USER_NAME` and print a personalized greeting to the console.
- **Level 2 (Medium):** Write a reusable helper function `get_env_or_default(key: String, default_val: String) -> String`.
- **Level 3 (Challenging):** Configure an application so that it switches its logging and database endpoints between development and production modes based on environment settings.

### 8. Practical project task: Task Management
Construct the configuration management component for our Task Management system:
```zelyra
fn load_app_title() -> String uses Environment {
    match env("APP_TITLE") {
        Some(app_title) => {
            return app_title
        }
        None => {
            return "Zelyra Task Manager 0.1"
        }
    }
}

fn main() uses Environment {
    print("System started: " + load_app_title())
}
```

### 9. Summary
- Environment variables provide flexible, secure configuration management across deployment tiers.
- Zelyra's `Option` type ensures you explicitly handle missing configuration values without runtime crashes.

### 10. Self-check review questions
1. Why must confidential API keys and credentials never be hard-coded into application source files?
2. What return type does the standard function `env()` produce?
3. Which capability is required by a function that reads environment variables?

---

## Chapter 38: Debugging and Optimization

### 1. What will I learn in this chapter?
In this chapter, you will learn:
- How to diagnose your project and environment thoroughly using the CLI command `zelyra doctor`.
- How to leverage compiler diagnostics and actionable error hints effectively.
- Systematic debugging methodologies for isolating logic bugs.
- How to identify and eliminate performance bottlenecks.

### 2. Why is this topic important?
Even with careful design, unexpected issues arise during software development: a database port might be unreachable, an environment variable might be misconfigured, or an algorithm might execute redundant operations. Guessing randomly wastes precious hours. Systematic debugging with Zelyra's built-in toolchain leads to immediate solutions in minutes.

### 3. Understandable explanation without unnecessary jargon
Zelyra provides a comprehensive toolkit for proactive system diagnostics:
- `zelyra check`: Validates syntax, types, capabilities, and formal contracts.
- `zelyra doctor`: Audits host system environments, database connections, open ports, and package configurations.
- `zelyra impact`: Pinpoints precisely which functions and modules are affected by a proposed change.

```bash
zelyra doctor main.zyl
```
If your MariaDB server is offline or unreachable, `zelyra doctor` immediately highlights the exact cause instead of leaving you stranded.

### 4. Small, progressive examples

**Example 1: Systematic diagnostic print statements**
```zelyra
fn calculate_sum(numbers: Int[]) -> Int {
    mutable total = 0
    for n in numbers {
        total = total + n
    }
    return total
}

fn main() {
    values: Int[] = [10, 20, 30]
    result = calculate_sum(values)
    print("Calculated result:")
    print(result)
}
```

**Example 2: Guarding against infinite loops using loop invariants**
```zelyra
fn safe_count(limit: Int) -> Int
    requires { limit > 0 }
{
    mutable i = 0
    while i < limit
        invariant { i >= 0 }
    {
        i = i + 1
    }
    return i
}

fn main() {
    print(safe_count(10))
}
```

### 5. Typical errors and their causes
- **Error:** Modifying lines of code at random when an unexpected behavior occurs.
  *Cause:* Accurately isolate the problem first using `zelyra check`, log output, and compiler hints.
- **Error:** Suspecting database driver bugs when the project configuration in `zelyra.toml` simply lacks required permissions.
  *Cause:* Run `zelyra doctor` to detect configuration issues instantly.

### 6. Key takeaways
1. `zelyra doctor` is your first step when investigating environment, network, or database failures.
2. Contracts and invariants catch algorithmic errors before code ever enters production.
3. Systematic root-cause debugging is faster and far more reliable than trial-and-error guessing.

### 7. Exercises (Level 1 Easy, Level 2 Medium, Level 3 Challenging)
- **Level 1 (Easy):** Run `zelyra doctor` against your Task Management project and inspect each verification line.
- **Level 2 (Medium):** Create a function with a deliberate logic error in an isolated test harness and analyze compiler feedback.
- **Level 3 (Challenging):** Write a multi-step data transformation pipeline that outputs structured status logs at each milestone.

### 8. Practical project task: Task Management
Build an internal health check routine for our Task Management system:
```zelyra
fn perform_self_test() -> Bool {
    test_ok = 1 + 1 == 2
    return test_ok
}

fn main() {
    print("Starting system self-test...")
    if perform_self_test() {
        print("[OK] System functioning properly.")
    } else {
        print("[ERROR] Internal system error.")
    }
}
```

### 9. Summary
- The Zelyra toolchain (`doctor`, `check`, `impact`) provides rapid clarity when encountering defects.
- Contracts and loop invariants prevent logical state corruption before it becomes a production bug.

### 10. Self-check review questions
1. What system and project aspects are audited by `zelyra doctor`?
2. How can you systematically isolate a defect within an iterative calculation?
3. What role do loop invariants play in preventing logical regression?

---

## Chapter 39: Deployment and Operations

### 1. What will I learn in this chapter?
In this chapter, you will learn:
- How Zelyra web applications are deployed to production environments.
- How to package and operate your application inside lightweight Docker containers.
- How Zelyra connects to a production MariaDB cluster.
- How to maintain high availability using the built-in HTTP server with `zelyra serve`.

### 2. Why is this topic important?
Software is of little value if it only runs on a developer's local workstation. It must operate reliably 24/7 in production or cloud infrastructure. In traditional technology stacks, deployment often involves labyrinthine configurations of PHP-FPM, Apache/Nginx reverse proxies, and process managers. Zelyra radically simplifies operations: a single configuration file and a lightweight binary container are all you need.

### 3. Understandable explanation without unnecessary jargon
Zelyra includes its own high-performance, asynchronous web server:
```bash
zelyra serve src/main.zyl 0.0.0.0:8080
```
For professional production deployments, you package your application into a Docker container:
- The container contains the compiled Zelyra binary, your project source files, and `zelyra.toml`.
- On startup, the container automatically runs database schema migrations via `zelyra db apply` and launches the web server.

### 4. Small, progressive examples

**Example 1: Production-ready project configuration**
```toml
# zelyra.toml
[package]
name = "tasks_production"
version = "1.0.0"

[capabilities]
database = true
filesystem = false
network = true
```

**Example 2: Clean web server entry point**
```zelyra
database main {
    engine: mariadb
    database: "tasks_prod"
}

table tasks {
    id: Id primary auto
    name: String(120) required
    done: Bool default false
}

page "/" {
    html {
        <h1>Zelyra Task Manager Live</h1>
        <p>Production system active and secure.</p>
    }
}
```

**Example 3: Server startup commands**
On your production host or orchestration cluster:
```bash
zelyra db apply src/main.zyl
zelyra serve src/main.zyl 0.0.0.0:80
```

### 5. Typical errors and their causes
- **Error:** Starting the web server before applying database migrations (`zelyra db apply`).
  *Cause:* New tables and columns must exist in MariaDB before incoming HTTP requests attempt to query them.
- **Error:** Forgetting to expose port `8080` in Docker.
  *Cause:* Ensure your `docker run` command or compose definition includes `-p 8080:8080`.

### 6. Key takeaways
1. `zelyra serve` launches the built-in HTTP server without external web server dependencies.
2. `zelyra db apply` safely migrates production schemas to the latest version.
3. Explicit capability constraints in `zelyra.toml` harden your production server against unauthorized system access.

### 7. Exercises (Level 1 Easy, Level 2 Medium, Level 3 Challenging)
- **Level 1 (Easy):** Launch your web application locally on port 3000 using `zelyra serve main.zyl 127.0.0.1:3000`.
- **Level 2 (Medium):** Author a `docker-compose.yml` file orchestrating MariaDB and your Zelyra application container.
- **Level 3 (Challenging):** Simulate a production schema update: add a new column, run `zelyra db plan` to review SQL statements, and execute `zelyra db apply`.

### 8. Practical project task: Task Management
Consolidate all production deployment settings for our Task Management system:
```zelyra
database main {
    engine: mariadb
    database: "tasks_production"
}

table tasks {
    id: Id primary auto
    name: String(100) required
    completed: Bool default false
}

page "/" {
    html {
        <html>
            <body>
                <h1>Task Management - Production System</h1>
                <p>System ready for user traffic.</p>
            </body>
        </html>
    }
}
```

### 9. Summary
- Zelyra applications run directly and performantly without complex server stack dependencies.
- Database migrations, type-checked schemas, and web serving mesh seamlessly into one unified workflow.

### 10. Self-check review questions
1. Which command launches the integrated web application server?
2. Why must `zelyra db apply` be executed prior to starting the production HTTP server?
3. What architectural advantages does an embedded HTTP server offer over external web server setups?

# PART X – CAPSTONE PROJECT AND LOOKING AHEAD

---

## Chapter 40: The Grand Capstone Project: Complete Task Management

### 1. What will I learn in this chapter?
In this grand finale, you will learn:
- How all the individual building blocks learned throughout this book fuse into a cohesive, production-ready application.
- How databases, authentication, role permissions, CRUD dashboards, REST APIs, and business contracts mesh seamlessly together.
- How to read, understand, compile, and deploy the entire monolithic application on a live server.

### 2. Why is this topic important?
Understanding isolated code snippets is one thing—building a real, production-ready web application from a single cohesive blueprint is the true craft of software engineering. This capstone project demonstrates the extraordinary elegance of Zelyra: in a single, well-structured file, you create a database-backed, authenticated web system with UI views and REST APIs that would require dozens of fragmented files in legacy frameworks.

### 3. Understandable explanation without unnecessary jargon: Overview of the Complete Project
Our complete Task Management application comprises:
1. **Database & Schema Persistence:** Target MariaDB engine with relational tables for tasks (`tasks`), users (`users`), sessions (`auth_sessions`), and permissions (`user_permissions`).
2. **Authentication:** Integrated `auth users` mechanism with protected session tokens and secure Argon2 password hashing.
3. **Business Domain Logic:** Pure functions governed by formal contracts (`requires`, `ensures`) for calculating completion rates and priority classifications.
4. **CRUD Interface:** Full administrative web dashboard featuring card-mode listings, detail views, creation/edit forms, and safe deletion workflows.
5. **REST API:** Strongly typed JSON endpoints for programmatic external access.

### 4. Small, progressive examples: The Complete Project: Final Source Code

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

### 5. Typical errors and their causes
- **Error:** Launching the web server before executing `zelyra db apply`.
  *Cause:* MariaDB tables and columns must be migrated before incoming web traffic can be serviced.
- **Error:** Missing database credentials in your `.env` configuration.
  *Cause:* Ensure `DB_HOST`, `DB_USER`, and `DB_PASSWORD` are configured in your local `.env` file.

### 6. Key takeaways
1. In Zelyra, a complete, secure web application is declared in one cohesive, maintainable source file.
2. Type safety, contracts, user authentication, and REST APIs integrate with zero boilerplate.
3. This application compiles instantly with `zelyra check` and serves live traffic via `zelyra serve`.

### 7. Exercises (Level 1 Easy, Level 2 Medium, Level 3 Challenging)
- **Level 1 (Easy):** Compile the complete project using `zelyra check` and verify that 0 errors are reported.
- **Level 2 (Medium):** Extend the `tasks` schema with a `faellig_am: Date` deadline field and update the form and list views accordingly.
- **Level 3 (Challenging):** Set up a local MariaDB instance, apply the schema with `zelyra db apply`, and create your first real production tasks through your web browser.

### 8. Practical project task: Task Management - Launching Your Own Production Server
Initialize the project directory and start the server:
```bash
zelyra new task_manager --template minimal
cd task_manager
# Insert the complete source code above into src/main.zyl
zelyra check src/main.zyl
zelyra serve src/main.zyl 0.0.0.0:8080
```
Navigate to `http://localhost:8080` in your browser—your own live Zelyra application is operational!

### 9. Summary
- The capstone project unites all nine preceding parts of this textbook into one production-grade system.
- You have learned how to build modern, fault-tolerant web applications from the ground up.

### 10. Self-check review questions
1. What components were successfully combined in the capstone project?
2. Why does Zelyra require so few lines of code to deliver a comprehensive CRUD system?
3. What sequential steps are required to deploy this project to a fresh production server?

---

<a id="chapter-41-the-zelyra-roadmap-from-030-to-10"></a>

## Chapter 41: The Zelyra Roadmap (From 0.3.0 to 1.0)

### 1. What will I learn in this chapter?
In this chapter, you will learn:
- The developmental lifecycle of Zelyra: what the experimental 0.3.0 release delivers and what comes next.
- Planned features for the next milestones: complete modules, package management, and WebAssembly compilation.
- How backward compatibility and stability guarantees are maintained through version 1.0.

### 2. Why is this topic important?
A programming language is a living ecosystem. When you invest time into mastering Zelyra, you want assurance that the language has a clear strategic roadmap, professional stewardship, and that existing code remains compatible in future releases.

### 3. Understandable explanation without unnecessary jargon: Roadmap Overview
Zelyra's roadmap is structured across implementation phases and release
milestones:
- **Phases 1 to 3 (Foundations):** Lexer, parser, AST, static type checker, control flow, functions, and formal contracts (`requires`, `ensures`). *(Completed)*
- **Phases 4 to 6 (Database & Data):** MariaDB and SQLite engines, type-checked `sql<T[]>`, migrations, transactions, FileSystem and Clock capabilities. *(Completed)*
- **Phases 7 to 9 (Web & Security):** `page`, `html`, `form` with CSRF/XSS protection, `crud` views, `auth` with Argon2, `api` with automated OpenAPI schema generation, Typed Holes, and structured JSON diagnostics. *(Completed)*
- **0.3.0 (current experimental release):** tested compiler and database
  paths, installer and update check, generated business applications, and
  Linux and Windows x86_64 release artifacts. Human onboarding acceptance was
  deferred to the mandatory 0.4.0 gate.
- **0.2.0 (previous release):** typed maps, declarative search, filtering and
  pagination, reusable views and slots, generated CRUD, authentication and
  permissions, audit support, setup/doctor tooling, and machine-readable
  compiler interfaces.
- **0.4.0 (proposed):** modules, database lifecycle, safer account/API
  workflows, and independent human onboarding acceptance. See the [release
  plan](../../release-plans/0.4.0.en.md).
- **Later milestones:** a package manager, WebAssembly compilation, and any
  LTS commitment remain future work. The current branch has an experimental
  import slice for functions, types, records, tables, views, components, and
  project-wide database configuration. It is not in release 0.3.0 and does
  not yet provide a complete, stable project-module model.

### 4. Small, progressive examples: Zelyra's Guarantees to Developers
- **No breaking changes without deprecation cycles:** Syntax changes are introduced with generous transition periods and explicit compiler hints.
- **Formal language specification:** Every language construct is grounded in an unambiguous formal grammar.

### 5. Typical errors and their causes: Common Misconceptions
- **Misconception:** "Zelyra 0.3.0 is production-ready because its core paths work."
  *Correction:* Zelyra 0.3.0 is an experimental, tested scope. The repository
  documents its supported paths and residual risks; production approval is not
  claimed.
- **Misconception:** Assuming that imports work in every Zelyra version like they do in another language.
  *Correction:* Published 0.3.0 has no module imports. The current development branch supports static, project-local imports for a limited set of declarations through `check`, `build`, `run`, `serve`, `context`, `verify`, and `impact`; complete module packaging, visibility, and tool integration remain planned.

### 6. Key takeaways
1. Zelyra follows a disciplined, transparent roadmap from the current 0.3.0
   release toward later milestones.
2. The core platform has tested experimental paths for database integration,
   web applications, static safety, and AI-native tooling; it is not approved
   for production today.
3. A complete module model, package distribution, and WebAssembly remain
   future work; the current development branch has a limited experimental
   multi-file compiler slice.

### 7. Exercises (Level 1 Easy, Level 2 Medium, Level 3 Challenging)
- **Level 1 (Easy):** Read the official `CHANGELOG.md` in the Zelyra GitHub repository.
- **Level 2 (Medium):** Compare the features implemented in Phase 9 with the milestones planned for Phase 11.
- **Level 3 (Challenging):** Write an architectural proposal for a future reusable Zelyra package you plan to publish once Phase 12 launches.

### 8. Practical project task: Task Management - Auditing Zelyra Version and Environment
Verify your installed toolchain version and validate your host runtime environment:
```bash
zelyra --version
zelyra doctor
```

### 9. Summary
- Zelyra is advancing steadily and methodically toward its 1.0 LTS milestone.
- The modular phased development methodology guarantees predictable, rock-solid evolution without breaking developer workflows.

### 10. Self-check review questions
1. Which core capabilities are implemented in the tested 0.3.0 scope?
2. Which future milestone is planned for granular `import` statements?
3. Why does the experimental status matter before deploying Zelyra to production?

---

## Chapter 42: Your Journey as a Zelyra Developer

### 1. What will I learn in this chapter?
In this final chapter, you will learn:
- How to deepen your acquired expertise and apply it to ambitious real-world applications.
- Core architectural principles for engineering durable, maintainable software systems.
- How to participate in the growing Zelyra community and contribute to its open-source ecosystem.

### 2. Why is this topic important?
Programming cannot be mastered merely by reading; it is mastered by building. This textbook has provided you with a rigorous foundation—now your personal journey as an autonomous software engineer begins. With Zelyra, you command a modern, statically verified, and future-proof language engineered to support you at every stage of development.

### 3. Understandable explanation without unnecessary jargon: The Five Golden Rules for Zelyra Developers
1. **Model First:** Always initiate projects with your `table` schema. A well-modeled data schema eliminates half of all future implementation defects.
2. **Define Contracts:** Anchor critical functions with `requires` and `ensures`. Contracts simultaneously serve as live documentation, formal verification, and unit tests.
3. **Grant Capabilities Deliberately:** Keep the vast majority of your codebase purely functional (zero capabilities), and isolate external side effects explicitly.
4. **Treat Errors as Values:** Leverage `Result` and `Option`. Eliminate excuses for unhandled runtime crashes and null-pointer exceptions.
5. **Collaborate with AI:** Exploit Zelyra's `--format json` diagnostics and Typed Holes `_` to turn modern AI coding assistants into reliable, high-velocity copilots.

### 4. Small, progressive examples: Architectural Patterns and Best Practices

**Example 1: Pure functions safeguarded by contracts**
```zelyra
fn calculate_completion_percentage(completed: Int, total: Int) -> Int
    requires { total > 0 && completed >= 0 && completed <= total }
    ensures { result >= 0 && result <= 100 }
{
    return (completed * 100) / total
}

fn main() {
    print(calculate_completion_percentage(4, 5))
}
```

**Example 2: Explicit capability boundaries separating domain logic from I/O**
```zelyra
fn format_system_status(service_name: String, is_active: Bool) -> String {
    if is_active {
        return service_name + " is active."
    }
    return service_name + " is offline."
}

fn log_status(message: String) uses FileSystem {
    write_text("system.log", message)
    print("Logged: " + message)
}

fn main() uses FileSystem {
    status = format_system_status("TaskWorker", true)
    log_status(status)
}
```

### 5. Typical errors and their causes
- **Error:** Abandoning contracts and type safety when rushing through new projects.
  *Cause:* In the long run, skipped contracts cause difficult-to-trace regression bugs. Invest the few seconds to specify `requires` and `ensures`.
- **Error:** Over-assigning capabilities indiscriminately.
  *Cause:* Giving every function `uses Database, FileSystem, Network` defeats the purpose of the capability security model.

### 6. Key takeaways
1. Software quality starts with a clean data schema and contract-based design.
2. Pure functions make up the reliable core of your application; side effects are strictly declared capabilities.
3. Continuous learning and practical coding are the keys to becoming an accomplished Zelyra developer.

### 7. Exercises (Level 1 Easy, Level 2 Medium, Level 3 Challenging)
- **Level 1 (Easy):** Review your previous code snippets from earlier chapters and verify that all functions have descriptive parameter names.
- **Level 2 (Medium):** Take one of your earlier exercises and add strict `requires` and `ensures` contracts to all calculation functions.
- **Level 3 (Challenging):** Draft a complete architecture plan (database schema, capabilities, pure domain functions, and REST routes) for one of the suggested next projects.

### 8. Practical project task: Task Management - Final Reflection and System Extension Ideas
Reflect on the complete Task Management system built across this textbook and consider extensions:
- **Personal Household Budget:** Incomes, expenditures, categories, and monthly summaries utilizing Zelyra CRUD.
- **Support Ticket System:** Customer inquiries, priority escalation, staff assignments, and automated email dispatches.
- **Client & Project Time Tracking:** Timesheet tracking with `uses Clock` and billing reports exported as structured JSON.

### 9. Summary
Congratulations! You have successfully mastered all 10 parts and 42 chapters of *Learning Zelyra*. You master the fundamentals, type system, error handling, databases, web applications, APIs, and secure software architecture. You are now equipped to create your own robust, production-grade applications with Zelyra!

### 10. Self-check review questions
1. Which unique feature of Zelyra do you appreciate most after completing this course?
2. Why is the seamless interplay of language, compiler, and database in Zelyra so revolutionary?
3. Which project will you build next with Zelyra?
