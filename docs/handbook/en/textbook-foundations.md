# PART I – UNDERSTANDING ZELYRA AND PROGRAMMING

---

## Chapter 1: Welcome to Zelyra

### 1. What will I learn in this chapter?
In this introductory chapter, you will learn:
- What a programming language is at its core and what purpose it serves.
- What makes Zelyra distinctive and why it was created as an independent programming language.
- Which practical goals Zelyra pursues and what kinds of tasks it is specifically tailored for.
- How Zelyra guarantees readability, reliability, and safety right from the start.
- Why Zelyra was intentionally designed for both human software engineers and AI coding assistants.
- How this textbook is structured and how you can work with it most effectively.

### 2. Why is this topic important?
Before you write your first line of code, you should understand the fundamental problem that Zelyra solves. Modern software engineering—especially when building data-intensive and web-driven applications—frequently suffers from massive fragmentation: you design a database table schema in SQL, re-implement the exact same rules inside a backend framework (such as Laravel, Express, or Django), re-validate the same constraints a third time in the frontend (HTML/JavaScript), and then manually generate API schemas.
Zelyra breaks through this fragmentation: you define your data models, business rules, and interfaces in one cohesive language—and Zelyra derives verified, secure application components from that single source of truth.

### 3. Understandable explanation without unnecessary jargon
A **programming language** is a precise set of rules and instructions. It allows you to give unambiguous commands to a computer. Computers are extraordinarily fast, but they have no innate common sense: if an instruction is ambiguous or an unhandled condition occurs, the application either crashes or produces critical bugs.

**Zelyra** is a statically typed language. This means the compiler checks types during compilation and can report certain type mismatches before execution. Which additional rules it checks depends on the implemented checking scope; this does not guarantee that a program is error-free.

At the same time, Zelyra is **database- and web-centric**:
- A table schema (`table`) is not an isolated SQL migration file, but a first-class language element.
- Variables are **immutable by default**. As a result, values cannot change unexpectedly behind the scenes.
- No null pointer surprises: missing or optional values must be declared explicitly as `Option`.

### 4. Small, progressive examples

Let us examine a first minimal Zelyra program:

```zelyra
# Our first Zelyra program: A greeting
fn main() {
    print("Welcome to Zelyra!")
}
```

To give this program more modular structure, we can break the task down into a reusable function:

```zelyra
fn greet(name: String) -> String {
    return "Hello, " + name + "! Welcome to the world of Zelyra."
}

fn main() {
    message = greet("Developer")
    print(message)
}
```

### Comments: `#` and `##`

`#` starts a line comment, either at the start of a line or after a statement, and continues to the end of that line. A block comment is enclosed by two `##` delimiters. Each delimiter stands alone on its own line; the content may span multiple lines. Block comments do not nest. Inside a string, `#` is ordinary text.

```zelyra
# A line comment
##
This text is a block comment.
It can span multiple lines.
##
fn main() {
    print("Hello") # Comment after a statement
    print("# remains visible inside the string")
}
```

Existing `//` comments remain compatible. Use `#` in new examples. The compact spelling `## text ##` on one line does not start a multiline block in the documented lexer; use the standalone delimiters shown above.

### 5. Typical errors and their causes
- **Error:** Placing a semicolon at the end of a line (as in Java, C++, or PHP).
  *Cause:* In Zelyra, line breaks cleanly delimit statements. Redundant semicolons clutter the code.
- **Error:** Mistaking Zelyra for a framework or configuration language.
  *Cause:* Zelyra has its own syntax and type system. Its compiler translates and checks Zelyra programs; the runtime and project tools complement the language for supported use cases.

### 6. Key takeaways
1. Zelyra unifies data models, business logic, and user interfaces within a single cohesive language.
2. What you mean is explicitly written in the code: no hidden magic, no implicit null values.
3. The compiler reports detected syntax and type errors before execution. Runtime errors and cases outside its checks can still occur.

### 7. Exercises
- **Level 1 (Easy):** Modify the greeting program so that it prints your own first name and hometown.
- **Level 2 (Medium):** Write a second function `farewell(name: String) -> String` that constructs a parting message, and call both functions within `main()`.
- **Level 3 (Challenging):** Identify three common bugs that frequently occur in dynamically typed languages due to typos or type mismatches (such as passing a number where text is expected), and explain how a static compiler prevents them prior to deployment.

### 8. Practical project task: Task Management
Throughout this book, we will step-by-step construct a robust, real-world **Task Management** system. We begin by laying the foundation:
Create a file named `tasks_start.zyl` that cleanly prints the system name and version number to the screen:

```zelyra
fn main() {
    system_name = "Zelyra TaskManager"
    app_version = "0.9.9"
    print(system_name + " (Version " + app_version + ") started.")
}
```

### 9. Summary
- Zelyra is a statically typed, secure, and highly readable language designed for business logic, databases, and the web.
- Zelyra eliminates redundancies between database schemas, validation layers, and API definitions.
- Variables are immutable by default, enabling the compiler to guarantee stability and clarity.

### 10. Self-check review questions
1. What fundamentally distinguishes a statically typed language from a dynamically typed language?
2. Why is it advantageous to declare database table schemas directly within the programming language itself?
3. Why are immutable values by default safer than mutable variables?

---

## Chapter 2: How a Program Works

### 1. What will I learn in this chapter?
In this chapter, you will learn:
- How your written source code is systematically transformed into an executing program.
- The roles played by the Lexer, Parser, Type Checker, Verifier, and Interpreter in Zelyra.
- What precisely takes place when a Zelyra program boots up.
- The fundamental difference between syntax (form) and semantics (meaning).
- What an algorithm is and how the IPO model (Input, Process, Output) operates.

### 2. Why is this topic important?
Diagnosing and fixing programming bugs becomes intuitive once you understand at which stage of the toolchain an issue is detected. A syntax error indicates malformed text; a type error indicates contradictory logic; a runtime error indicates an unforeseen condition encountered during execution. Understanding this processing pipeline eliminates fear when facing compiler diagnostics.

### 3. Understandable explanation without unnecessary jargon
A computer processor directly understands only binary zeros and ones (machine code). When we humans author a text file containing Zelyra code (e.g., `program.zyl`), that code traverses several distinct stages:

```text
Source Code (.zyl)
   │
   ▼
[1. Lexer]: Breaks down raw text into words/tokens
   │
   ▼
[2. Parser]: Builds an Abstract Syntax Tree (AST)
   │
   ▼
[3. Type Checker]: Validates types, symbols, and capabilities
   │
   ▼
[4. Verifier]: Formally checks loop invariants and contracts
   │
   ▼
[5. Runtime / Interpreter]: Executes the verified instructions
```

- **Syntax** represents grammar and structure: Are brackets balanced? Are keywords spelled correctly?
- **Semantics** represents meaning: Writing `age = "twenty-five"` is syntactically valid text, but if you attempt arithmetic operations with it, it is semantically meaningless.
- **Algorithm**: A finite, step-by-step procedure designed to solve a specific problem.
- **IPO Model**: Input is received, processed according to strict rules, and output is produced.

### 4. Small, progressive examples

A straightforward algorithm calculating remaining days until a deadline:

```zelyra
fn days_until_target(target_day: Int, current_day: Int) -> Int {
    remaining = target_day - current_day
    return remaining
}

fn main() {
    today = 10
    due_date = 24
    days = days_until_target(due_date, today)
    print(days)
}
```

### 5. Typical errors and their causes
- **Error:** Opening a curly brace `{` without providing a closing brace `}`.
  *Cause:* This is a **syntax error** (`E-PARSE-001`). The parser halts immediately because the syntax tree cannot be completed.
- **Error:** Assigning a string to an integer variable: `age: Int = "20"`.
  *Cause:* This is a **type error** (`E-TYPE-001`). The parser understands the grammatical structure, but the type checker rejects the assignment.

### 6. Key takeaways
1. The lexer scans characters, the parser understands structure, and the type checker validates semantic meaning.
2. The earlier an error is intercepted (at compile time rather than in production), the safer and more cost-effective the software is.
3. Every program adheres to the IPO model: take input, process using unambiguous logic, and deliver output.

### 7. Exercises
- **Level 1 (Easy):** Draw a flowchart illustrating the execution steps of `main()` in the deadline example above.
- **Level 2 (Medium):** Extend the `days_until_target` function with an `if` statement: if `current_day > target_day`, return `0`.
- **Level 3 (Challenging):** Explain in your own words why Zelyra statically validates code (`zelyra check`) before running rather than blindly executing statements line by line.

### 8. Practical project task: Task Management
In our Task Management application, we must determine task urgency based on remaining days. If fewer than or equal to 3 days remain, the task is considered urgent:

```zelyra
fn is_urgent(remaining_days: Int) -> Bool {
    return remaining_days <= 3
}

fn main() {
    deadline_in_days = 2
    urgent = is_urgent(deadline_in_days)
    if urgent {
        print("Warning: Task has high priority!")
    } else {
        print("Task is on schedule.")
    }
}
```

### 9. Summary
- Code moves through a defined pipeline: lexing, parsing, type checking, verification, and execution.
- Zelyra guarantees that syntax and types are thoroughly validated before any program runs.
- Algorithms convert inputs into reliable outputs through clear, logical sequences.

### 10. Self-check review questions
1. At what point in the toolchain is an unclosed quotation mark detected?
2. What does the IPO (Input, Process, Output) model describe?
3. Why does a statically typed system refuse to execute when text and numbers are improperly combined?

---

## Chapter 3: Installing and Setting Up Zelyra

### 1. What will I learn in this chapter?
- System requirements for the Zelyra development environment on Linux, macOS, and Windows.
- Platform-specific Docker setup and verification with `docker compose version`.
- How to install Zelyra from source (`./install.sh` / `install.ps1`) or the release archive (`--release v1.0.7`).
- Complete compiler version query with `zelyra --version` and environment diagnosis with `zelyra doctor`.
- The integrated, token-protected web setup assistant (`zelyra setup --web`).
- Typical permission and port conflicts (such as Docker socket permissions and automatic port selection).

### 2. Why is this topic important?
A smooth toolchain is the foundation of every successful project. If commands are missing or environment variables misconfigured, valuable time is lost. Zelyra provides a lean, user-local installer that requires no third-party package managers or root privileges. Additionally, the new web setup assistant visually guides beginners through database and container initialization.

### 3. Understandable explanation without unnecessary jargon
Zelyra requires neither Apache nor heavy third-party runtimes for simple programs. The Zelyra CLI (`zelyra`) is a single, highly optimized binary executable.

Choose the path that fits your operating system:
1. **Linux / macOS:** Clone the official repository and execute `./install.sh` (or install a release binary). Zelyra is installed user-locally to `~/.local/bin`. For Docker, use the [official Linux engine instructions](https://docs.docker.com/engine/install/) on Linux and [Docker Desktop for Mac](https://docs.docker.com/desktop/setup/install/mac-install/) on macOS.
2. **Windows:** On Windows, `install.ps1` for PowerShell and `install.cmd` for the command prompt are provided. For complete MariaDB projects, [Docker Desktop for Windows](https://docs.docker.com/desktop/setup/install/windows-install/) with enabled Compose support is recommended.
3. **Platform Docker Checks:** Zelyra deliberately does not install Docker itself or request root permissions. Before starting Compose services, verify your environment with `docker compose version`. If Compose is missing or socket access is denied, Zelyra outputs clear, platform-specific hints.

### 4. Small, progressive examples

**Step 1: Install Zelyra**
From source (Linux / macOS):
```bash
git clone https://github.com/sf1976/zelyra.git
cd zelyra
./install.sh
```

Or directly as a precompiled release archive without Rust:
```bash
./install.sh --release v1.0.7
```

On Windows (PowerShell):
```powershell
git clone https://github.com/sf1976/zelyra.git
Set-Location zelyra
.\install.ps1 -Release v1.0.7
```

**Step 2: Verify version and help**
```bash
zelyra --version
zelyra --help
```

`zelyra --version` outputs the full compiler and package version (for example, `zelyra 1.0.7`). The language compatibility line remains 0.1.

**Step 3: Verify Docker Compose (for MariaDB projects)**
```bash
docker compose version
```

**Step 4: Run system diagnostics**
```bash
zelyra doctor
```

This command analyzes environment paths, ports, and tools. Use `zelyra doctor --json` for structured machine-readable output.

**Step 5: Guided Web Setup Assistant (Optional)**
In any MariaDB project directory, start the visual assistant:
```bash
zelyra setup --web
```

Zelyra opens a local HTTP server on `127.0.0.1:3030` with a random, single-use security token. There, you can generate your `.env` configuration, start the MariaDB container, and apply database schemas with one click.

`zelyra setup` prepares the MariaDB environment. A minimal project created
with `zelyra init` without `--mariadb` runs directly with
`zelyra run main.zyl` and does not need this setup step. Paths are interpreted
literally: after `mkdir test_a`, `./test_a` points to that directory, while
`/test_a` on Linux means a directory directly under the filesystem root. If
only the relative directory exists, the CLI suggests that correction.

### 5. Typical errors and their causes
- **Error:** `zelyra: command not found`
  *Cause:* The directory `~/.local/bin` is not yet in your `$PATH` variable. Run `export PATH="$HOME/.local/bin:$PATH"` or restart your terminal.
- **Error:** `permission denied while trying to connect to the Docker daemon socket`
  *Cause:* On Linux systems, your user does not yet belong to the `docker` group. Run `sudo usermod -aG docker $USER` and log back in. Zelyra catches this and provides a clear hint.
- **Error:** Default port 3000 or 3306 is occupied.
  *Cause:* Another local service is using the port. `zelyra setup` and `zelyra new` automatically detect collisions and select the next free host port.

### 6. Key takeaways
1. The Zelyra CLI bundles compiler, runner, form checker, web server, and setup assistant in a single tool.
2. `docker compose version` and `zelyra doctor` verify the health of your environment at any time.
3. The release can be installed without Rust using `--release v1.0.7`.
4. `zelyra setup --web` provides an intuitive, browser-based initial setup with a secure one-time token.

### 7. Exercises
- **Level 1 (Easy):** Run `zelyra --version` and `zelyra doctor` in your terminal and note the output.
- **Level 2 (Medium):** Explore the help page with `zelyra check --help` and review the `--format json` option.
- **Level 3 (Challenging):** Set up file association in your text editor so that `.zyl` files are highlighted cleanly.

### 8. Practical project task: Prepare the task manager environment
Create a directory on your machine and ensure the Zelyra toolchain runs properly:

```bash
mkdir my-taskmanager
cd my-taskmanager
echo 'fn main() { print("TaskManager environment ready.") }' > test.zyl
zelyra check test.zyl
zelyra run test.zyl
```

### 9. Summary
- Zelyra is installed via `./install.sh`, Windows PowerShell script, or precompiled release archives.
- The command-line tool `zelyra` contains all necessary functions.
- `zelyra doctor` ensures everything is configured correctly.

### 10. Self-check review questions
1. Which command displays the full compiler and package version?
2. Why does Zelyra require no external web server like Apache for web services?
3. What does `zelyra doctor` check?

---

## Chapter 4: The First Zelyra Project

### 1. What will I learn in this chapter?
- How to create a Zelyra project with `zelyra new` or `zelyra init` (including `--mariadb`).
- How a standard project folder is structured (`main.zyl`, `zelyra.toml`, `.env`).
- Automatic port selection (`ZELYRA_HOST_PORT` and `ZELYRA_DB_HOST_PORT`) when defaults are occupied.
- How `zelyra setup --all` and `zelyra setup --web` automate the first-run workflow.
- Optional feature switches (`[features]` in `zelyra.toml` or `.env`) and inspection with `zelyra config`.
- When a program requires a `main()` function and how to check, run, and format code.

### 2. Why is this topic important?
As soon as programs exceed ten lines, they belong in a clean project structure. A standardized directory structure ensures that configurations, database models, web routes, and business logic have a predictable location.

### 3. Understandable explanation without unnecessary jargon
With `zelyra new <projectname>`, you create a turnkey project:

- **`main.zyl`**: The entry file. Here you define either `fn main()` or declare tables, web pages, and APIs.
- **`zelyra.toml`**: The durable project configuration (name, version, capabilities, and optional feature switches like `web`, `api`, `crud`, `auth`, `audit`).
- **`.env`**: Local secrets and ports not committed to version control (`DATABASE_URL`, `ZELYRA_HOST_PORT`, `ZELYRA_DB_HOST_PORT`).
- **Docker & MariaDB**: With `--mariadb`, Zelyra generates `Dockerfile`, `docker-compose.mariadb.yml`, and `.env.example`.

**Automatic Port Selection:** If default ports 3000 (web) or 3306 (MariaDB) are already in use, Zelyra automatically scans and assigns free ports in the newly generated `.env`.

**Optional Feature Switches:** You can enable or disable project surfaces in `zelyra.toml` or `.env`:
```toml
[features]
web = true
api = true
crud = true
auth = true
audit = true
```

If source code uses a disabled surface, the compiler reports `E-FEATURE-001`. You can inspect the effective configuration at any time without exposing secrets:
```bash
zelyra config main.zyl
zelyra config main.zyl --format=json
```

### 4. Small, progressive examples

**Create a project (Minimal or MariaDB):**
```bash
# Minimal script project:
zelyra new taskmanager --template minimal
cd taskmanager

# Or complete MariaDB web project:
zelyra new taskmanager-web --mariadb
cd taskmanager-web
```

**First-time setup in one command:**
```bash
zelyra setup --all
```

This creates a protected `.env`, starts the MariaDB containers, and applies the schema.

**Inspect effective project configuration:**
```bash
zelyra config main.zyl
```

**Check and run project:**
```bash
zelyra check main.zyl
zelyra run main.zyl
```

**Format project code:**
```bash
zelyra fmt main.zyl
```

Other documented CLI commands used during development:

```bash
zelyra verify main.zyl
zelyra db plan main.zyl
zelyra editor
```

`zelyra verify` runs the supported contract and invariant checks; Chapter 19 explains their scope. `zelyra db plan` shows proposed schema changes. Review the plan before applying changes with `zelyra db apply`; Chapter 24 describes the workflow. `zelyra editor` starts Zelyra Studio.

The current CLI reference lists neither a `zelyra test` subcommand nor separate `zelyrac build` or `zelyra lsp` programs. Write assertions with `assert(...)` in an executable `.zyl` file and run it with `zelyra run`; the reference does not document a dedicated test runner.

### 5. Typical errors and their causes
- **Error:** Creating files without the `.zyl` extension.
  *Cause:* The compiler strictly expects files ending in `.zyl`.
- **Error:** Running a CLI program without `fn main()`.
  *Cause:* `zelyra run` looks for `fn main()`. In web services running with `zelyra serve`, `main()` is optional.

### 6. Key takeaways
1. `zelyra new` creates a standardized project structure.
2. In `zelyra.toml`, name, version, capabilities, and feature switches are managed.
3. `zelyra setup --all` and `zelyra setup --web` streamline MariaDB setup.
4. `zelyra fmt` ensures consistent formatting across the team.

### 7. Exercises
- **Level 1 (Easy):** Create a project `my_first_project` with `zelyra new` and run it.
- **Level 2 (Medium):** Run `zelyra config main.zyl` and inspect the active feature switches.
- **Level 3 (Challenging):** Format an unformatted `.zyl` file using `zelyra fmt main.zyl`.

### 8. Practical project task: Initialize the Task Management project
Create the project that will accompany us throughout this book:

```bash
zelyra new zelyra-tasks --template minimal
cd zelyra-tasks
```

Add a menu to `main.zyl`:
```zelyra
fn show_menu() {
    print("=================================")
    print("    ZELYRA TASK MANAGEMENT       ")
    print("=================================")
    print("1: List all tasks")
    print("2: Create new task")
    print("3: Exit")
}

fn main() {
    show_menu()
}
```

Check with `zelyra check main.zyl` and run with `zelyra run main.zyl`.

### 9. Summary
- Zelyra projects have a clean structure of source code (`.zyl`) and configuration (`zelyra.toml`).
- `zelyra check` verifies correctness; `zelyra run` executes the program.
- `zelyra fmt` formats code according to standard conventions.

### 10. Self-check review questions
1. What is the role of `zelyra.toml`?
2. What happens if a default host port is occupied when creating a MariaDB project?
3. When does a Zelyra program require a `main()` function?

---

# PART II – LANGUAGE FUNDAMENTALS

## Chapter 5: Values and Data Types

### 1. What will I learn in this chapter?
In this chapter, you will learn:
- What values and data types are and why they form the backbone of reliable software.
- The fundamental numeric types: `Int`, `UInt`, `Float`, and `Decimal`.
- Text and character types: `String` and `Char`.
- Boolean truth values (`Bool`) and the empty type (`Unit`).
- Date and time representations: `Timestamp`, `Date`, `Time`, and `Duration`.
- The difference between automatic type inference and explicit type annotations.

### 2. Why is this topic important?
In the real world, you cannot add apples to oranges. Without a type system, however, a computer would do precisely that: it would happily attempt to multiply a postal code by a monetary price, or interpret arbitrary binary gibberish as a calendar date. In Zelyra, the type system prevents such absurdities before code ever runs. A data type specifies exactly which values are valid and which operations are permitted on them.

### 3. Understandable explanation without unnecessary jargon
Every value in Zelyra possesses a definite type. You can either specify the type explicitly or let Zelyra infer it automatically based on the assigned expression:

```zelyra
fn main() {
    # Explicit type annotation: Name followed by colon and type
    count: Int = 10

    # Automatic type inference: Zelyra immediately detects that this is a String
    task_name = "Important Meeting"

    print(task_name)
}
```

The core data types in Zelyra:
- **`Int`**: 64-bit signed integers, e.g., `-5`, `0`, `42`.
- **`UInt`**: Unsigned integers (`>= 0`), ideal for record IDs and positive counters.
- **`Float`**: Floating-point numbers for scientific calculations, e.g., `3.1415`.
- **`Decimal`**: Fixed-point values created with `decimal("0.10")`, with nine fractional digits. Base-10 arithmetic is supported; excess precision, overflow, and division by zero are rejected. Define business rounding rules for money explicitly.
- **`Bool`**: Boolean values representing truth. Exactly two states exist: `true` or `false`.
- **`String`**: UTF-8 character text enclosed in double quotation marks: `"Hello World"`.
- **`Char`**: Individual characters enclosed in single quotation marks: `'A'`, `'z'`.
- **`Unit`**: Represents "no meaningful value", analogous to `void` in other languages. When a function only performs side effects and returns nothing, its type is `Unit`.

### 4. Small, progressive examples

```zelyra
fn main() {
    task_id: Int = 101
    task_name: String = "Update server"
    is_done: Bool = false
    estimated_hours: Float = 2.5
    hourly_rate: Float = 85.50

    print(task_name)
    print(is_done)
}
```

Zelyra strictly guards against type mismatches:
If you attempt to write `task_id = "one hundred"`, the compiler rejects the assignment immediately with error `E-TYPE-001`.

### 5. Typical errors and their causes
- **Error:** Computing financial amounts using `Float`.
  *Cause:* IEEE-754 floating-point numbers can introduce subtle inaccuracies such as `0.1 + 0.2 = 0.30000000000000004`. In Zelyra, always use `Decimal` for financial logic.
- **Error:** Enclosing a single character in double quotes when a `Char` is expected.
  *Cause:* `"A"` is a `String`, whereas `'A'` is a `Char`.

### 6. Key takeaways
1. Data types prevent invalid operations between incompatible kinds of information.
2. For financial balances and currency: always use `Decimal`, never `Float`.
3. Zelyra infers types accurately, but explicit type annotations clearly document design intent.

### 7. Exercises
- **Level 1 (Easy):** Declare three variables for your favorite book: title (`String`), publication year (`Int`), and whether you have finished reading it (`Bool`).
- **Level 2 (Medium):** Compute the total cost of a task from `estimated_hours` and `hourly_rate`, and print the result.
- **Level 3 (Challenging):** Explain why an entity ID is typically better modeled as an `Int` or a nominal type `type TaskId = Id` rather than an arbitrary `String`.

### 8. Practical project task: Task Management
Extend our Task Management project in `main.zyl` by defining the typed attributes of an individual task:

```zelyra
fn main() {
    task_id: Int = 1
    task_name: String = "Verify database schema"
    is_done: Bool = false
    priority: Int = 1 # 1 = high, 2 = medium, 3 = low

    print("Task #" + "1" + ": " + task_name)
    if is_done {
        print("Status: Done")
    } else {
        print("Status: Open (Priority: high urgency)")
    }
}
```

### 9. Summary
- Zelyra provides a rich palette of primitive data types for numbers, text, and logic.
- Types can be explicitly annotated or automatically inferred by the compiler.
- Strict compile-time type verification catches logical bugs during development.

### 10. Self-check review questions
1. Why should `Decimal` always be chosen over `Float` for currency calculations?
2. What is the fundamental syntactic and semantic difference between `"Z"` and `'Z'`?
3. Which two distinct values can a `Bool` hold?

---

## Chapter 6: Variables and Immutability

### 1. What will I learn in this chapter?
In this chapter, you will learn:
- What a variable represents in computer memory.
- Why variables in Zelyra are immutable by default.
- How to declare mutable variables explicitly using the `mutable` keyword.
- Variable scopes and lifetimes within code blocks.
- Why immutability makes software dramatically more robust and predictable.

### 2. Why is this topic important?
One of the most frequent causes of subtle bugs in languages such as JavaScript, Python, or C++ is uncontrolled mutability: one function quietly mutates a shared variable, causing an entirely unrelated module to crash.
Zelyra adheres to a clear principle: **Constants are the baseline.** If a value is intended to change during runtime execution, you must consciously signal that behavior using `mutable`.

### 3. Understandable explanation without unnecessary jargon
Think of a variable as a labeled box stored in computer memory:

- **Immutable Binding (Default):**
  ```zelyra
  fn main() {
      task_name = "Tax Return"
      print(task_name)
  }
  ```

  You place the text `"Tax Return"` into the box labeled `task_name` and seal it. Nobody is permitted to replace the content of that box. Any code reading the box can rely on its contents remaining invariant.

- **Mutable Variable (`mutable`):**
  ```zelyra
  fn main() {
      mutable counter = 0
      counter = counter + 1
      print(counter)
  }
  ```

  Here, the box remains unsealed. You are allowed to remove the current value and replace it with a new one.

**Scope and Lifetimes:**
Variables exist exclusively within the block `{ ... }` in which they are declared. When execution exits that block, Zelyra automatically reclaims the variable. This frees memory and eliminates naming conflicts.

### 4. Small, progressive examples

**Example 1: Immutability prevents accidental overwrites**
```zelyra
fn main() {
    project = "Zelyra Core"
    # project = "New Project" // ERROR: Compiler blocks assignment to immutable variable!
    print(project)
}
```

**Example 2: When `mutable` is appropriate (counters and accumulators)**
```zelyra
fn main() {
    mutable open_tasks = 5
    print(open_tasks)

    # One task has been completed:
    open_tasks = open_tasks - 1
    print(open_tasks)
}
```

**Example 3: Variable Scopes**
```zelyra
fn main() {
    scope_var = "Global in main"
    if true {
        local_var = "Only visible inside if"
        print(local_var)
        print(scope_var)
    }
    # print(local_var) // ERROR: `local_var` no longer exists outside the block!
}
```

### 5. Typical errors and their causes
- **Error:** Attempting to reassign a variable declared without `mutable`.
  *Cause:* Zelyra issues `cannot assign to immutable variable`. If a value must change over time, declare it as `mutable name = ...`.
- **Error:** Declaring every variable as `mutable` out of habit.
  *Cause:* Poor design style. Restrict `mutable` to locations where values truly need to evolve (e.g., loop counters or accumulators).

### 6. Key takeaways
1. In Zelyra, variables are immutable by default.
2. When a value is intended to change, `mutable` must be explicitly declared.
3. Variables exist exclusively within their declared block `{ ... }`.

### 7. Exercises
- **Level 1 (Easy):** Create an immutable variable holding your username and print it.
- **Level 2 (Medium):** Create `mutable score = 100`, deduct 15 points, add 30 points, and print the intermediate values.
- **Level 3 (Challenging):** Explain why immutability provides significant safety benefits in concurrent applications running multiple operations in parallel.

### 8. Practical project task: Task Management
In our Task Management application, we want to count how many tasks remain to be completed:

```zelyra
fn main() {
    mutable open_count = 3
    print("Start: Tasks to complete: ")
    print(open_count)

    # First task completed:
    open_count = open_count - 1
    print("Intermediate count: Still open:")
    print(open_count)

    # Second task completed:
    open_count = open_count - 1
    print("Final count: Still open:")
    print(open_count)
}
```

### 9. Summary
- Immutability is Zelyra's default, preventing unintended side effects.
- Mutable variables must be explicitly declared with `mutable`.
- Curly braces define the scope and lifetime of all variables.

### 10. Self-check review questions
1. What occurs if you assign a new value to a variable declared without `mutable`?
2. Why does immutability serve as a fundamental security and stability feature?
3. Can a variable declared inside an inner block be accessed outside of that block?

---

## Chapter 7: Operators and Expressions

### 1. What will I learn in this chapter?
In this chapter, you will learn:
- What an operator is and what constitutes an expression.
- Arithmetic operators for numbers: `+`, `-`, `*`, `/`, `%`.
- Comparison operators: `==`, `!=`, `<`, `<=`, `>`, `>=`.
- Logical operators: `&&` (AND), `||` (OR), and `!` (NOT).
- Operator precedence and clean expression grouping using parentheses.

### 2. Why is this topic important?
Software does not merely store static values; it computes results, compares facts, and combines logical conditions. An operator is the mechanism that transforms raw data into actionable knowledge. Mastering operators enables you to translate complex business rules into crisp, reliable expressions.

### 3. Understandable explanation without unnecessary jargon
- An **expression** is any segment of source code that evaluates to a concrete value. For instance, `5 + 3` is an expression that yields `8`.
- An **operator** is the symbolic token that dictates the operation (such as `+` or `==`).

**Arithmetic Operators:**
- `+`: Addition (also used for string concatenation and array merging).
- `-`: Subtraction (or numerical negation: `-x`).
- `*`: Multiplication.
- `/`: Division.
- `%`: Modulo (remainder of integer division, e.g., `7 % 3 == 1`).

**Comparison Operators (always evaluate to `Bool`):**
- `==`: Is equal to?
- `!=`: Is not equal to?
- `<` / `<=`: Less than / Less than or equal to?
- `>` / `>=`: Greater than / Greater than or equal to?

**Logical Operators:**
- `&&` (AND): Evaluates to `true` only if **both** sides are `true` (`true && true == true`).
- `||` (OR): Evaluates to `true` if **at least one** side is `true`.
- `!` (NOT): Inverts a truth value (`!true == false`).

### 4. Small, progressive examples

```zelyra
fn main() {
    # Arithmetic
    base_time = 60
    buffer_time = 15
    total_time = base_time + buffer_time
    print(total_time)

    # Comparison
    is_long = total_time > 60
    print(is_long)

    # Logical combination
    has_buffer = buffer_time > 0
    is_critical = total_time > 120 && !has_buffer
    print(is_critical)
}
```

### 5. Typical errors and their causes
- **Error:** Using a single equals sign `=` inside a comparison check: `if x = 5`.
  *Cause:* `=` is the assignment operator! Equality comparisons in Zelyra strictly require the double equals `==`.
- **Error:** Division by zero (`x / 0`).
  *Cause:* Results in a runtime abort. Always validate that the divisor is non-zero before dividing.

### 6. Key takeaways
1. Assign values using `=`, compare values using `==`.
2. Arithmetic expressions respect operator precedence; use parentheses whenever clarity is needed.
3. `&&` demands that both operands are true, while `||` succeeds when either operand is true.

### 7. Exercises
- **Level 1 (Easy):** Use `==` and `%` to verify whether `10 % 2` equals `0` (even-number check).
- **Level 2 (Medium):** Write an expression testing whether an integer `age` falls within the range `18` to `65` (inclusive).
- **Level 3 (Challenging):** Write an expression for a discount rule: a customer qualifies for a discount if they are a VIP (`is_vip == true`) OR if their order total exceeds 100 AND they are not a new customer.

### 8. Practical project task: Task Management
In our Task Management project, we need to determine whether a task is overdue and requires immediate attention:

```zelyra
fn main() {
    days_remaining = -2
    is_done = false
    is_blocked = false

    # A task is overdue if days < 0 and it is not yet completed
    is_overdue = days_remaining < 0 && !is_done

    # High urgency: Overdue and not blocked by other tasks
    requires_attention = is_overdue && !is_blocked

    print("Task overdue?")
    print(is_overdue)
    print("Needs immediate intervention?")
    print(requires_attention)
}
```

### 9. Summary
- Operators combine individual values into expressive statements.
- Comparison operators yield boolean values (`Bool`).
- Logical operators (`&&`, `||`, `!`) allow the expression of intricate business rules.

### 10. Self-check review questions
1. What is the difference between `=` and `==`?
2. What value does the expression `5 > 3 && 2 > 10` evaluate to?
3. What does the `%` modulo operator compute?

---

## Chapter 8: Input and Output

### 1. What will I learn in this chapter?
In this chapter, you will learn:
- How to display information reliably on the console using `print()`.
- How to read a line interactively from the terminal with `read_console()`.
- How text and variables are formatted via string concatenation.
- How Zelyra receives input through parameters, environment variables, files, the terminal, and web routes.
- Why Zelyra requires explicit `Capabilities` for external system interactions.

### 2. Why is this topic important?
A program that cannot receive input or communicate output is useless to users. Input and output (I/O) connect the computational logic of your code with the real world. Because interactions with the keyboard, filesystem, or network introduce security risks, Zelyra regulates these operations far more strictly than older languages.

### 3. Understandable explanation without unnecessary jargon
- **Output:** The built-in `print(value)` instruction accepts numbers, booleans, strings, and structured objects, printing them directly to standard output (`stdout`).
- **Formatting:** Multiple strings and values can be combined using the plus operator `+`.
- **Input in Zelyra:**
  Programs receive input through parameters, environment variables, files, web requests, or interactively from the terminal:
  1. **Function Parameters:** Input data is passed directly during invocation.
  2. **Environment Variables:** `env("MY_KEY")` reads configuration parameters from the operating system.
  3. **Files:** `read_text("input.txt")` ingests persisted data.
  4. **Web Requests:** Form declarations (`form`) and route endpoints (`page "/user/{id}"`) process browser input.
  5. **Terminal:** `read_console("Prompt: ")` displays a prompt and reads one line. It returns `String?`: `None` means end of input, while an empty line is `Some("")`.

Terminal access is a capability. The calling function must declare `uses
Console`. Projects with a `[capabilities]` section must also set
`console = true`; new project templates leave this grant disabled by default.
Console input is intended for `zelyra run`. Web applications should use typed
requests and forms instead.
Input is not hidden; do not use `read_console()` for passwords or other
secrets.

```zelyra
fn main() uses Console {
    date = read_console("Date: ")
    match date {
        Some(value) => {
            print("Entered: " + value)
        }
        None => {
            print("No input.")
        }
    }
}
```

```toml
[capabilities]
console = true
```

### 4. Small, progressive examples

**Formatting Output:**
```zelyra
fn main() {
    task_name = "Release 1.0"
    percentage = 80
    print("Progress for " + task_name + ":")
    print(percentage)
}
```

**Input via Function Parameters and Environment Variables:**
```zelyra
fn process_task(task_name: String, priority_level: Int) {
    print("Processing: " + task_name)
    print("Priority level:")
    print(priority_level)
}

fn main() uses Environment {
    mode = env("APP_MODE")
    print("Current mode:")
    print(mode)
    process_task("Create backup", 1)
}
```

### 5. Typical errors and their causes
- **Error:** Attempting to concatenate an integer directly with a string using `+`: `"Value: " + 5`.
  *Cause:* Zelyra requires type compatibility. `5` is an `Int`, not a `String`. Print the number separately via `print(5)` or use conversion helpers.
- **Error:** Calling `env()` without declaring `uses Environment` on the enclosing function.
  *Cause:* Zelyra's capability security model requires functions interacting with the host environment to declare their capabilities explicitly.

### 6. Key takeaways
1. `print()` reliably outputs values and text to the console.
2. Inputs enter Zelyra programs via parameters, environment variables, files, the terminal, or web routes.
3. External system access requires declaring the corresponding capability (such as `uses Environment`).

### 7. Exercises
- **Level 1 (Easy):** Print a formatted contact card (name, job title, email address) using multiple `print()` statements.
- **Level 2 (Medium):** Write a function `print_task_entry(id: Int, task_name: String, is_done: Bool)` that prints all three fields cleanly formatted.
- **Level 3 (Challenging):** Write a CLI program using `read_console()` and explain when terminal input is useful and when structured web requests are a better fit.

### 8. Practical project task: Task Management
Construct an output formatting utility for our Task Management application:

```zelyra
fn print_header(section_name: String) {
    print("----------------------------------------")
    print("SECTION: " + section_name)
    print("----------------------------------------")
}

fn print_task_entry(entry_num: Int, task_name: String, is_done: Bool) {
    print("Task No: ")
    print(entry_num)
    print("Name: " + task_name)
    if is_done {
        print("Status: [X] DONE")
    } else {
        print("Status: [ ] OPEN")
    }
    print("----------------------------------------")
}

fn main() {
    print_header("TODAY'S TASKS")
    print_task_entry(1, "Work through the handbook", true)
    print_task_entry(2, "Practice Zelyra examples", false)
}
```

### 9. Summary
- Console output is performed cleanly and reliably using `print()`.
- External environment interactions are protected by explicit capabilities.
- Inputs are received through parameters, files, environment variables, `read_console()`, or web requests.

### 10. Self-check review questions
1. Which built-in function is used in Zelyra for console text output?
2. Why does accessing `env()` mandate the `uses Environment` declaration?
3. Which input channels are typical for server-based Zelyra software?

---

## Chapter 9: Decisions with Conditions

### 1. What will I learn in this chapter?
In this chapter, you will learn:
- How programs execute conditional branches using `if` and `else`.
- How to evaluate multi-way alternatives using `else if`.
- How to combine multiple conditions logically.
- How to author clean and exhaustive pattern matches with `match`.
- Common logic pitfalls with nested conditions and how to structure code cleanly.

### 2. Why is this topic important?
Without conditional branching, software would be as rigid as a music box cylinder: it would replay the exact same steps every single time. Software becomes truly capable when it dynamically reacts to changing circumstances: Is the user authenticated? Has the deadline passed? Is the balance sufficient? With `if` and `match`, you grant your code the ability to make decisions.

### 3. Understandable explanation without unnecessary jargon
- **`if` / `else`:** Evaluates a boolean condition. If it evaluates to `true`, the first block executes; otherwise, the `else` block runs:
  ```zelyra
  fn check_result(score: Int) {
      if score >= 50 {
          print("Passed!")
      } else {
          print("Unfortunately failed.")
      }
  }

  fn main() {
      check_result(75)
  }
  ```

- **`match`:** When evaluating a value against multiple known cases, `match` is far cleaner and more readable than sprawling `if / else if` cascades. In Zelyra, the compiler verifies that all possible cases are covered (exhaustiveness):
  ```zelyra
  fn show_status(status_code: Int) {
      match status_code {
          1 => { print("New") }
          2 => { print("In Progress") }
          3 => { print("Done") }
          _ => { print("Unknown Status") }
      }
  }

  fn main() {
      show_status(2)
  }
  ```

  The underscore `_` is the **wildcard pattern**: it matches all remaining values that were not explicitly listed.

### 4. Small, progressive examples

**Simple Branching:**
```zelyra
fn main() {
    open_tasks = 0
    if open_tasks == 0 {
        print("Great! All tasks are completed.")
    } else {
        print("There are still open tasks.")
    }
}
```

**Multi-Branch Decision with `match`:**
```zelyra
fn evaluate_priority(level: Int) -> String {
    match level {
        1 => { return "VERY URGENT" }
        2 => { return "NORMAL" }
        3 => { return "LOW" }
        _ => { return "UNKNOWN" }
    }
}

fn main() {
    print(evaluate_priority(1))
    print(evaluate_priority(2))
}
```

**Pattern Matching Status Codes:**
```zelyra
fn status_description(code: Int) -> String {
    match code {
        0 => { return "Draft" }
        1 => { return "Active" }
        2 => { return "Archived" }
        _ => { return "Invalid" }
    }
}

fn main() {
    print(status_description(1))
    print(status_description(99))
}
```

### 5. Typical errors and their causes
- **Error:** Omitting the wildcard `_` branch when matching over numbers.
  *Cause:* Numbers can take virtually infinite values. If you only handle `1` and `2`, the compiler signals a `non-exhaustive match`.
- **Error:** Excessive nesting (the arrow anti-pattern: `if { if { if { ... } } }`).
  *Cause:* Hard to read and debug. Flatten deep nesting using early returns or `match`.

### 6. Key takeaways
1. `if` branches based on a boolean value (`Bool`).
2. `match` checks values against patterns and enforces exhaustiveness.
3. The wildcard `_` safely catches all unlisted cases.

### 7. Exercises
- **Level 1 (Easy):** Write a function `is_adult(age: Int) -> Bool` that checks if `age >= 18`.
- **Level 2 (Medium):** Write a function `days_in_month(month: Int) -> Int` using `match` that returns the number of days for months 1 through 12 (standard 28 days for February).
- **Level 3 (Challenging):** Build a validation function that checks whether a password meets length criteria (at least 8 characters) and is not equal to `"12345678"`.

### 8. Practical project task: Task Management
Implement automated traffic-light priority classification for our Task Management system:

```zelyra
fn calculate_traffic_light(remaining_days: Int, is_finished: Bool) -> String {
    if is_finished {
        return "GREEN: Task is completed"
    } else {
        if remaining_days < 0 {
            return "RED: Deadline has passed!"
        } else {
            if remaining_days <= 2 {
                return "YELLOW: Due soon, please handle"
            } else {
                return "BLUE: On schedule"
            }
        }
    }
}

fn main() {
    print(calculate_traffic_light(5, false))
    print(calculate_traffic_light(1, false))
    print(calculate_traffic_light(-1, false))
    print(calculate_traffic_light(-1, true))
}
```

### 9. Summary
- `if` and `else` direct control flow based on boolean predicates.
- `match` facilitates clean, compiler-verified pattern branching.
- Well-structured conditional logic keeps business rules clear and maintainable.

### 10. Self-check review questions
1. When is `match` preferable to an `if / else if` chain?
2. What role does the wildcard pattern `_` perform in `match`?
3. Why does Zelyra mandate that pattern matches be exhaustive?

---

## Chapter 10: Repetition and Loops

### 1. What will I learn in this chapter?
In this chapter, you will learn:
- Why loops are indispensable in data processing and computation.
- Counted and condition-driven loops: `while condition { ... }`.
- Iteration across collections and arrays: `for element in array { ... }`.
- Unconditional loops: `loop { ... }`.
- Fine-grained loop control using `break` (exit) and `continue` (skip).
- Loop invariants (`invariant`), enabling formal mathematical proof of loop correctness.

### 2. Why is this topic important?
Computers were invented to perform repetitive, tedious tasks swiftly and without human fatigue or error. If you need to retrieve, inspect, and render 1,000 records from a database, you never write the processing logic a thousand times—you write it once inside a loop. Zelyra provides robust looping constructs along with formal invariant verification.

### 3. Understandable explanation without unnecessary jargon
Zelyra supports three primary loop forms:

1. **`for ... in`:** The safest and most concise way to traverse a collection of elements. The loop iterates over each element in sequence and halts automatically:
   ```zelyra
   fn main() {
       for num in [1, 2, 3] {
           print(num)
       }
   }
   ```

2. **`while condition`:** Continues execution as long as the condition evaluates to `true`. Useful when the exact number of iterations is not known in advance.
3. **`loop`:** A continuous loop that runs indefinitely until an inner `break` statement is encountered.

**Loop Control Commands:**
- **`break`**: Terminates the loop immediately. Execution resumes after the loop block.
- **`continue`**: Halts the current iteration and jumps directly to the next cycle.

**Loop Invariants (`invariant`):**
An invariant is a logical proposition that must remain `true` **before**, **during**, and **after** every single iteration of a loop (e.g., `invariant { counter >= 0 }`). The Zelyra verification engine (`zelyra verify`) checks loop invariants mathematically to prove that the loop cannot enter invalid states!

### 4. Small, progressive examples

**Example 1: `for ... in` across an Array**
```zelyra
fn main() {
    tasks = ["Plan", "Program", "Test", "Deploy"]
    for t in tasks {
        print("Step: " + t)
    }
}
```

**Example 2: `while` with Counter and `invariant`**
```zelyra
fn main() {
    mutable counter = 1
    while counter <= 3
        invariant { counter >= 1 }
    {
        print(counter)
        counter = counter + 1
    }
}
```

**Example 3: Targeted use of `break` and `continue`**
```zelyra
fn main() {
    for num in [1, 2, 3, 4, 5] {
        if num == 2 {
            # Skip the number 2:
            continue
        }
        if num == 4 {
            # Abort completely at 4:
            break
        }
        print(num)
    }
}
```

*Output:* Prints `1` and `3`!

### 5. Typical errors and their causes
- **Error:** Forgetting to increment the loop counter inside a `while` loop (`counter = counter + 1`).
  *Cause:* The loop condition remains permanently true, resulting in an **infinite loop** that freezes the program.
- **Error:** Off-by-one boundary errors with manual index counters.
  *Cause:* Whenever possible, use `for element in array` to avoid boundary mistakes completely.

### 6. Key takeaways
1. Prefer `for ... in` for collections and arrays.
2. `break` exits the loop immediately; `continue` proceeds to the next iteration.
3. Invariants document and mathematically prove the correctness and safety of loops.

### 7. Exercises
- **Level 1 (Easy):** Use a `while` loop to print numbers counting down from 10 to 1.
- **Level 2 (Medium):** Compute the sum of all numbers in the array `[10, 20, 30, 40]` using a `for` loop.
- **Level 3 (Challenging):** Search an array of numbers for the target value `42`. If found, print `"Found!"` and exit the loop immediately via `break`. If the number is not present, print `"Not found"` at the end.

### 8. Practical project task: Task Management
Let us apply loops to our Task Management project: we inspect the task list, mark completed tasks, and count total completions:

```zelyra
fn main() {
    tasks = ["Write specification", "Setup database", "Write tests"]
    mutable completed_count = 0

    print("Reviewing task list:")
    for t in tasks {
        if t == "Write specification" {
            print("[X] " + t)
            completed_count = completed_count + 1
        } else {
            print("[ ] " + t)
        }
    }

    print("Total completed tasks:")
    print(completed_count)
}
```

### 9. Summary
- Loops automate monotonous, repetitive processing tasks.
- `for ... in` safely iterates over arrays; `while` iterates conditionally.
- `break` and `continue` provide exact control over iteration flow.
- `invariant` enables formal mathematical verification with `zelyra verify`.

### 10. Self-check review questions
1. What is the difference between `break` and `continue`?
2. Why is `for ... in` safer when iterating over arrays than a manual `while` loop?
3. What role does an `invariant` play in program verification?

# PART III – STRUCTURING PROGRAMS

---

## Chapter 11: Functions and Procedures

### 1. What will I learn in this chapter?
In this chapter, you will learn:
- What functions are and how they make programs structured, readable, and reusable.
- How to pass parameters and declare return types using `-> Type`.
- What pure functions (*functions without side effects*) are and why they are invaluable.
- What procedures are (functions without a return value or with type `Unit`) that execute actions.
- How Zelyra fosters clear naming conventions and clean programming style.

### 2. Why is this topic important?
If you repeat every calculation and console output ten times in different places across your codebase, the dreaded "spaghetti code" phenomenon quickly takes over. When a business rule changes (for example, how a task deadline is calculated), you would have to locate and adjust all ten places—inevitably introducing bugs. Functions group a logical operation under a descriptive name: you write it once, test it thoroughly, and reuse it anywhere as often as needed.

### 3. Understandable explanation without unnecessary jargon
Think of a function like a kitchen appliance:
- You put ingredients in (**parameters**).
- The appliance processes the ingredients according to a fixed recipe (**function body**).
- At the end, a finished dish comes out (**return value**).

In Zelyra, you define functions using the `fn` keyword:
```zelyra
fn add(a: Int, b: Int) -> Int {
    return a + b
}

fn main() {
    print(add(2, 3))
}
```

If a function has no return value because it merely performs an action (such as printing a line of text to the console), its return type is `Unit` (and can simply be omitted):
```zelyra
fn print_separator() {
    print("----------------------------------------")
}

fn main() {
    print_separator()
}
```

### 4. Small, progressive examples

**Example 1: Calculation with a return value**
```zelyra
fn calculate_days_until_deadline(today_day: Int, deadline_day: Int) -> Int {
    return deadline_day - today_day
}

fn main() {
    days = calculate_days_until_deadline(10, 18)
    print(days)
}
```

**Example 2: Text formatting in a helper function**
```zelyra
fn format_task(id_text: String, text: String, done: Bool) -> String {
    mutable status_mark = "[ ]"
    if done {
        status_mark = "[X]"
    }
    return status_mark + " #" + id_text + ": " + text
}

fn main() {
    formatted = format_task("1", "Read documentation", true)
    print(formatted)
}
```

**Example 3: Procedure for header output**
```zelyra
fn show_header(user_name: String) {
    print("Logged in as: " + user_name)
    print("========================================")
}

fn main() {
    show_header("Alice")
}
```

### 5. Typical errors and their causes
- **Error:** Forgetting a `return` statement when a return type was declared.
  *Cause:* When a return type is specified, Zelyra strictly requires a matching value on every execution path.
- **Error:** Wrong argument order during a function call.
  *Cause:* Zelyra enforces strict parameter type checking. If the first parameter is an `Int`, you cannot pass a `String`.
- **Error:** Attempting to concatenate `String` and `Int` directly with `+`.
  *Cause:* In Zelyra, the `+` operator either concatenates two strings or adds two numbers of the same type. Convert or format values as strings or print them separately via `print()`.

### 6. Key takeaways
1. A function should fulfill exactly one single, clearly defined task.
2. Pure functions always produce identical outputs for identical inputs and produce no side effects.
3. Function names should be descriptive verbs or verb phrases (e.g., `calculate_difference`, `format_task`).

### 7. Exercises
- **Level 1 (Easy):** Write a function `double_val(num: Int) -> Int` that returns twice the given number.
- **Level 2 (Medium):** Write a function `is_urgent(days: Int) -> Bool` that returns `true` if fewer than 3 days remain.
- **Level 3 (Challenging):** Write a function `status_symbol(done: Bool) -> String` that returns `"[OK]"` if done and `"[OPEN]"` otherwise.

### 8. Practical project task: Task Management – Modularizing Task Display
Write a Zelyra program that formats and displays three tasks with an ID string, task name, and completion status using a reusable formatting function:

```zelyra
fn format_entry(id_text: String, name: String, done: Bool) -> String {
    mutable status_mark = "[OPEN]"
    if done {
        status_mark = "[OK]"
    }
    return status_mark + " Task " + id_text + ": " + name
}

fn main() {
    print(format_entry("1", "Collect mail", true))
    print(format_entry("2", "Pay invoice", false))
    print(format_entry("3", "Create backup", false))
}
```

### 9. Summary
- Functions divide programs into logical, manageable building blocks.
- Parameters and return types are strictly typed in Zelyra.
- Pure functions keep code maintainable, testable, and resilient against unexpected bugs.

### 10. Self-check review questions
1. What type does a function have that does not return any value?
2. Why are pure functions much easier to test than functions with global side effects?
3. What does the Zelyra compiler check during every function call?

---

## Chapter 12: Contracts and Preconditions (Design by Contract)

### 1. What will I learn in this chapter?
In this chapter, you will learn:
- What the concept of *Design by Contract* means.
- How to define preconditions using `requires`.
- How to formulate postconditions using `ensures` and the `result` keyword.
- How loop invariants are established with `invariant`.
- Why contracts are superior to defensive cascades of `if` statements.

### 2. Why is this topic important?
Bugs frequently occur because developers make unstated assumptions that are documented nowhere: "This function must never be called with a negative number!" When someone calls that function six months later with `-1`, the application fails unexpectedly. In traditional languages, developers either write endless defensive `if` checks or rely on wishful thinking and outdated comments. Zelyra elevates contracts to first-class language constructs: preconditions and postconditions are declared directly on function signatures and guaranteed to be enforced.

### 3. Understandable explanation without unnecessary jargon
A contract in Zelyra functions like a formal legal agreement between the caller and the function:
- **`requires` (Precondition):** The caller promises to supply only arguments that satisfy the condition (e.g., `value > 0`).
- **`ensures` (Postcondition):** In return, the function guarantees that its computed outcome (`result`) satisfies specific properties (e.g., `result >= 0`).

If a contract is violated, Zelyra halts immediately with a clear contract error (`E-CONTRACT-*`), identifying exactly which party breached the agreement.

### 4. Small, progressive examples

**Example 1: Precondition with requires**
```zelyra
fn divide(numerator: Int, denominator: Int) -> Int
    requires { denominator != 0 }
{
    return numerator / denominator
}

fn main() {
    result_val = divide(100, 4)
    print(result_val)
}
```

**Example 2: Combining precondition and postcondition**
```zelyra
fn adjust_priority(current_level: Int, delta: Int) -> Int
    requires { current_level >= 1 && delta >= 0 }
    ensures { result >= 1 }
{
    new_level = current_level + delta
    return new_level
}

fn main() {
    p = adjust_priority(2, 1)
    print(p)
}
```

**Example 3: Loop invariant**
```zelyra
fn count_up_to(limit: Int) -> Int
    requires { limit >= 0 }
    ensures { result == limit }
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
    print(count_up_to(5))
}
```

### 5. Typical errors and their causes
- **Error:** Attempting to validate raw user input via `requires`.
  *Cause:* Contracts are internal safeguards against programmer error and logic defects, not input sanitizers for untrusted user inputs. For user input, use form validation rules (`form`) or the `Result` type.
- **Error:** Writing an `ensures` clause that the function implementation logically cannot satisfy.
  *Cause:* If the function returns `-5` but the `ensures` clause demands `{ result >= 0 }`, the postcondition check will fail.

### 6. Key takeaways
1. `requires` protects the function against invalid inputs provided by the caller.
2. `ensures` guarantees the caller a valid outcome referenced via `result`.
3. Contracts transform implicit mental assumptions into verifiable, living specifications.

### 7. Exercises
- **Level 1 (Easy):** Write a function `naive_sqrt(x: Int) -> Int` with the precondition `requires { x >= 0 }`.
- **Level 2 (Medium):** Write a function `clamp_val(val: Int, min_val: Int, max_val: Int) -> Int` with preconditions requiring `min_val <= max_val` and appropriate postconditions.
- **Level 3 (Challenging):** Secure a function `percentage(part: Int, total_count: Int) -> Int` ensuring division by zero never occurs and the result is strictly between 0 and 100.

### 8. Practical project task: Task Management – Task Progress Calculator
Write a contract-guaranteed function for task progress in task management:
```zelyra
fn calculate_progress(completed: Int, total_count: Int) -> Int
    requires { total_count > 0 && completed >= 0 && completed <= total_count }
    ensures { result >= 0 && result <= 100 }
{
    return (completed * 100) / total_count
}

fn main() {
    ratio = calculate_progress(3, 4)
    print(ratio)
}
```

### 9. Summary
- Contracts (`requires`, `ensures`) document and enforce programming assumptions at runtime.
- `result` in `ensures` refers to the computed return value of the function.
- Loop invariants verify the integrity of the loop state across every iteration.

### 10. Self-check review questions
1. When is a `requires` condition evaluated: before or after function execution?
2. What does the keyword `result` refer to in a contract?
3. Why does a contract not replace an HTML form validation pattern?

---

## Chapter 13: Collections, Lists, and Dictionaries (Arrays & Maps)

### 1. What will I learn in this chapter?
In this chapter, you will learn:
- How to store multiple homogeneous values in an array (`Type[]`).
- The most important built-in collection functions: `len`, `append`, `contains`, `first`, `last`.
- How to iterate through collections using `for .. in`.
- How associative key-value collections are declared and used with `Map<Key, Value>`.
- The most important map functions: `get`, `put`, `contains`, `keys`, `values`.
- Why Zelyra's `get()` always returns a safe `Option` to eliminate null pointer crashes.

### 2. Why is this topic important?
An application capable of storing only isolated values would be practically useless. In real-world software, we almost always work with collections: lists of tasks, emails, or table rows fetched from a database.
Sometimes we need to access items by sequential order (lists or arrays). Very often, however, we need to look up data directly using a unique key—such as user preferences by user ID, localized translation dictionaries, or telephone country codes. For these use cases, Zelyra provides typed dictionaries (`Map`).

### 3. Understandable explanation without unnecessary jargon

#### Part A: Arrays – The Ordered List
An array is like a compartmentalized organizer or a pillbox with numbered slots: every slot has a fixed position and holds exactly one element. In Zelyra, all elements within the same container must have the identical type (`Int[]`, `String[]`, etc.):

```zelyra
fn main() {
    numbers: Int[] = [10, 20, 30, 40]
    print(len(numbers))
}
```

Zelyra provides powerful built-in functions for arrays:
- `len(items)`: Returns the number of elements.
- `append(items, value)`: Returns a new array with the element appended.
- `first(items)`: Returns the first element as an `Option`.
- `last(items)`: Returns the last element as an `Option`.
- `contains(items, value)`: Checks whether an element is present (`Bool`).

#### Part B: Maps – Associative Key-Value Dictionaries
A `Map` maps each unique key (*Key*) to exactly one corresponding value (*Value*).
- **Type notation:** `Map<KeyType, ValueType>`, for example `Map<String, Int>` or `Map<String, String>`.
- **Literal syntax:** `Map { "key": value }`
- **Key types:** The documented types are `Int`, `UInt`, `String`, `Bool`, `Char`, and `Timestamp`. JSON requires String keys.

```zelyra
fn main() {
    country_codes: Map<String, Int> = Map {
        "de": 49
        "at": 43
        "ch": 41
    }
}
```

Key operations on maps:
- **`get(map, key)`**: Looks up a key. Because a key might not exist, `get()` **always** returns a safe `Option<Value>` (`Some(val)` or `None`)—never a null pointer crash!
- **`put(map, key, value)`**: Inserts a key-value pair or updates an existing key. Following Zelyra's functional immutability principles, `put()` returns a new, updated Map.
- **`contains(map, key)`**: Returns `true` if the key exists in the map.
- **`keys(map)`**: Returns all keys as a typed array (`KeyType[]`).
- **`values(map)`**: Returns all values as a typed array (`ValueType[]`).

### 4. Small, progressive examples

**Example 1: Creating and iterating over an array**
```zelyra
fn main() {
    task_ids: Int[] = [101, 102, 103, 104]
    for id in task_ids {
        print(id)
    }
}
```

**Example 2: Appending elements to an array**
```zelyra
fn main() {
    mutable items: Int[] = [1, 2, 3]
    items = append(items, 4)
    print(len(items))
}
```

**Example 3: Creating, updating, and querying a Map**
```zelyra
fn main() {
    prices: Map<String, Int> = Map {
        "Coffee": 3
        "Tea": 2
    }
    
    # Add a new item
    mutable current_prices = put(prices, "Cake", 4)
    # Update existing price
    current_prices = put(current_prices, "Coffee", 4)
    
    # Safely query using match
    match get(current_prices, "Coffee") {
        Some(price) => {
            print("Coffee price in Euro:")
            print(price)
        }
        None => {
            print("Item not found.")
        }
    }
    
    # Output all keys
    print(keys(current_prices))
}
```

**Example 4: Checking key existence in a Map**
```zelyra
fn main() {
    settings: Map<String, Bool> = Map {
        "dark_mode": true
        "notifications": false
    }
    
    if contains(settings, "dark_mode") {
        print("Dark mode setting is explicitly configured.")
    }
}
```

### 5. Typical errors and their causes
- **Error:** Attempting to mix different types within a single array or map (e.g., `[1, "Hello"]`).
  *Cause:* Zelyra is strictly homogeneous. All elements in an array, and all keys and values in a map, must conform to their declared types.
- **Error:** Treating `map.get("key")` as a raw value without unwrapping the `Option`.
  *Cause:* Zelyra guarantees compile-time safety. Because a key might not be in the dictionary, the compiler forces you to handle `Some` and `None` using `match` or default values.
- **Error:** Using non-scalar types (such as arrays) as Map keys.
  *Cause:* Map keys must use a supported comparable type (`Int`, `UInt`, `String`, `Bool`, `Char`, `Timestamp`). JSON supports only String keys.

### 6. Key takeaways
1. Arrays in Zelyra are strictly homogeneous: `Int[]`, `String[]`, `Bool[]`.
2. Iteration is performed safely and cleanly with `for element in collection { ... }`.
3. `Map<Key, Value>` stores unique key-value associations.
4. `get(map, key)` always returns an `Option` (`Some` or `None`), preventing runtime exceptions.
5. `put(map, key, value)` functionally produces a new, updated dictionary.

### 7. Exercises
- **Level 1 (Easy):** Create an array containing three strings and print each element using a `for` loop.
- **Level 2 (Medium):** Create a `Map<String, Int>` with three product names and their prices. Query both an existing and a non-existing item using `get()` and `match`.
- **Level 3 (Challenging):** Write a function `count_words(words: String[]) -> Map<String, Int>` that counts how often each word occurs in a list and returns the result as a Map.

### 8. Practical project task: Task Management – Priority Registry
Build a priority lookup table for our task management system:
```zelyra
fn show_priority(priorities: Map<String, Int>, task_name: String) {
    match get(priorities, task_name) {
        Some(level) => {
            print("Priority for " + task_name + ":")
            print(level)
        }
        None => {
            print("No priority registered for: " + task_name)
        }
    }
}

fn main() {
    prio_map: Map<String, Int> = Map {
        "Database migration": 1
        "Tweak CSS styles": 3
        "Write documentation": 2
    }
    
    show_priority(prio_map, "Database migration")
    show_priority(prio_map, "Coffee break")
}
```

### 9. Summary
- Arrays (`Type[]`) store ordered sequences of homogeneous elements.
- Maps (`Map<Key, Value>`) store key-value dictionaries with safe `get()`, `put()`, `contains()`, `keys()`, and `values()`.
- Both collection types are fully type-safe and integrate seamlessly with Zelyra's `Option` type system.

### 10. Self-check review questions
1. What type does the expression `["A", "B", "C"]` evaluate to?
2. Why does `append([1, 2], "Three")` fail at compile time?
3. How do you efficiently check whether a value exists inside an array?

---

## Chapter 14: Creating Custom Data Types (Records & Tables)

### 1. What will I learn in this chapter?
In this chapter, you will learn:
- How to create your own domain-specific data types.
- What nominal types (`type TaskId = Id`) are and how they prevent accidental ID confusion.
- What enum and sum types mean, and which variants are currently built into Zelyra.
- How records are defined in Zelyra as a `table` with attributes (`id: Id primary auto`).
- Why strong typing fundamentally elevates software quality.

### 2. Why is this topic important?
In fragile software architectures, almost everything is represented as a primitive integer or string (an anti-pattern known as "Primitive Obsession"). If a function expects `verify(user_id: Int, task_id: Int)` and you accidentally swap the arguments, a compiler using raw `Int` types will never notice—and your application might assign sensitive records to the wrong user. With distinct custom types, Zelyra distinguishes between a `UserId` and a `TaskId` right at compile time.

### 3. Understandable explanation without unnecessary jargon
Custom data types allow you to model real-world concepts accurately:
1. **Domain Type Aliases:**
   ```zelyra
   type TaskId = Id

   fn main() {
       print("Type alias TaskId active")
   }
   ```

   The alias gives the ID a domain name. It has no separate runtime identity; business rules need validation within the actual supported checking scope.
2. **Enum and sum types:**

   An enum describes a value that must be one of a fixed set of named variants. A variant may also carry a value; this is why enums are often called sum types. The general concept can be sketched as `enum State { Draft, Active, Archived }`, but this is **conceptual notation, not supported Zelyra source syntax**. User-defined enum declarations are outside the current documented language surface. Zelyra does provide the built-in sum types `Option<T>` and `Result<T, E>`; use `match` to handle their alternatives.
3. **Tables as Structured Data Types:**
   ```zelyra
   table tasks {
       id: Id primary auto
       description: String required
       done: Bool
   }

   fn main() {
       print("Table schema defined")
   }
   ```

   This schema does not merely specify a database table; in Zelyra, it simultaneously declares the in-memory data type for a task!

### 4. Small, progressive examples

**Example 1: Defining a custom type alias for IDs**
```zelyra
type TaskId = Id

fn main() {
    print("Type alias defined successfully")
}
```

**Example 2: Defining a data model as a table**
```zelyra
table tasks {
    id: Id primary auto
    description: String required
    done: Bool
}

fn main() {
    print("Table schema for tasks defined")
}
```

**Example 3: Working with typed attributes in functions**
```zelyra
table projects {
    id: Id primary auto
    name: String required
    active: Bool
}

fn show_project_status(p_name: String, p_active: Bool) {
    mutable status_text = "Paused"
    if p_active {
        status_text = "Active"
    }
    print("Project: " + p_name + " [" + status_text + "]")
}

fn main() {
    show_project_status("Web Portal", true)
}
```

### 5. Typical errors and their causes
- **Error:** Naming a table field with a reserved keyword such as `title`, `list`, or `detail`.
  *Cause:* In Zelyra, these identifiers are reserved for queries and UI views. Use descriptive names like `name`, `description`, or `subject` instead.
- **Error:** Declaring the primary key with `primary key` instead of `primary`.
  *Cause:* In Zelyra, the standard column specification is `id: Id primary auto`.

### 6. Key takeaways
1. Custom data types reflect your business domain and eliminate accidental parameter swapping.
2. `table` declarations in Zelyra act as both database schemas and native language types.
3. Reserved words (`title`, `list`, `action`, `field`, etc.) must never be used as column or variable names.

### 7. Exercises
- **Level 1 (Easy):** Create a nominal type alias `type UserId = Id`.
- **Level 2 (Medium):** Model a table `categories` with `id: Id primary auto` and `category_name: String required`.
- **Level 3 (Challenging):** Model a table `notes` that links to a task via a field `task_id: Id`.

### 8. Practical project task: Task Management – Core Task Schema
Define the complete core Zelyra schema for our task management system:

```zelyra
table tasks {
    id: Id primary auto
    name: String required
    description: String
    priority: Int
    is_done: Bool
}

fn main() {
    print("Core schema of task management active.")
}
```

### 9. Summary
- Custom types give raw data unambiguous meaning and context.
- `table` seamlessly unifies schema declaration and static type modeling in the language.
- Static typing catches logical mismatches while you write your code.

### 10. Self-check review questions
1. What advantage does `type TaskId = Id` offer compared to a simple `Int`?
2. Why must table fields in Zelyra always possess an explicit type?
3. Which words must be avoided when naming fields?

---

## Chapter 15: Modules and Code Organization

### 1. What will I learn in this chapter?
In this chapter, you will learn:
- How a standard Zelyra project is structured on disk.
- The central role of the `zelyra.toml` project configuration manifest.
- How to separate code into logical concerns (schema, business logic, views).
- How the Zelyra CLI verifies and builds cohesive multi-file projects.
- The current development status and roadmap for modules and imports.

### 2. Why is this topic important?
When starting out, it is tempting to write an entire program in a single file. However, as your task management application expands—incorporating database tables, dozens of business functions, web forms, and validation rules—navigating a monolithic 2,000-line file becomes unmanageable. Professional software engineering demands structuring code so that every team member can locate schemas, business logic, and UI definitions instantly.

### 3. Understandable explanation without unnecessary jargon
A professional Zelyra project follows a clean, standardized directory layout:
- `zelyra.toml`: The birth certificate and configuration manifest of the project. It declares the package name, version, and security capabilities.
- `src/schema.zyl`: Contains all table declarations (`table`) and custom domain types.
- `src/main.zyl`: Contains the primary entry point (`fn main()`) and application orchestration.

```toml
[package]
name = "task_planner"
version = "0.1.0"
authors = ["Developer <dev@example.com>"]

[capabilities]
filesystem = false
database = true
```

*Language compatibility note:* Zelyra supports explicit project-relative imports such as `import "src/models.zyl" as models`. These imports connect source files inside one project; they do not provide a package registry or dynamic runtime imports. The dependency graph and deployment closure remain experimental and may be incomplete. The language compatibility line remains 0.1.

### 4. Small, progressive examples

**Example 1: A clean main entry point**
```zelyra
fn main() {
    print("Zelyra task system ready.")
}
```

**Example 2: Separating logic functions**
```zelyra
fn format_system_status(status_text: String) -> String {
    return "[STATUS] " + status_text
}

fn main() {
    print(format_system_status("Database connected"))
}
```

**Example 3: Declaring capabilities in the project manifest**
In `zelyra.toml`, you explicitly configure which system resources the project is allowed to request. If your application accesses a database, `database = true` must be enabled under `[capabilities]`.

### 5. Typical errors and their causes
- **Error:** Attempting to use an `import` syntax from other languages such as Python, JavaScript, or Rust.
  *Cause:* Zelyra uses project-relative imports such as `import "src/models.zyl" as models`. Access supported public declarations through the module name; there is no package registry.
- **Error:** Deleting `zelyra.toml` or executing CLI commands from outside the project root directory.
  *Cause:* Commands like `zelyra run` look for `zelyra.toml` in the current working directory to configure capabilities and compilation paths.

### 6. Key takeaways
1. `zelyra.toml` governs project metadata, dependencies, and capability security policies.
2. Maintain a clean separation of concerns: data models (`table`), business logic (`fn`), and UI views.
3. A clean project layout prevents accidental coupling and speeds up teamwork.

### 7. Exercises
- **Level 1 (Easy):** Generate a new project skeleton using `zelyra new task_app` and explore the generated files.
- **Level 2 (Medium):** Update `zelyra.toml` with a project description and increment the version number to `0.3.0`.
- **Level 3 (Challenging):** Write an application structured into three separate functions handling initialization, business processing, and output reporting.

### 8. Practical project task: Task Management – Project Structure
Establish the task management architecture with the following implementation in `main.zyl`:

```zelyra
table tasks {
    id: Id primary auto
    name: String required
    done: Bool
}

fn start_system() {
    print("========================================")
    print("   TASK MANAGER SUCCESSFULLY STARTED")
    print("========================================")
}

fn main() {
    start_system()
}
```

### 9. Summary
- Projects are configured and secured via `zelyra.toml`.
- Zelyra evaluates project files as a cohesive, strongly typed whole.
- Modular architecture ensures sustainable scalability and seamless team collaboration.

### 10. Self-check review questions
1. Which file contains the metadata and capability definitions of a Zelyra project?
2. Why is decoupling the data schema from execution logic recommended?
3. How does the Zelyra CLI compile and verify an entire project at once?

# PART IV – SAFETY AND ERROR HANDLING

---

## Chapter 16: Error Types and Their Causes

### 1. What will I learn in this chapter?
In this chapter, you will learn:
- The four primary categories of errors encountered in software development.
- The differences between syntax errors, type errors, contract/capability errors, and logic errors.
- How the Zelyra compiler guides you with precise diagnostics detailing line numbers, column locations, and actionable hints.
- Why early compile-time error detection saves substantial time and prevents costly bugs in production.

### 2. Why is this topic important?
Errors are a natural, everyday part of programming. Even seasoned software engineers encounter dozens of compiler errors every day. The difference between struggling and proficient developers is not making zero mistakes, but being able to read and understand error diagnostics quickly. Zelyra was intentionally designed so that the vast majority of errors are caught right at compile time (`zelyra check`) long before your application ever reaches an end user.

### 3. Understandable explanation without unnecessary jargon
In software engineering, we distinguish between four fundamental classes of errors:
1. **Syntax Errors (`E-LEX-*`, `E-PARSE-*`):** You violated the grammar of the programming language—analogous to a punctuation or spelling mistake in human language (such as forgetting a closing curly bracket).
2. **Type Errors (`E-TYPE-*`):** The grammar is syntactically valid, but the data types do not fit together (such as attempting to add text to an integer).
3. **Contract and Capability Errors (`E-CONTRACT-*`, `E-CAP-*`):** A pre-agreed precondition was breached, or a function attempted to access restricted system resources (such as reading the filesystem without permission).
4. **Logic Errors:** The program compiles and executes without crashing, but produces incorrect results because the algorithm was wrong (e.g., adding a discount instead of subtracting it).

### 4. Small, progressive examples

**Example 1: Typical syntax error (and reading compiler output)**
When you write clean, matching brackets:
```zelyra
# Syntactically correct:
fn correct_brackets() {
    print("All brackets are closed.")
}

fn main() {
    correct_brackets()
}
```

**Example 2: Capability error (Security capability verification)**
If a function attempts to access restricted system resources without declaring the required permission, Zelyra stops immediately:
```zelyra
fn read_file(path: String) -> String
    uses FileSystem
{
    return read_text(path)
}

fn main() {
    print("File access properly declared.")
}
```

**Example 3: Exposing logic errors through contracts**
```zelyra
fn calculate_discount_price(original: Int, discount: Int) -> Int
    requires { original >= 0 && discount >= 0 && discount <= original }
    ensures { result <= original }
{
    return original - discount
}

fn main() {
    price = calculate_discount_price(100, 20)
    print(price)
}
```

### 5. Typical errors and their causes
- **Error:** Ignoring compiler diagnostics without looking at the line and column indicators.
  *Cause:* Zelyra points you directly to the exact source location where the issue originated.
- **Error:** Using `+` between strings and numbers.
  *Cause:* Zelyra strictly enforces static type safety. Format values as strings or print them as separate arguments.

### 6. Key takeaways
1. A compiler error is not a failure—it is an automated, instant code review.
2. The earlier an error is caught (compile time vs. runtime), the safer and cheaper your application is to maintain.
3. Contracts (`requires`, `ensures`) turn subtle, creeping logic bugs into immediate, reproducible contract violations.

### 7. Exercises
- **Level 1 (Easy):** Intentionally trigger a syntax error (e.g., omitting a bracket) and examine the output of `zelyra check`.
- **Level 2 (Medium):** Write a function with a type mismatch error and resolve it following the compiler's diagnostic hints.
- **Level 3 (Challenging):** Write an age verification function equipped with contracts that immediately rejects invalid ages (e.g., negative numbers).

### 8. Practical project task: Task Management – Fault-Tolerant Task Duration Logging
Write a verified function for task management that prevents negative hours from being recorded:
```zelyra
fn log_hours(previous_hours: Int, new_hours: Int) -> Int
    requires { previous_hours >= 0 && new_hours >= 0 }
    ensures { result >= previous_hours }
{
    return previous_hours + new_hours
}

fn main() {
    total_hours = log_hours(5, 3)
    print(total_hours)
}
```

### 9. Summary
- Zelyra categorizes errors into syntax, type, contract/capability, and logic categories.
- Static checking and contract enforcement catch defects before code ever deploys.

### 10. Self-check review questions
1. What error code family (`E-...`) is generated when a keyword is misspelled or missing?
2. Why can a program compile completely without errors and still calculate incorrect values?
3. How do formal contracts help uncover hidden logical reasoning bugs?

---

## Chapter 17: Errors as Values – The Result Pattern

### 1. What will I learn in this chapter?
In this chapter, you will learn:
- What the `Result` pattern is and why Zelyra avoids uncontrolled exceptions.
- The two variants: `Ok(value)` for success and `Err(message)` for failure.
- How to deconstruct results safely using `match`.
- Why treating errors as regular values makes your software transparent, predictable, and crash-proof.

### 2. Why is this topic important?
In many traditional languages (like Java, Python, or PHP), functions signal failure by throwing exceptions. If an exception goes uncaught anywhere in the call stack, the entire web request crashes with an HTTP 500 error. In Zelyra, there are no uncontrolled runtime exceptions: whenever an operation can fail (such as a missing record or an invalid input), it explicitly returns a `Result<T, E>`. The compiler guarantees that you handle both success and error outcomes before your code can run.

### 3. Understandable explanation without unnecessary jargon
Imagine receiving a delivery parcel:
- If the courier delivers the package successfully, you open it and find the item inside: `Ok(content)`.
- If the address does not exist, you receive a return slip stating the reason: `Err("Address unknown")`.

A parcel never "explodes" in your face—you simply inspect what arrived:
```zelyra
fn divide_safe(a: Int, b: Int) -> Result<Int, String> {
    if b == 0 {
        return Err("Division by zero is not allowed")
    }
    return Ok(a / b)
}

fn main() {
    print("Safe division defined.")
}
```

### 4. Small, progressive examples

**Example 1: Defining and handling a function with Result**
```zelyra
fn check_priority(level: Int) -> Result<Int, String> {
    if level < 1 {
        return Err("Priority too low (minimum 1)")
    }
    if level > 3 {
        return Err("Priority too high (maximum 3)")
    }
    return Ok(level)
}

fn main() {
    res = check_priority(2)
    match res {
        Ok(level) => {
            print("Valid priority:")
            print(level)
        }
        Err(err_msg) => {
            print(err_msg)
        }
    }
}
```

**Example 2: Handling the error case**
```zelyra
fn get_balance(pin: Int) -> Result<Int, String> {
    if pin != 1234 {
        return Err("Incorrect PIN!")
    }
    return Ok(500)
}

fn main() {
    attempt = get_balance(9999)
    match attempt {
        Ok(amount) => {
            print(amount)
        }
        Err(err_msg) => {
            print("Denied: " + err_msg)
        }
    }
}
```

**Example 3: Safe value validation**
```zelyra
fn check_task_name_length(task_name: String) -> Result<String, String> {
    if task_name == "" {
        return Err("Task name must not be empty.")
    }
    return Ok(task_name)
}

fn main() {
    outcome = check_task_name_length("Project Report")
    match outcome {
        Ok(t) => {
            print("Valid name: " + t)
        }
        Err(e) => {
            print("Error: " + e)
        }
    }
}
```

### 5. Typical errors and their causes
- **Error:** Attempting to access the inner value directly without unpacking it using `match`.
  *Cause:* `Result<T, E>` is a container type. You must unwrap it through pattern matching.
- **Error:** Omitting one of the two branches (`Ok` or `Err`) in a `match` expression.
  *Cause:* Zelyra enforces exhaustive pattern matching to ensure unhandled failure paths are impossible.

### 6. Key takeaways
1. `Result<T, E>` makes error handling explicit by treating errors as regular return values.
2. `Ok(v)` represents success carrying a value, while `Err(e)` represents an explained failure.
3. Pattern matching with `match` requires handling all outcomes, preventing silent uncaught crashes.

### 7. Exercises
- **Level 1 (Easy):** Write a function `check_even(num: Int) -> Result<Int, String>` that returns `Ok(num)` if divisible by 2, or `Err("Odd")` otherwise.
- **Level 2 (Medium):** Write a function `validate_username(name: String) -> Result<String, String>` that rejects empty strings or `"admin"`.
- **Level 3 (Challenging):** Implement an arithmetic division function that validates inputs and returns twice the result on success.

### 8. Practical project task: Task Management – Validating Task Creation
Create a robust validation function for adding new tasks:
```zelyra
fn create_task_checked(name: String, priority: Int) -> Result<String, String> {
    if name == "" {
        return Err("Name must not be empty!")
    }
    if priority < 1 {
        return Err("Priority must be at least 1!")
    }
    return Ok("Task [" + name + "] created successfully.")
}

fn main() {
    item1 = create_task_checked("Complete documentation", 1)
    match item1 {
        Ok(msg) => {
            print(msg)
        }
        Err(err_msg) => {
            print("Error: " + err_msg)
        }
    }

    item2 = create_task_checked("", 0)
    match item2 {
        Ok(msg) => {
            print(msg)
        }
        Err(err_msg) => {
            print("Error: " + err_msg)
        }
    }
}
```

### 9. Summary
- The `Result` pattern replaces uncontrolled exceptions with strictly typed values.
- Zelyra guarantees at compile time that failure cases cannot be silently ignored.

### 10. Self-check review questions
1. What do `T` and `E` represent in `Result<T, E>`?
2. Why do unhandled errors in Zelyra never cause unexpected application crashes?
3. How do you safely extract the payload from a `Result` value?

---

## Chapter 18: Nothingness Does Not Exist – Working Safely with Option

### 1. What will I learn in this chapter?
In this chapter, you will learn:
- Why Tony Hoare described the invention of `null` as his "billion-dollar mistake."
- How Zelyra completely eliminates `null` and replaces it with the type-safe `Option<T>` (shorthand `T?`).
- How to wrap values in `Some(value)` and represent their absence with `None`.
- How built-in collection operations like `first` and `last` leverage `Option`.

### 2. Why is this topic important?
In languages like JavaScript, Java, PHP, or C, the value `null` lurks everywhere. Whenever code inadvertently calls a method or reads an attribute on a `null` reference, the entire application crashes with a dreaded `NullPointerException` or `TypeError: Cannot read properties of null`. In Zelyra, `null` simply does not exist. Standard values are strictly guaranteed to exist. Whenever a value may legitimately be absent, it must be explicitly typed as an `Option`.

### 3. Understandable explanation without unnecessary jargon
Think of an `Option` as a gift box:
- The box may contain a present: `Some("Smartphone")`.
- Or the box is completely empty: `None`.

You cannot accidentally use the gift without first unpacking the box using `match`:
```zelyra
fn find_task_by_id(id: Int) -> Option<String> {
    if id == 42 {
        return Some("Set up server")
    }
    return None
}

fn main() {
    print("Task search defined.")
}
```

### 4. Small, progressive examples

**Example 1: Creating and matching an Option**
```zelyra
fn main() {
    found: Option<String> = Some("Zelyra 0.1 Handbook")
    match found {
        Some(item_name) => {
            print("Found: " + item_name)
        }
        None => {
            print("No match found")
        }
    }
}
```

**Example 2: Safe array inspection with first() and last()**
Accessing an empty collection in Zelyra never crashes your software—it cleanly returns `None`:
```zelyra
fn main() {
    my_numbers: Int[] = [100, 200, 300]
    first_item = first(my_numbers)
    match first_item {
        Some(val) => {
            print("First value:")
            print(val)
        }
        None => {
            print("The list is empty!")
        }
    }
}
```

**Example 3: Providing fallback defaults with Option**
```zelyra
fn task_name_or_default(opt_name: Option<String>) -> String {
    match opt_name {
        Some(t) => {
            return t
        }
        None => {
            return "Untitled"
        }
    }
}

fn main() {
    print(task_name_or_default(Some("Important Meeting")))
    print(task_name_or_default(None))
}
```

### 5. Typical errors and their causes
- **Error:** Assuming that an `Option<String>` can be used directly as a `String`.
  *Cause:* The box is not the gift. You must unpack it using pattern matching (`match`).
- **Error:** Attempting to assign `None` to a standard non-optional variable.
  *Cause:* Regular variables are guaranteed to always contain a concrete value.

### 6. Key takeaways
1. Zelyra completely eliminates `null`, rendering `NullPointerException` crashes impossible.
2. Whenever a value may be absent, declare it as `Option<T>` or `T?`.
3. `Some(x)` wraps a present value, while `None` explicitly indicates absence.

### 7. Exercises
- **Level 1 (Easy):** Write a function `find_partner(name: String) -> Option<String>` that returns `Some("Juliet")` when given `"Romeo"`, and `None` otherwise.
- **Level 2 (Medium):** Retrieve the last element of an integer array using `last()` and print its value or an empty-list notice.
- **Level 3 (Challenging):** Write a search function that searches an array of IDs and returns the matching index position as `Option<Int>`.

### 8. Practical project task: Task Management – Safe Task Detail Lookup
Implement safe task description retrieval for our task management system:
```zelyra
fn find_task_description(id: Int) -> Option<String> {
    if id == 1 {
        return Some("Create database schema for Zelyra")
    }
    if id == 2 {
        return Some("Design web interface")
    }
    return None
}

fn main() {
    lookup1 = find_task_description(1)
    match lookup1 {
        Some(text) => {
            print("Task 1: " + text)
        }
        None => {
            print("Task 1 not found!")
        }
    }

    lookup99 = find_task_description(99)
    match lookup99 {
        Some(text) => {
            print("Task 99: " + text)
        }
        None => {
            print("Task 99 not found!")
        }
    }
}
```

### 9. Summary
- `Option<T>` shields applications from the disastrous errors caused by unexpected null references.
- Zelyra statically enforces that every possible `None` condition is addressed before execution.

### 10. Self-check review questions
1. Why is there no `null` keyword or concept in Zelyra?
2. What is the fundamental difference between `String` and `Option<String>`?
3. Which two branches must always be present when pattern matching an `Option`?

---

## Chapter 19: Testing and Quality Assurance

### 1. What will I learn in this chapter?
In this chapter, you will learn:
- Why automated testing forms the indispensable backbone of reliable, sustainable software.
- How Zelyra's verification tool (`zelyra verify`) formally proves function contracts.
- How to design and structure assertion-based test suites.
- The philosophy of *Test-Driven Development* (TDD).
- How assertions and project-specific test runners work, and which test commands the current CLI provides.

### 2. Why is this topic important?
Manual testing—clicking around in a browser or manually calling functions—is tedious, inconsistent, and error-prone. As software grows in complexity, even minor modifications in one subsystem inevitably introduce regressions in seemingly unrelated modules. Automated tests act as an automated safety net, proving that existing business logic continues to function according to specification days, months, and years into the future.

### 3. Understandable explanation without unnecessary jargon
Quality assurance in Zelyra rests on two powerful pillars:
1. **Formal Contract Verification with `zelyra verify`:**
   The compiler mathematically checks whether preconditions and postconditions (`requires`, `ensures`) hold under all possible valid inputs.
2. **Automated Assertion Testing:**
   Test routines supply sample inputs to business functions and compare actual results with expected outcomes. The built-in `assert(condition, message)` check reports a failure when its condition is false.

The current CLI reference does not list a `zelyra test` subcommand. Keep
project-specific test routines in an executable `.zyl` file and run them with
`zelyra run <file.zyl>`. Use `zelyra check` before execution and
`zelyra verify` for the supported formal contract checks.

### 4. Small, progressive examples

**Example 1: A direct assertion**
~~~zelyra
fn main() {
    result = 6 * 7
    assert(result == 42, "Expected the multiplication result to be 42")
}
~~~

**Example 2: A lightweight assertion helper**
```zelyra
fn assert_test(test_name: String, condition: Bool) {
    if condition {
        print("[PASS] " + test_name)
    } else {
        print("[FAIL] " + test_name)
    }
}

fn double_val(x: Int) -> Int {
    return x * 2
}

fn main() {
    assert_test("Double 5 is 10", double_val(5) == 10)
    assert_test("Double 0 is 0", double_val(0) == 0)
}
```

**Example 3: Testing Option return values**
```zelyra
fn is_adult(age: Int) -> Option<Bool> {
    if age < 0 {
        return None
    }
    return Some(age >= 18)
}

fn main() {
    test1 = is_adult(20)
    match test1 {
        Some(ok) => {
            if ok {
                print("[PASS] 20 years is of age")
            } else {
                print("[FAIL] Unexpected state")
            }
        }
        None => {
            print("[FAIL] Invalid age")
        }
    }
}
```

**Example 4: Protecting logic with formal contract verification**
```zelyra
fn calculate_overtime(hours: Int, regular_hours: Int) -> Int
    requires { hours >= 0 && regular_hours >= 0 }
    ensures { result >= 0 }
{
    if hours > regular_hours {
        return hours - regular_hours
    }
    return 0
}

fn main() {
    print(calculate_overtime(45, 40))
}
```

### 5. Typical errors and their causes
- **Error:** Testing only the "happy path" and ignoring boundary conditions (zero values, negative numbers, empty collections).
  *Cause:* Most real-world regressions occur at the extreme boundaries of input ranges.
- **Error:** Forgetting to run automated checks after refactoring code.
  *Cause:* Always make `zelyra check` and your test suites an automatic habit whenever you edit source files.

### 6. Key takeaways
1. Untested code is broken code that has simply not failed in public yet.
2. `zelyra verify` mathematically evaluates function contracts directly at the language level.
3. High-value tests deliberately target edge cases, boundaries, and failure paths.

### 7. Exercises
- **Level 1 (Easy):** Write three test cases for a function `add(a: Int, b: Int) -> Int`.
- **Level 2 (Medium):** Write assertion tests for the `Option`-based lookup function from Chapter 18.
- **Level 3 (Challenging):** Implement a comprehensive test suite for a function that checks if a task name satisfies business rules (non-empty, minimum length).

### 8. Practical project task: Task Management – Test Runner for Task Business Logic
Construct a lightweight test suite for the core business rules of the task management application:
```zelyra
fn test_case(description: String, ok: Bool) {
    if ok {
        print("OK: " + description)
    } else {
        print("ERROR: " + description)
    }
}

fn matches_priority(p: Int) -> Bool {
    return p == 1
}

fn main() {
    print("Starting test suite: Task logic")
    test_case("Priority 1 matches", matches_priority(1) == true)
    test_case("Priority 2 is ignored", matches_priority(2) == false)
    print("Test suite completed.")
}
```

### 9. Summary
- Automated verification safeguards software reliability across long development cycles.
- Combining `zelyra verify` with assertion test suites provides comprehensive confidence in business logic.

### 10. Self-check review questions
1. What does the term "regression" mean in software development?
2. What responsibility does the CLI command `zelyra verify` perform?
3. Why are boundary values (e.g., 0 or maximum capacity) critical in test design?

# PART V – PRACTICAL DATA PROCESSING

---

## Chapter 20: Working with Files

### 1. What will I learn in this chapter?
In this chapter, you will learn:
- How to create, read, list, and delete text files using Zelyra.
- Why file operations in Zelyra strictly require the `uses FileSystem` capability.
- The most important standard library functions: `read_text`, `write_text`, `delete_file`, and `list_dir`.
- How to persist task lists to disk as files.

### 2. Why is this topic important?
Variables in memory are volatile; they vanish the moment your program exits or the operating system restarts. To preserve data permanently—such as export files, application settings, or audit logs—it must be written to non-volatile storage. At the same time, arbitrary file system access represents a significant security liability. Zelyra protects your system by enforcing fine-grained capabilities: functions cannot touch the disk unless they explicitly declare the appropriate permissions upfront.

### 3. Understandable explanation without unnecessary jargon
Think of the file system as a secure document archive:
- When you want to file a document, you write text into it (`write_text`).
- When you want to inspect a file, you read its contents (`read_text`).
- You can only enter the physical archive room if you hold the keycard: `uses FileSystem`.

```zelyra
fn save_note(path: String, content: String)
    uses FileSystem
{
    write_text(path, content)
}

fn main() uses FileSystem {
    save_note("note.txt", "Shopping list: Milk, Bread")
    print("Note saved.")
}
```

### 4. Small, progressive examples

**Example 1: Writing and reading text files**
```zelyra
fn file_workflow() uses FileSystem {
    path = "tasks_export.txt"
    write_text(path, "Task 1: Learn Zelyra")
    text = read_text(path)
    print("Read content: " + text)
}

fn main() uses FileSystem {
    file_workflow()
}
```

**Example 2: Cleaning up files with delete_file**
```zelyra
fn cleanup_file(path: String) uses FileSystem {
    delete_file(path)
    print("File deleted.")
}

fn main() uses FileSystem {
    write_text("temp.txt", "Short-lived")
    cleanup_file("temp.txt")
}
```

**Example 3: Listing directory contents**
```zelyra
fn show_files(dir_path: String) uses FileSystem {
    files = list_dir(dir_path)
    for file_name in files {
        print("Found file: " + file_name)
    }
}

fn main() uses FileSystem {
    show_files(".")
}
```

### 5. Typical errors and their causes
- **Error:** Calling `read_text` or `write_text` without declaring `uses FileSystem`.
  *Cause:* Zelyra's capability system (`E-CAP-001`) prevents any unauthorized access to secondary storage.
- **Error:** Forgetting that `main()` must also declare `uses FileSystem` when calling functions that access the file system.
  *Cause:* Capabilities propagate up the call stack; a caller cannot invoke an effectful function without having that permission itself.

### 6. Key takeaways
1. Any function interacting with the file system must declare `uses FileSystem`.
2. `write_text` creates a new file or completely overwrites an existing one.
3. `read_text` returns the entire contents of a file as a `String`.

### 7. Exercises (Level 1 Easy, Level 2 Medium, Level 3 Challenging)
- **Level 1 (Easy):** Write a program that writes a greeting message into `hello.txt`.
- **Level 2 (Medium):** Write a function that reads a file and displays its content on the console.
- **Level 3 (Challenging):** Implement a simple logging function that appends status messages line by line and persists them to disk.

### 8. Practical project task: Task Management
Export the pending tasks of our application into a text file:
```zelyra
fn export_tasks(path: String) uses FileSystem {
    content = "[ ] Complete documentation\n[OK] Install Zelyra compiler"
    write_text(path, content)
    print("Tasks successfully exported to " + path + ".")
}

fn main() uses FileSystem {
    export_tasks("tasks_today.txt")
}
```

### 9. Summary
- Zelyra provides clean, safe, and efficient primitives for file input and output.
- The capability-based security model prevents unauthorized file manipulation and data leaks.

### 10. Self-check review questions
1. Which capability must a function request before it can invoke `write_text`?
2. Why does Zelyra require `uses FileSystem` on `main()` even if `main` only delegates to another function?
3. Which standard library function returns a list of all filenames inside a directory?

---

## Chapter 21: Date, Time, Randomness, and Structured Data

### 1. What will I learn in this chapter?
In this chapter, you will learn:
- How to retrieve current timestamps with `now()` (`uses Clock`).
- How to generate pseudo-random values using `random_int(min, max)` (`uses Random`).
- How to serialize data structures into universal JSON with `json_encode`.
- How to parse JSON strings back into strongly typed structures using `json_decode<T>`.

### 2. Why is this topic important?
Virtually every real-world application requires handling temporal events: When was a task created? When does a deadline expire? Similarly, structured formats such as JSON serve as the lingua franca across the web—from REST APIs to configuration files. Zelyra incorporates time, randomness, and JSON natively into the language, providing strict type-safety without external dependencies.

### 3. Understandable explanation without unnecessary jargon
- **Timestamps:** Calling `now()` returns the precise current system time as a `Timestamp`. Because reading the system clock is an external, non-deterministic effect, your function must declare `uses Clock`.
- **Random Numbers:** Calling `random_int(1, 10)` generates a pseudo-random integer between 1 and 10, requiring `uses Random`.
- **JSON:** JSON is a lightweight text format easily parsed by machines and humans. With `json_encode`, you turn Zelyra arrays and records into a serialized string ready for network transfer or file storage.

```zelyra
fn show_time() uses Clock {
    current_time = now()
    print("Current system time recorded")
}

fn main() uses Clock {
    show_time()
}
```

### 4. Small, progressive examples

**Example 1: Deadlines and time tracking with Clock**
```zelyra
fn log_creation(task_name: String) uses Clock {
    created_at = now()
    print("Task created: " + task_name)
}

fn main() uses Clock {
    log_creation("Patch server")
}
```

**Example 2: Generating random ticket numbers**
```zelyra
fn generate_ticket_number() -> Int uses Random {
    return random_int(1000, 9999)
}

fn main() uses Random {
    ticket = generate_ticket_number()
    print("Your ticket code:")
    print(ticket)
}
```

**Example 3: Exporting data as JSON**
```zelyra
fn export_ids_as_json(ids: Int[]) -> String {
    return json_encode(ids)
}

fn main() {
    ids: Int[] = [101, 102, 103]
    json_text = export_ids_as_json(ids)
    print("JSON output: " + json_text)
}
```

### 5. Typical errors and their causes
- **Error:** Calling `now()` without declaring `uses Clock` in the function signature.
  *Cause:* Clock queries are inherently non-deterministic; Zelyra requires explicit capability declarations.
- **Error:** Passing malformed JSON text into `json_decode`.
  *Cause:* Zelyra strictly validates JSON input; schema mismatches or syntax errors result in a `Result` failure.

### 6. Key takeaways
1. `now()` yields the current timestamp and requires `uses Clock`.
2. `random_int` produces random integers and demands `uses Random`.
3. `json_encode` converts Zelyra data structures into standard JSON strings.

### 7. Exercises (Level 1 Easy, Level 2 Medium, Level 3 Challenging)
- **Level 1 (Easy):** Generate a random integer between 1 and 6 (simulating a dice roll) and print it.
- **Level 2 (Medium):** Write a function that serializes an array of integer status codes into JSON.
- **Level 3 (Challenging):** Combine `uses Clock` and `uses FileSystem` to write a timestamped log entry into `log.txt`.

### 8. Practical project task: Task Management
Create a JSON snapshot of current task IDs and persist it to disk:
```zelyra
fn save_snapshot(file_name: String, ids: Int[])
    uses Clock, FileSystem
{
    json_data = json_encode(ids)
    write_text(file_name, json_data)
    print("Snapshot saved successfully.")
}

fn main() uses Clock, FileSystem {
    current_ids: Int[] = [1, 2, 5, 8]
    save_snapshot("tasks_snapshot.json", current_ids)
}
```

### 9. Summary
- Zelyra includes native, type-safe support for timestamps, randomness, and JSON data exchange.
- Effect capabilities (`Clock`, `Random`) guarantee auditability and safety across your codebase.

### 10. Self-check review questions
1. Which capability must a function declare to call `now()`?
2. Why does Zelyra enforce the `uses Random` permission for generating random numbers?
3. What standard format does `json_encode` produce?

---

## Chapter 22: Concurrency and Background Tasks

### 1. What will I learn in this chapter?
In this chapter, you will learn:
- What concurrency is and when tasks should be executed simultaneously.
- How Zelyra's structured `parallel` block works: `parallel { a = await ...; b = await ... }`.
- Why Zelyra avoids uncontrolled raw threads and callback hell.
- How deterministic concurrency keeps your application performant and free of race conditions.

### 2. Why is this topic important?
Modern CPUs and servers feature multiple cores. When an application needs to generate independent reports or query several remote APIs, executing them sequentially one after another wastes time. Running tasks in parallel allows programs to complete in a fraction of the time. However, in many languages concurrency introduces insidious bugs such as race conditions and deadlocks. Zelyra eliminates these dangers through structured, deterministic concurrency.

### 3. Understandable explanation without unnecessary jargon
Imagine a commercial restaurant kitchen:
- If the chef fries the steak first, then fries the potatoes, and finally washes the salad, the steak gets cold.
- A skilled chef begins all three tasks simultaneously and waits until all three dishes are ready to be served.

In Zelyra, you achieve this using the `parallel` block paired with `await`:
```zelyra
fn calculate_part_1() -> Int {
    return 40
}

fn calculate_part_2() -> Int {
    return 60
}

fn main() {
    parallel {
        result_1 = await calculate_part_1()
        result_2 = await calculate_part_2()
    }
    print("Both subtasks completed in parallel.")
}
```

### 4. Small, progressive examples

**Example 1: Parallel sub-computations**
```zelyra
fn compute_statistics() -> String {
    return "Statistics computed"
}

fn load_archive() -> String {
    return "Archive loaded"
}

fn main() {
    parallel {
        stats = await compute_statistics()
        archive = await load_archive()
    }
    print("Parallel loading successful.")
}
```

**Example 2: Independent data retrieval**
```zelyra
fn fetch_sum_a() -> Int {
    return 100
}

fn fetch_sum_b() -> Int {
    return 250
}

fn main() {
    parallel {
        val_a = await fetch_sum_a()
        val_b = await fetch_sum_b()
    }
    print("Sums calculated.")
}
```

### 5. Typical errors and their causes
- **Error:** Using `await` outside of a `parallel` block.
  *Cause:* Zelyra strictly restricts `await` to the body of a `parallel { ... }` block.
- **Error:** Placing general statements or complex branching inside a `parallel` block.
  *Cause:* A `parallel` block is dedicated to concurrent evaluation; its statements must conform to the `identifier = await call()` assignment pattern.

### 6. Key takeaways
1. `parallel { ... }` executes independent operations concurrently.
2. Every line in a `parallel` block adheres to `variable = await expression()`.
3. Structured concurrency guarantees completion and prevents dangling background processes.

### 7. Exercises (Level 1 Easy, Level 2 Medium, Level 3 Challenging)
- **Level 1 (Easy):** Define two simple arithmetic functions and execute them inside a `parallel` block.
- **Level 2 (Medium):** Create two functions that each produce a string message, and evaluate both concurrently.
- **Level 3 (Challenging):** Simulate checking two project validation conditions in parallel before initializing a workflow.

### 8. Practical project task: Task Management
Accelerate application startup by fetching user profiles and task lists simultaneously:
```zelyra
fn load_user_profile() -> String {
    return "Profile: Developer"
}

fn load_task_list() -> String {
    return "5 tasks loaded"
}

fn main() {
    print("Starting parallel retrieval...")
    parallel {
        profile = await load_user_profile()
        tasks = await load_task_list()
    }
    print("Dashboard ready.")
}
```

### 9. Summary
- Concurrency in Zelyra is structured, deterministic, and protected against race conditions.
- The `parallel` construct neatly unifies asynchronous computations in a concise block.

### 10. Self-check review questions
1. Where in a Zelyra program is the `await` keyword permitted?
2. What specific syntax must statements inside a `parallel` block follow?
3. What is the primary benefit of parallel execution over sequential evaluation?
