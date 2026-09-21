# The Zelyra Handbook

**From foundations to database-backed web applications. State intent. Prove correctness.**

English · [Deutsche Ausgabe](/handbuch)

Welcome to the complete Zelyra Handbook. It includes both the introductory textbook **"Learning Zelyra – Understandable Programming from the Foundations to Your Own Application"** (Parts I to X, Chapters 1 to 42), the **Technical Reference Manual** (Chapters 1 to 23), and comprehensive **Appendices** (A to J).

> **Project status:** Compiler 0.2.0 implements a tested, experimental subset of language line 0.1. Zelyra is not approved for production use.

## Status marks

- ✅ **Implemented and verified:** present in the current repository and successfully run in this pass.
- 🧪 **Experimental:** present, but early or constrained.
- 🗺️ **Planned:** part of the language vision, not yet reliably available.
- ❌ **Currently unavailable:** not present in the current CLI.

## Table of Contents

### Learning Zelyra – The Textbook

- **[PART I – UNDERSTANDING ZELYRA AND PROGRAMMING](#part-i-understanding-zelyra-and-programming)**
  - [Chapter 1: Welcome to Zelyra](#chapter-1-welcome-to-zelyra)
  - [Chapter 2: How a Program Works](#chapter-2-how-a-program-works)
  - [Chapter 3: Installing and Setting Up Zelyra](#chapter-3-installing-and-setting-up-zelyra)
  - [Chapter 4: The First Zelyra Project](#chapter-4-the-first-zelyra-project)
- **[PART II – LANGUAGE FUNDAMENTALS](#part-ii-language-fundamentals)**
  - [Chapter 5: Values and Data Types](#chapter-5-values-and-data-types)
  - [Chapter 6: Variables and Immutability](#chapter-6-variables-and-immutability)
  - [Chapter 7: Operators and Expressions](#chapter-7-operators-and-expressions)
  - [Chapter 8: Input and Output](#chapter-8-input-and-output)
  - [Chapter 9: Decisions with Conditions](#chapter-9-decisions-with-conditions)
  - [Chapter 10: Repetition and Loops](#chapter-10-repetition-and-loops)
- **[PART III – STRUCTURING PROGRAMS](#part-iii-structuring-programs)**
  - [Chapter 11: Functions and Procedures](#chapter-11-functions-and-procedures)
  - [Chapter 12: Contracts and Preconditions (Design by Contract)](#chapter-12-contracts-and-preconditions-design-by-contract)
  - [Chapter 13: Collections, Lists, and Dictionaries (Arrays & Maps)](#chapter-13-collections-lists-and-dictionaries-arrays-and-maps)
  - [Chapter 14: Creating Custom Data Types (Records & Tables)](#chapter-14-creating-custom-data-types-records-tables)
  - [Chapter 15: Modules and Code Organization](#chapter-15-modules-and-code-organization)
- **[PART IV – SAFETY AND ERROR HANDLING](#part-iv-safety-and-error-handling)**
  - [Chapter 16: Error Types and Their Causes](#chapter-16-error-types-and-their-causes)
  - [Chapter 17: Errors as Values – The Result Pattern](#chapter-17-errors-as-values-the-result-pattern)
  - [Chapter 18: Nothingness Does Not Exist – Working Safely with Option](#chapter-18-nothingness-does-not-exist-working-safely-with-option)
  - [Chapter 19: Testing and Quality Assurance](#chapter-19-testing-and-quality-assurance)
- **[PART V – PRACTICAL DATA PROCESSING](#part-v-practical-data-processing)**
  - [Chapter 20: Working with Files](#chapter-20-working-with-files)
  - [Chapter 21: Date, Time, Randomness, and Structured Data](#chapter-21-date-time-randomness-and-structured-data)
  - [Chapter 22: Concurrency and Background Tasks](#chapter-22-concurrency-and-background-tasks)
- **[PART VI – DATABASES WITH ZELYRA](#part-vi-databases-with-zelyra)**
  - [Chapter 23: Why Zelyra Understands Databases Directly](#chapter-23-why-zelyra-understands-databases-directly)
  - [Chapter 24: Defining Tables and Data Modeling](#chapter-24-defining-tables-and-data-modeling)
  - [Chapter 25: Querying and Modifying Data](#chapter-25-querying-and-modifying-data)
- **[PART VII – WEB APPLICATIONS AND FORMS](#part-vii-web-applications-and-forms)**
  - [Chapter 26: Rendering Web Pages](#chapter-26-rendering-web-pages)
  - [Chapter 27: Forms and User Inputs](#chapter-27-forms-and-user-inputs)
  - [Chapter 28: The Complete CRUD Pattern](#chapter-28-the-complete-crud-pattern)
  - [Chapter 29: Users, Passwords, and Sessions](#chapter-29-users-passwords-and-sessions)
  - [Chapter 30: APIs and Data Exchange](#chapter-30-apis-and-data-exchange)
- **[PART VIII – THE DISTINCTIVE FEATURES OF ZELYRA](#part-viii-the-distinctive-features-of-zelyra)**
  - [Chapter 31: Readability as the Highest Priority](#chapter-31-readability-as-the-highest-priority)
  - [Chapter 32: AI-Nativity – Why Zelyra Is Built for AI Assistants](#chapter-32-ai-nativity-why-zelyra-is-built-for-ai-assistants)
  - [Chapter 33: Safety through Capabilities](#chapter-33-safety-through-capabilities)
  - [Chapter 34: Zelyra in Comparison](#chapter-34-zelyra-in-comparison)
- **[PART IX – FROM DESIGN TO FINISHED APPLICATION](#part-ix-from-design-to-finished-application)**
  - [Chapter 35: Planning Software – From Idea to Design](#chapter-35-planning-software-from-idea-to-design)
  - [Chapter 36: Architecture and Clean Code Structure](#chapter-36-architecture-and-clean-code-structure)
  - [Chapter 37: Configuration and Environment Variables](#chapter-37-configuration-and-environment-variables)
  - [Chapter 38: Debugging and Optimization](#chapter-38-debugging-and-optimization)
  - [Chapter 39: Deployment and Operations](#chapter-39-deployment-and-operations)
- **[PART X – CAPSTONE PROJECT AND LOOKING AHEAD](#part-x-capstone-project-and-looking-ahead)**
  - [Chapter 40: The Grand Capstone Project: Complete Task Management](#chapter-40-the-grand-capstone-project-complete-task-management)
  - [Chapter 41: The Zelyra Roadmap (From 0.2.0 to 1.0)](#chapter-41-the-zelyra-roadmap-from-020-to-10)
  - [Chapter 42: Your Journey as a Zelyra Developer](#chapter-42-your-journey-as-a-zelyra-developer)

### Technical Reference Manual

- [1. What makes Zelyra different](#1-what-makes-zelyra-different)
- [2. Installation](#2-installation)
- [3. Your first program](#3-your-first-program)
- [4. Create a project and use the CLI](#4-create-a-project-and-use-the-cli)
- [5. Variables, types, and functions](#5-variables-types-and-functions)
- [6. Option, Result, and pattern matching](#6-option-result-and-pattern-matching)
- [7. MariaDB and tables](#7-mariadb-and-tables)
- [8. Inspecting and applying schemas](#8-inspecting-and-applying-schemas)
- [9. Native SQL](#9-native-sql)
- [10. Web pages](#10-web-pages)
- [11. Forms](#11-forms)
- [12. CRUD](#12-crud)
- [13. Authentication and permissions](#13-authentication-and-permissions)
- [14. Capabilities](#14-capabilities)
- [15. Contracts and verification](#15-contracts-and-verification)
- [16. Configuration and secrets](#16-configuration-and-secrets)
- [17. Diagnostics and troubleshooting](#17-diagnostics-and-troubleshooting)
- [18. Testing and contributing](#18-testing-and-contributing)
- [19. What comes next](#19-what-comes-next)
- [20. Zelyra compared with Rust](#20-zelyra-compared-with-rust)
- [21. Positioning and current development status](#21-positioning-and-current-development-status)
- [22. Roadmap from the current repository](#22-roadmap-from-the-current-repository)
- [23. AI-native development](#23-ai-native-development)
- [24. Authoritative Sources and Compiler Verification (Source Authority)](#24-authoritative-sources-and-compiler-verification-source-authority)

### Appendices

- [Appendix A: Quickstart / Cheat Sheet (Syntax Cheat Sheet)](#appendix-a-quickstart-cheat-sheet-syntax-cheat-sheet)
- [Appendix B: Complete Zelyra Diagnostic & Error Code Reference](#appendix-b-complete-zelyra-diagnostic-error-code-reference)
- [Appendix C: Zelyra CLI Command Reference](#appendix-c-zelyra-cli-command-reference)
- [Appendix D: Standard Library Overview](#appendix-d-standard-library-overview)
- [Appendix E: SQL Cheat Sheet for Zelyra Developers](#appendix-e-sql-cheat-sheet-for-zelyra-developers)
- [Appendix F: HTML and Web Reference in Zelyra](#appendix-f-html-and-web-reference-in-zelyra)
- [Appendix G: Glossary of Technical Terms](#appendix-g-glossary-of-technical-terms)
- [Appendix H: Solutions to Chapter Exercises](#appendix-h-solutions-to-chapter-exercises)
- [Appendix I: Frequently Asked Questions (FAQ)](#appendix-i-frequently-asked-questions-faq)
- [Appendix J: Next Resources and Community](#appendix-j-next-resources-and-community)

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

**Zelyra** is a modern, statically typed language. "Statically typed" means that before a program ever runs, the Zelyra compiler rigorously inspects whether all components fit together correctly. If a function expects text but you accidentally pass it a number, Zelyra alerts you immediately at compile time—before your code ever reaches a server or end user.

At the same time, Zelyra is **database- and web-centric**:
- A table schema (`table`) is not an isolated SQL migration file, but a first-class language element.
- Variables are **immutable by default**. As a result, values cannot change unexpectedly behind the scenes.
- No null pointer surprises: missing or optional values must be declared explicitly as `Option`.

### 4. Small, progressive examples

Let us examine a first minimal Zelyra program:

```zelyra
// Our first Zelyra program: A greeting
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

### 5. Typical errors and their causes
- **Error:** Placing a semicolon at the end of a line (as in Java, C++, or PHP).
  *Cause:* In Zelyra, line breaks cleanly delimit statements. Redundant semicolons clutter the code.
- **Error:** Assuming Zelyra is merely a library, framework, or lightweight scripting layer.
  *Cause:* Zelyra is an independent compiled language with its own type system, static analysis, and verification pipeline.

### 6. Key takeaways
1. Zelyra unifies data models, business logic, and user interfaces within a single cohesive language.
2. What you mean is explicitly written in the code: no hidden magic, no implicit null values.
3. The compiler acts as your verification partner: it catches bugs early, before they can cause damage in production.

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
    version = "0.1.50"
    print(system_name + " (Version " + version + ") started.")
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
- How to install Zelyra from source (`./install.sh` / `install.ps1`) or precompiled release archives (`--release v0.2.0`).
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
./install.sh --release v0.2.0
```
On Windows (PowerShell):
```powershell
git clone https://github.com/sf1976/zelyra.git
Set-Location zelyra
.\install.ps1 -Release v0.2.0
```

**Step 2: Verify version and help**
```bash
zelyra --version
zelyra --help
```
`zelyra --version` outputs the full compiler and package version (for example, `zelyra 0.2.0`). The language compatibility line remains 0.1.

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

### 5. Typical errors and their causes
- **Error:** `zelyra: command not found`
  *Cause:* The directory `~/.local/bin` is not yet in your `$PATH` variable. Run `export PATH="$HOME/.local/bin:$PATH"` or restart your terminal.
- **Error:** `permission denied while trying to connect to the Docker daemon socket`
  *Cause:* On Linux systems, your user does not yet belong to the `docker` group. Run `sudo usermod -aG docker $USER` and log back in. Zelyra catches this and provides a clear hint.
- **Error:** Default port 3000 or 3306 is occupied.
  *Cause:* Another local service is using the port. `zelyra setup` and `zelyra new` automatically detect collisions and select the next free host port.

### 6. Key takeaways
1. The Zelyra CLI bundles compiler, runner, form checker, migrator, web server, and setup assistant in a single tool.
2. `docker compose version` and `zelyra doctor` verify the health of your environment at any time.
3. Official release binaries can be installed directly with `--release v0.2.0`.
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
    // Explicit type annotation: Name followed by colon and type
    count: Int = 10

    // Automatic type inference: Zelyra immediately detects that this is a String
    task_name = "Important Meeting"

    print(task_name)
}
```

The core data types in Zelyra:
- **`Int`**: 64-bit signed integers, e.g., `-5`, `0`, `42`.
- **`UInt`**: Unsigned integers (`>= 0`), ideal for record IDs and positive counters.
- **`Float`**: Floating-point numbers for scientific calculations, e.g., `3.1415`.
- **`Decimal`**: Fixed-point numbers with guaranteed precision—indispensable for financial amounts to eliminate floating-point rounding artifacts!
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
    priority: Int = 1 // 1 = high, 2 = medium, 3 = low

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
    // project = "New Project" // ERROR: Compiler blocks assignment to immutable variable!
    print(project)
}
```

**Example 2: When `mutable` is appropriate (counters and accumulators)**
```zelyra
fn main() {
    mutable open_tasks = 5
    print(open_tasks)

    // One task has been completed:
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
    // print(local_var) // ERROR: `local_var` no longer exists outside the block!
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

    // First task completed:
    open_count = open_count - 1
    print("Intermediate count: Still open:")
    print(open_count)

    // Second task completed:
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
    // Arithmetic
    base_time = 60
    buffer_time = 15
    total_time = base_time + buffer_time
    print(total_time)

    // Comparison
    is_long = total_time > 60
    print(is_long)

    // Logical combination
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

    // A task is overdue if days < 0 and it is not yet completed
    is_overdue = days_remaining < 0 && !is_done

    // High urgency: Overdue and not blocked by other tasks
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
            // Skip the number 2:
            continue
        }
        if num == 4 {
            // Abort completely at 4:
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
- **Key types:** All scalar types (`String`, `Int`, `Id`, etc.) are permitted.

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
    
    // Add a new item
    mutable current_prices = put(prices, "Cake", 4)
    // Update existing price
    current_prices = put(current_prices, "Coffee", 4)
    
    // Safely query using match
    match get(current_prices, "Coffee") {
        Some(price) => {
            print("Coffee price: " + str(price) + " Euro")
        }
        None => {
            print("Item not found.")
        }
    }
    
    // Output all keys
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
  *Cause:* Map keys must be scalar types (`String`, `Int`, `Id`) to guarantee deterministic comparison and JSON serialization.

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
            print("Priority for " + task_name + ": Level " + str(level))
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
- How records are defined in Zelyra as a `table` with attributes (`id: Id primary auto`).
- Why strong typing fundamentally elevates software quality.

### 2. Why is this topic important?
In fragile software architectures, almost everything is represented as a primitive integer or string (an anti-pattern known as "Primitive Obsession"). If a function expects `verify(user_id: Int, task_id: Int)` and you accidentally swap the arguments, a compiler using raw `Int` types will never notice—and your application might assign sensitive records to the wrong user. With distinct custom types, Zelyra distinguishes between a `UserId` and a `TaskId` right at compile time.

### 3. Understandable explanation without unnecessary jargon
Custom data types allow you to model real-world concepts accurately:
1. **Nominal Type Aliases:**
   ```zelyra
   type TaskId = Id

   fn main() {
       print("Type alias TaskId active")
   }
   ```
   This gives `TaskId` its own distinct type identity.
2. **Tables as Structured Data Types:**
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

*Note on Language Version 0.1:* The `import` keyword for fine-grained package modularization is currently slated on the Zelyra roadmap for Phase 11/12. In the current 0.1 release, source files in a project are compiled cohesively within the project context.
`// [Placeholder: Module imports - not yet specified in Zelyra 0.1; see Roadmap Phase 11/12]`

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
  *Cause:* Zelyra 0.1 compiles files within a unified project context; external import syntax is scheduled for Roadmap Phase 11.
- **Error:** Deleting `zelyra.toml` or executing CLI commands from outside the project root directory.
  *Cause:* Commands like `zelyra run` look for `zelyra.toml` in the current working directory to configure capabilities and compilation paths.

### 6. Key takeaways
1. `zelyra.toml` governs project metadata, dependencies, and capability security policies.
2. Maintain a clean separation of concerns: data models (`table`), business logic (`fn`), and UI views.
3. A clean project layout prevents accidental coupling and speeds up teamwork.

### 7. Exercises
- **Level 1 (Easy):** Generate a new project skeleton using `zelyra new task_app` and explore the generated files.
- **Level 2 (Medium):** Update `zelyra.toml` with a project description and increment the version number to `0.2.0`.
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
// Syntactically correct:
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
- An outlook on Zelyra's future integrated test framework.

### 2. Why is this topic important?
Manual testing—clicking around in a browser or manually calling functions—is tedious, inconsistent, and error-prone. As software grows in complexity, even minor modifications in one subsystem inevitably introduce regressions in seemingly unrelated modules. Automated tests act as an automated safety net, proving that existing business logic continues to function according to specification days, months, and years into the future.

### 3. Understandable explanation without unnecessary jargon
Quality assurance in Zelyra rests on two powerful pillars:
1. **Formal Contract Verification with `zelyra verify`:**
   The compiler mathematically checks whether preconditions and postconditions (`requires`, `ensures`) hold under all possible valid inputs.
2. **Automated Assertion Testing:**
   You write concise test routines that supply sample inputs to your business functions and verify that the actual output matches the expected outcome.

*Roadmap Note:* The built-in CLI test framework (`zelyra test`) is scheduled on the Zelyra roadmap for Phase 11/12. In Zelyra 0.1, automated quality assurance is conducted using `zelyra check`, `zelyra verify`, and structured test-runner entry points.
`// [Placeholder: Zelyra Test Framework - realized in 0.1 via verify and test runners; see Roadmap Phase 11]`

### 4. Small, progressive examples

**Example 1: A lightweight assertion helper**
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

**Example 2: Testing Option return values**
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

**Example 3: Protecting logic with formal contract verification**
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
- **Error:** Omitting the `auth_sessions` table.
  *Cause:* Zelyra requires an explicit table for tracking session tokens cryptographically.

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

## Chapter 41: The Zelyra Roadmap (From 0.2.0 to 1.0)

### 1. What will I learn in this chapter?
In this chapter, you will learn:
- The developmental lifecycle of Zelyra: what the experimental 0.2.0 release delivers and what comes next.
- Planned features for the next milestones: module imports, package management, and WebAssembly compilation.
- How backward compatibility and stability guarantees are maintained through version 1.0.

### 2. Why is this topic important?
A programming language is a living ecosystem. When you invest time into mastering Zelyra, you want assurance that the language has a clear strategic roadmap, professional stewardship, and that existing code remains compatible in future releases.

### 3. Understandable explanation without unnecessary jargon: Roadmap Overview
Zelyra's roadmap is structured across implementation phases and release
milestones:
- **Phases 1 to 3 (Foundations):** Lexer, parser, AST, static type checker, control flow, functions, and formal contracts (`requires`, `ensures`). *(Completed)*
- **Phases 4 to 6 (Database & Data):** MariaDB and SQLite engines, type-checked `sql<T[]>`, migrations, transactions, FileSystem and Clock capabilities. *(Completed)*
- **Phases 7 to 9 (Web & Security):** `page`, `html`, `form` with CSRF/XSS protection, `crud` views, `auth` with Argon2, `api` with automated OpenAPI schema generation, Typed Holes, and structured JSON diagnostics. *(Completed)*
- **0.2.0 (current experimental release):** Typed maps, declarative search,
  filtering and pagination, reusable views and slots, generated CRUD,
  authentication and permissions, audit support, setup/doctor tooling,
  machine-readable compiler interfaces, and tested Linux/Windows x86_64
  distribution paths.
- **0.3.0 (proposed):** Better beginner onboarding, release evidence, schema
  safety, and a reproducible real-application acceptance path. See the [release
  plan](../../release-plans/0.3.0.en.md); it is not a release promise.
- **Later milestones:** Fine-grained modules/imports, a package manager,
  WebAssembly compilation, and any LTS commitment remain future work.

### 4. Small, progressive examples: Zelyra's Guarantees to Developers
- **No breaking changes without deprecation cycles:** Syntax changes are introduced with generous transition periods and explicit compiler hints.
- **Formal language specification:** Every language construct is grounded in an unambiguous formal grammar.

### 5. Typical errors and their causes: Common Misconceptions
- **Misconception:** "Zelyra 0.2.0 is production-ready because its core paths work."
  *Correction:* Zelyra 0.2.0 is an experimental, tested scope. The repository
  documents its supported paths and residual risks; production approval is not
  claimed.
- **Misconception:** Blindly assuming syntax conventions from other languages exist today.
  *Correction:* Zelyra is intentionally independent. Unimplemented features (such as dynamic `import` or custom enum types) are explicitly slated for Phases 11 and 12 on the roadmap.

### 6. Key takeaways
1. Zelyra follows a disciplined, transparent roadmap from the current 0.2.0
   release toward later milestones.
2. The core platform has tested experimental paths for database integration,
   web applications, static safety, and AI-native tooling; it is not approved
   for production today.
3. Fine-grained module imports, package distribution, and WebAssembly remain
   future work.

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
1. Which core capabilities are implemented in the tested 0.2.0 scope?
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

# TECHNICAL REFERENCE MANUAL

## 1. What makes Zelyra different

A typical business application describes the same fact repeatedly: in the
database, backend, form, and API. Zelyra aims to turn those copies into one
traceable chain:

~~~text
Table → Types → SQL → Forms → CRUD → Web page → API/OpenAPI
~~~

This field:

~~~zelyra
email: Email?
~~~

already says that the value is an email address, may be absent, affects SQL
nullability, can select a suitable form control, and must be handled safely by
views.

SQL remains SQL. Zelyra does not force a respectable `JOIN` to disguise itself
as a 38-link method chain. SQL has suffered enough.

## 2. Installation

### Prerequisites

For language examples, you need:

- Linux, macOS, or Windows (PowerShell/WSL);
- `curl`;
- a working shell.

MariaDB is required only for database, form actions, auth, and CRUD examples.

### Install from the repository

~~~bash
git clone https://github.com/sf1976/zelyra.git
cd zelyra
./install.sh
~~~

The installer is repeatable and user-local. Control options:

~~~bash
./install.sh --help
./install.sh --dry-run --root "$HOME/.local"
./install.sh --check
./install.sh --uninstall
~~~

Use `--no-rustup` to disable automatic Rust installation; `--no-path` suppresses PATH instructions. With `--root PATH` or `ZELYRA_INSTALL_ROOT`, you can choose a different user-local target. Outdated `cargo` PATH entries are detected and not executed blindly.

Published releases for Linux x86_64 and Windows x86_64 can be installed without Rust or Cargo. The archive is downloaded over HTTPS and verified with SHA-256:

~~~bash
./install.sh --release v0.2.0
~~~

On Windows, `install.ps1` is available for PowerShell and `install.cmd` for the Command Prompt:

~~~powershell
git clone https://github.com/sf1976/zelyra.git
Set-Location zelyra
.\install.ps1
zelyra --version
~~~

Release archive installation on Windows:

~~~powershell
.\install.ps1 -Release v0.2.0
~~~

Then:

~~~bash
zelyra --version
zelyra --help
~~~

If your shell cannot find `zelyra`:

~~~bash
export PATH="$HOME/.local/bin:$PATH"
~~~

### Docker and container environment

Zelyra does not install Docker itself, modify operating-system packages, or request root privileges. For container and MariaDB workflows, Docker with Compose support is required:

- **Linux:** Follow the [official Linux installation guide](https://docs.docker.com/engine/install/).
- **Windows:** Use [Docker Desktop for Windows](https://docs.docker.com/desktop/setup/install/windows-install/) with Compose support.
- **macOS:** Use [Docker Desktop for Mac](https://docs.docker.com/desktop/setup/install/mac-install/).

Verify Docker Compose before first run:

~~~bash
docker compose version
~~~

If Docker is installed but access to its socket is denied, Zelyra reports a safe Linux group-membership remedy (`sudo usermod -aG docker $USER`) instead of raw Docker errors. Port collisions are also reported safely without exposing credentials.

### Updating Zelyra

An update consists of two separate parts: updating the compiler in the compiler repository, and verifying your application project. Your `.zyl` files, `zelyra.toml`, and `.env` live in your application project and are never overwritten by `install.sh`.

#### Update from the Git repository

~~~bash
cd /path/to/zelyra
git status --short
git pull --ff-only origin main
cargo check --workspace
./install.sh
zelyra doctor /path/to/your-project/main.zyl --json
~~~

Check the installed version with `zelyra --version`:

~~~bash
zelyra --version
~~~

## 3. Your first program

Create `hello.zyl`:

~~~zelyra
fn main() {
    print("Hello from Zelyra")
}
~~~

Run it:

~~~bash
zelyra run hello.zyl
~~~

A slightly more ambitious program:

~~~zelyra
fn fibonacci(n: Int) -> Int {
    if n <= 1 {
        return n
    }

    return fibonacci(n - 1) + fibonacci(n - 2)
}

fn main() {
    print(fibonacci(10))
}
~~~

The result is `55`. The computer survived. We may continue.

## 4. Create a project and use the CLI

### Compiler repository and application projects

The GitHub repository `sf1976/zelyra` is the compiler repository. It contains lexer, parser, runtime, database, and web modules, as well as the CLI. Your own application is a separate directory. You do not need to work inside the Zelyra compiler source, and you should never store credentials there.

The project root is the directory containing your Zelyra source file and, if present, `zelyra.toml`.

### ✅ Create projects with the CLI

~~~bash
zelyra new addressbook
cd addressbook
zelyra run main.zyl
~~~

Initialize an existing directory:

~~~bash
mkdir addressbook
cd addressbook
zelyra init
~~~

For a local MariaDB and web-server template:

~~~bash
zelyra new machine-management --mariadb
cd machine-management
~~~

This creates `main.zyl`, `.env.example`, `Dockerfile`, `docker-compose.mariadb.yml`, and a protected `.env` with random passwords.

**Automatic Port Selection on Conflict:**
If default ports `3000` (web) or `3306` (MariaDB) are occupied, `zelyra new`, `zelyra init`, and `zelyra setup` automatically select the next free host ports and record them in `.env`. The optional `--web-port <p>`, `--host-port <p>`, and `--db-host-port <p>` flags enforce exact ports.

### ✅ Zelyra Setup Assistant (Console and Web)

`zelyra setup` provides unified setup actions across CLI and browser:

#### Console setup

From a MariaDB project directory:

~~~bash
zelyra setup
zelyra setup --database
zelyra setup --schema
zelyra setup --all
zelyra setup --host-port 18080 --db-host-port 3308
~~~

- `zelyra setup`: Creates a protected `.env` if missing. Existing `.env` files are never overwritten.
- `zelyra setup --database`: Starts Compose services (detects `docker compose` or legacy `docker-compose`).
- `zelyra setup --schema`: Starts services and safely applies the schema from `main.zyl`.
- `zelyra setup --all`: Executes configuration, container startup, and schema migration in one step.

#### Local browser setup (`zelyra setup --web`)

~~~bash
zelyra setup --web
~~~

The server binds strictly to `127.0.0.1:3030` by default and outputs a one-time URL with a secure random token:

~~~text
Zelyra setup web is running on http://127.0.0.1:3030/
open: http://127.0.0.1:3030/?token=<local-token>
~~~

- Provides browser actions to configure `.env`, start containers, and apply schemas.
- Strictly local-only for security; never expose to public interfaces.
- If port `3030` is busy, the assistant automatically chooses the next free port. Stop with `Ctrl+C`.

Verify your project state non-destructively:

~~~bash
zelyra doctor main.zyl --env-file .env
~~~

For a complete business starter project:

~~~bash
zelyra new machine-management --template mariadb-crud
cd machine-management
zelyra setup --all
~~~

## 5. Variables, types, and functions

Values are immutable by default:

~~~zelyra
machine_name = "Press 7"
capacity: Int = 120
active = true
~~~

Mutation is explicit:

~~~zelyra
mutable completed = 0
completed = completed + 1
~~~

The compiler is not paranoid. It has simply seen things.

Core types include:

~~~text
Int UInt Float Decimal Bool String Char Bytes
Timestamp Date Time Duration Email Url Uuid Money
~~~

Functions are typed:

~~~zelyra
fn available_capacity(total: Int, reserved: Int) -> Int {
    return total - reserved
}
~~~

Nominal IDs keep domains separate:

~~~zelyra
type MachineId = Id
type OrderId = Id
~~~

An `OrderId` is not a `MachineId`, even when both look similar underneath.
Business mistakes do not become correct by sharing a storage type.

## 6. Option, Result, and pattern matching

Ordinary types are never null. Absence is explicit:

~~~zelyra
email: Email?
~~~

Handle both cases:

~~~zelyra
match email {
    Some(value) => print(value)
    None => print("No email address")
}
~~~

Pattern matching must be exhaustive, preventing the forgotten case that would
otherwise introduce itself during a Friday deployment.

Functions expose failures through their return type. The current language core
uses `Result<T, E>` with `Ok` or `Err`:

~~~zelyra
fn load_number(found: Bool) -> Result<Int, String> {
    if found {
        return Ok(42)
    }
    return Err("not found")
}
~~~

## 7. MariaDB and tables

✅ MariaDB is the default backend and primary runtime reference. The generated
Compose template uses `mariadb:11`; the compiler does not enforce a specific
MariaDB server version. Database commands call the external `mariadb` client.

~~~zelyra
database main {
    engine: mariadb
}

table departments {
    id: Id primary auto
    name: String(100) required unique
}

table machines {
    id: Id primary auto
    number: String(30) required unique
    name: String(100) required
    department: Department required
    active: Bool default true
}
~~~

Zelyra recognizes the relationship and can use it for foreign keys, forms, and
select controls.

| Zelyra | MariaDB |
|---|---|
| `Id primary auto` | generated primary ID |
| `String(100)` | `VARCHAR(100)` |
| `String` | `TEXT` |
| `Email` | email-compatible text column |
| `Bool` | Boolean database value |
| `Timestamp` | timestamp value |

### Prepare MariaDB safely

Create a dedicated database and application user. Do not run the Zelyra web
server as the MariaDB `root` account. The following is run as an administrative
MariaDB user; the password is requested interactively:

~~~bash
mariadb --host=127.0.0.1 --port=3307 --user=root --password
~~~

~~~sql
CREATE DATABASE `address_book`
    CHARACTER SET utf8mb4
    COLLATE utf8mb4_unicode_ci;

CREATE USER 'zelyra'@'127.0.0.1'
    IDENTIFIED BY 'ENTER_A_LOCAL_PASSWORD_HERE';

GRANT SELECT, INSERT, UPDATE, DELETE, CREATE, ALTER, INDEX, REFERENCES
    ON `address_book`.* TO 'zelyra'@'127.0.0.1';

SHOW GRANTS FOR 'zelyra'@'127.0.0.1';
~~~

Use `IF NOT EXISTS` when the database must be repeatably prepared. The grants
are deliberately limited to this database; never use `GRANT ALL ON *.*` in an
application quickstart. InnoDB is the expected engine for transactions and
foreign keys, but the current Zelyra SQL generator does not add
`ENGINE=InnoDB` itself. Inspect and supplement generated SQL before applying it
if your server policy requires that clause.

The listed grants cover normal CRUD work and the initial schema. A destructive
`db apply --allow-destructive` may additionally need `DROP`; grant it only
deliberately and, where possible, temporarily. `db setup` is an administrator
operation because it creates the database itself.

The generated Compose file publishes `127.0.0.1:3306` by default. If that host
port is occupied, change its binding to `127.0.0.1:3307:3306`: the host port is
then `3307`, while MariaDB remains on `3306` inside the container. Another
Compose service connects to host `mariadb` and port `3306`; a host process uses
`127.0.0.1` and the published port.

### Inspect the address schema

For the example, `zelyra db create src/main.zyl` currently produces, in essence:

~~~sql
CREATE TABLE IF NOT EXISTS `addresses` (
    `id` BIGINT PRIMARY KEY NOT NULL AUTO_INCREMENT,
    `first_name` VARCHAR(100) NOT NULL,
    `last_name` VARCHAR(100) NOT NULL,
    `street` VARCHAR(150) NOT NULL,
    `postal_code` VARCHAR(10) NOT NULL,
    `city` VARCHAR(100) NOT NULL,
    `email` VARCHAR(255)
) ENGINE=InnoDB DEFAULT CHARACTER SET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
~~~

For MariaDB, the generated table statements include
`ENGINE=InnoDB DEFAULT CHARACTER SET=utf8mb4 COLLATE=utf8mb4_unicode_ci`.
The output is therefore ready for the documented MariaDB baseline, but it is
still a reviewed schema proposal: inspect `db plan` before applying changes to
an existing database.

### Additional database backends

Besides MariaDB, the schema CLI can currently process SQLite and PostgreSQL.
Select the backend in the `.zyl` file:

~~~zelyra
database main { engine: sqlite database: "address-book.sqlite3" }
database main { engine: postgres database: "address-book" }
~~~

For SQLite, `DATABASE_URL` uses the `sqlite://` or `sqlite:` scheme; MariaDB
accepts `mariadb://` and the compatible `mysql://` name. The `create`,
`inspect`, `plan`, and `apply` schema operations account for tables, columns,
foreign keys, and indexes. Automatic database creation through `db setup` and
`db bootstrap` currently supports MariaDB and SQLite; for PostgreSQL use
`db create`, `db inspect`, `db plan`, and then `db apply`.

~~~bash
export DATABASE_URL='sqlite:///tmp/address-book.sqlite3'
zelyra db bootstrap src/main.zyl
zelyra db inspect src/main.zyl
~~~

The native SQL runtime is still primarily exercised with MariaDB. A supported
schema backend therefore does not automatically mean that every runtime query
behaves identically on every backend.

## 8. Inspecting and applying schemas

Zelyra uses no migration classes. The source file is the desired schema; the
CLI compares it with the current database. The actual workflow is:

1. Check the source: `zelyra check src/main.zyl`.
2. Print SQL only: `zelyra db create src/main.zyl`.
3. Read the current schema: `zelyra db inspect src/main.zyl`.
4. Plan the difference: `zelyra db plan src/main.zyl`.
5. Read the plan and assess its risk.
6. Apply only after review: `zelyra db apply src/main.zyl`.

Supply the connection only through the process environment:

~~~bash
export DATABASE_URL='mariadb://zelyra:ENTER_A_LOCAL_PASSWORD_HERE@127.0.0.1:3307/address_book'
~~~

⚠️ Zelyra does **not** load `.env` automatically. It is a safe local place to
store values, but the process must receive the variables before the CLI runs;
see section 16.

Print the desired SQL:

~~~bash
zelyra db create src/main.zyl > sql/addresses.generated.sql
~~~

If you added `ENGINE=InnoDB` and the charset clauses after review, apply that
exact file manually with the MariaDB client:

~~~bash
mariadb --host=127.0.0.1 --port=3307 --user=zelyra --password \
    address_book < sql/addresses.generated.sql
~~~

`zelyra db apply` does not read an edited SQL file; it regenerates its plan from
the `.zyl` source. Use either the unchanged Zelyra plan or the reviewed manual
client path, rather than blindly doing both.

Inspect, plan, and apply:

~~~bash
zelyra db inspect src/main.zyl
zelyra db plan src/main.zyl
zelyra db apply src/main.zyl
~~~

Destructive changes require explicit permission:

~~~bash
zelyra db apply src/main.zyl --allow-destructive
~~~

That flag does not mean “probably fine.” It means “I read the plan, have a
backup, and my pulse is normal.”

For MariaDB, `db setup` and its `db bootstrap` alias first try to create the
database and then apply the generated schema. `DATABASE_URL` therefore needs
administrative privileges for that operation. An application user with limited
grants should use `db create` and have an administrator apply the SQL instead.
Without `DATABASE_URL`, `db plan` can plan against an empty schema; that is a
preview, not a connection test.

❌ There is no `zelyra db check`, `zelyra schema inspect`, `zelyra schema plan`,
or `zelyra schema apply` command. The actual readiness test is
`zelyra doctor src/main.zyl`; it checks static rules, Cargo, `DATABASE_URL`
when set, and the local web port.

### In 10 minutes to a first Zelyra application

These steps consistently use the directory `address-book`, source file
`src/main.zyl`, database `address_book`, user `zelyra`, and host port `3307`.

1. Create the project and copy the environment template:

   ~~~bash
   zelyra new address-book --mariadb
   cd address-book
   cp .env.example .env
   ~~~

2. Put local placeholder values into `.env`. Compose reads the `MARIADB_*`
   values when the container is first initialized; Zelyra itself reads only
   `DATABASE_URL`. Change the Compose binding to `127.0.0.1:3307:3306` before
   starting it.

3. Start MariaDB:

   ~~~bash
   docker compose -f docker-compose.mariadb.yml up -d mariadb
   docker compose -f docker-compose.mariadb.yml ps
   ~~~

4. As a MariaDB administrator, create the database and user with the SQL from
   section 7. Use an obvious local placeholder, never a real password in docs.

5. Save the valid address-book source as `src/main.zyl` and check it:

   ~~~bash
   zelyra check src/main.zyl
   ~~~

6. Load `DATABASE_URL` into the process and test readiness. `.env` is not
   automatically loaded by Zelyra:

   ~~~bash
   set -a
   . ./.env
   set +a
   zelyra doctor src/main.zyl --json
   ~~~

7. Generate and inspect SQL:

   ~~~bash
   zelyra db create src/main.zyl > sql/addresses.generated.sql
   sed -n '1,160p' sql/addresses.generated.sql
   ~~~

8. Inspect and plan:

   ~~~bash
   zelyra db inspect src/main.zyl
   zelyra db plan src/main.zyl
   ~~~

9. Apply only after review:

   ~~~bash
   zelyra db apply src/main.zyl
   ~~~

10. Start the web server. A CRUD route also needs a web definition accepted by
    the server; `crud` alone is not a static export:

    ~~~bash
    zelyra serve src/main.zyl 127.0.0.1:3000
    ~~~

On Windows, use `Copy-Item` and set the process variable in PowerShell:

~~~powershell
Copy-Item .env.example .env
$env:DATABASE_URL = 'mariadb://zelyra:ENTER_A_LOCAL_PASSWORD_HERE@127.0.0.1:3307/address_book'
zelyra doctor src/main.zyl --json
~~~

The generated Compose template publishes host port `3306` by default. `3307`
in this handbook is a deliberate alternative to avoid a collision; change the
Compose port binding before copying the quickstart.

## 9. Native SQL

✅ SQL is a language element:

~~~zelyra
fn load_active_machines() -> Machine[]
    uses Database
{
    return sql<Machine[]> {
        SELECT id, number, name, department_id, active
        FROM machines
        WHERE active = true
        ORDER BY number
    }
}
~~~

Named parameters are bound safely:

~~~zelyra
fn load_machine(id: MachineId) -> Machine?
    uses Database
{
    return sql<Machine?> {
        SELECT id, number, name, department_id, active
        FROM machines
        WHERE id = :id
    }
}
~~~

Where schema information is available, Zelyra checks tables, columns, aliases,
parameters, nullability, result mappings, and the `Database` capability.

Writes can be grouped in transactions:

~~~zelyra
transaction {
    sql {
        UPDATE machines
        SET active = false
        WHERE id = :id
    }
}
~~~

## 10. Web pages

✅ A complete, safe web layer with typed data bindings, declarative query controls, and reusable views is implemented.

~~~zelyra
page "/machines/{name}" {
    html {
        <html>
            <body>
                <h1>Machine {name}</h1>
                <p>Running smoothly.</p>
            </body>
        </html>
    }
}
~~~

Start the web server:

~~~bash
zelyra serve app.zyl
~~~

Then visit:

~~~text
http://127.0.0.1:3000/machines/Press-7
~~~

Path parameters are HTML-escaped by default. The web core handles GET routes, path parameters, query strings, and HTML responses.

### Typed view data loading and collection loops

View interpolations are checked before the server starts. A page may load a typed record and use checked field access:

~~~zelyra
page "/customers/{name}" {
    load customer = sql<Customer> {
        SELECT id, name FROM customers WHERE name = :name
    }
    html { <h1>{customer.name}</h1> }
}
~~~

SQL is checked against the schema, route parameters are safely bound, and capabilities and permissions are enforced. Collections can be rendered with a typed server-side loop:

~~~zelyra
page "/customers" {
    load customers = sql<Customer[]> { SELECT id, name FROM customers }
    html { <ul>for customer in customers { <li>{customer.name}</li> }</ul> }
}
~~~

### Declarative web query controls: Search, sort, pagination, and filter

Pages can declare typed query inputs for explicit SQL:

~~~zelyra
page "/customers" {
    input { search: String? }

    load customers = sql<Customer[]> {
        SELECT id, name FROM customers
        WHERE (:search IS NULL OR name LIKE CONCAT('%', :search, '%'))
        ORDER BY name
    }

    html { <p>Search: {search}</p> }
}
~~~

For page collections declaring search, sorting, pagination, or filtering, Zelyra automatically generates semantic query controls and preserves URL state:

~~~zelyra
page "/customers" {
    search { name email }
    sort { name }
    paginated 25
    filter { name quantity }
    load customers = sql<Customer[]> { SELECT id, name, quantity FROM customers }
    html {
        <p>Page: {page} of {pages} (Total: {total})</p>
        <p>Sort: {sort} ({order})</p>
    }
}
~~~

- `search { name email }`: Compiler-checked whitelist for `?search=...`, parameterized `LIKE` query.
- `sort { name }`: Accepts declared result fields; `order` accepts `asc` or `desc` (`/customers?sort=name&order=desc`).
- `paginated 25`: Validates `page` as positive integer, applies parameterized `LIMIT`/`OFFSET`, and exposes `page`, `pages`, and `total` as `UInt`.
- `filter { ... }`: Supports typed operators: text fields support `eq`, `contains`, `starts_with`, `ends_with`, and null checks; numeric fields additionally support `gt`, `gte`, `lt`, `lte`. Examples: `/customers?filter_name__contains=Acme` or `/customers?filter_quantity__gte=10`. Invalid operators return controlled HTTP 400.

### Reusable view layouts and components

A named view provides a reusable page layout with slots:

~~~zelyra
view SiteShell {
    html {
        <html><body><main><slot /></main></body></html>
    }
}

page "/customers" {
    view: SiteShell
    html { <h1>Customers</h1> }
}
~~~

Typed components declare properties via `props`:

~~~zelyra
component Badge {
    props { text: String }
    html { <span class="badge">{text}</span> }
}

page "/status" {
    html { <Badge text="Ready" /> }
}
~~~

Components accept child HTML through default and named slots with fallback content:

~~~zelyra
component Panel {
    html {
        <section class="panel">
            <header><slot name="header">Default Header</slot></header>
            <div class="body"><slot /></div>
        </section>
    }
}

page "/dashboard" {
    html {
        <Panel>
            <slot name="header"><h1>My Dashboard</h1></slot>
            <p>Main panel body content.</p>
        </Panel>
    }
}
~~~

*Note:* Component slots nested inside a component invocation no longer require an outer page `view:` layout.

## 11. Forms

✅ Forms can inherit table rules:

~~~zelyra
form MachineCreate -> machines {
    fields {
        number
        name
        department
        active
    }
}
~~~

An explicit definition can add presentation and validation rules:

~~~zelyra
form ContactForm {
    field email: Email {
        label: "Email"
        required
        max: 255
        widget: email
    }
}
~~~

Validate without a server:

~~~bash
zelyra form validate examples/customer_form.zyl CustomerCreate \
    name="Example Ltd" email=info@example.test
~~~

With `zelyra serve`, Zelyra exposes a form at `/forms/FormName`. GET renders a
CSRF token; POST checks the token and validates the submitted values.

Database-backed action:

~~~zelyra
form CustomerCreate -> customers {
    fields { name email }

    action save {
        requires auth
        permits "customers.save"
        sql {
            INSERT INTO customers (name, email)
            VALUES (:name, :email)
        }

        redirect "/customers"
    }
}
~~~

Form actions may declare their own authorization. The permission is checked
when the form is rendered and again before submission.

## 12. CRUD

✅ The shortest case is satisfyingly short:

~~~zelyra
crud Machine -> machines
~~~

Configured resource:

~~~zelyra
crud Machine -> machines {
    title: "Machines"
    list { number name department active }
    search { number name }
    filter { department active }
}
~~~

Zelyra provides list and detail pages, Create/Edit forms, search, filters,
allowlisted sorting, pagination, and CSRF-protected deletion. Configured fields
are checked against the schema.

Example URLs:

~~~text
/machines
/machines?search=Press
/machines?filter_active=true
/machines?sort=number&order=asc
/machines/new
/machines/42/edit
~~~

🗺️ Fully custom typed components and fine-grained view overrides are part of
the continuing view roadmap.

### CRUD views, actions, and soft delete

🧪 The current CRUD layer can be customized inside the safe default paths. A
list can use cards without losing search, filters, sorting, pagination, output
escaping, or permission checks:

~~~zelyra
crud Customer -> customers {
    view {
        list {
            mode: cards
            empty: "No customers found."
        }
        detail {
            mode: cards
            title: "Customer details"
        }
        form {
            mode: cards
            title: "Customer form"
            submit: "Save customer"
        }
        delete {
            title: "Delete customer"
            message: "This action cannot be undone."
            submit: "Delete now"
        }
        loading { message: "Loading customers ..." }
        error {
            title: "Customers unavailable"
            message: "Please try again later."
        }
    }
}
~~~

For the common case, a shared field profile can uniformly control the generated
list, detail view, and create/edit forms:

~~~zelyra
crud Customer -> customers {
    view {
        fields { name email active }
    }
}
~~~

An explicit `list { ... }` remains a local override for list/detail. Primary keys
and auto-generated fields are automatically excluded from forms; the compiler
rejects unknown profile fields. When the technical `id` column is omitted from
visible `fields` (as in `name email active` above), Zelyra automatically links
the first displayed field (`name`) to the record detail page for both table and
card layouts.

Domain actions remain POST-only, parameterized, and protected:

~~~zelyra
crud Customer -> customers {
    action deactivate {
        label: "Deactivate customer"
        confirm: "Really deactivate this customer?"
        permits "customers.edit"
        sql {
            UPDATE customers
            SET active = false
            WHERE id = :id
        }
        success "Customer deactivated."
        redirect "/customers"
    }
}
~~~

The runtime enforces the database capability, CSRF protection,
authentication, and the declared permission. Actions can also declare typed
fields, a server-side confirmation page, and custom success or error pages.
This extends the existing CRUD runtime; it is not a freely programmable
frontend generator.

Reversible deletion is available through soft delete:

~~~zelyra
table customers {
    id: Id primary auto
    name: String(100) required
    deleted_at: Timestamp?
}

crud Customer -> customers {
    soft_delete { column: deleted_at }
}
~~~

Normal lists and details show only rows whose column is `NULL`; archived
records are available through `?archived=true` and can be restored through a
CSRF-protected action. Permanent deletion, retention rules, and bulk archiving
remain planned.

When an auth definition declares an audit table, CRUD create, update, delete,
archive, restore, and custom actions write events in the same MariaDB
transaction. Passwords, tokens, secrets, and hashes are removed from change
details.

## 13. Authentication and permissions

🧪 Zelyra supports Argon2 login, persistent MariaDB sessions, logout, route
guards, and database-backed permission checks. Five failed attempts for the
same normalized e-mail address within 15 minutes trigger a 60-second HTTP 429
lockout. A successful login rotates and invalidates the previous browser
session token.

Every state-changing browser form requires its CSRF token and a same-origin
`Origin` or `Referer` matching the request `Host` and effective scheme.
Malformed, missing, or cross-origin evidence is rejected, so copying a token
from another browser cannot authorize a cross-site form submission. API
requests carrying browser-origin information or a browser session cookie also
receive this check, including `GET` requests because handlers are not yet
statically restricted to read-only behavior. Cross-origin API access is
possible only for an exact origin explicitly listed in the project's CORS
policy; a session cookie additionally requires credentialed CORS. The current
CSRF token is process-scoped rather than individually stored per session, so
these origin checks are a required part of the protection.

The Zelyra server currently speaks plain HTTP. Put it behind a trusted TLS
terminating proxy before exposing it beyond a local development machine. The
proxy must preserve the public `Host`, overwrite `X-Forwarded-Proto` with the
actual external scheme, and prevent direct public access to the application
port. Zelyra uses that header to validate the effective origin and to add the
`Secure` attribute to session cookies for HTTPS requests. Never trust a
client-supplied forwarded header at an exposed proxy boundary.

The server also checks every supplied `Host` header against
`ZELYRA_ALLOWED_HOSTS`. The default permits only `localhost`, `127.0.0.1`, and
`[::1]`, which also blocks DNS rebinding through arbitrary hostnames. Add the
actual hostname explicitly to the comma-separated `.env` setting when using a
custom domain or LAN host. Schemes, ports, and wildcards are not accepted.
Duplicate security-sensitive request headers such as `Host`, `Origin`,
`Referer`, `Cookie`, and `Authorization` are rejected to avoid ambiguous
parsing. The response policy `Referrer-Policy: same-origin` allows
same-origin API GETs to provide that evidence without sending referrer
information to other origins.
Process environment overrides the project `.env`, which overrides the
fallback. The allowlist does not replace TLS or origin/CSRF checks.

Create a value for the required `password_hash` column with the CLI. The
interactive command disables password echo and asks for confirmation:

~~~bash
zelyra auth hash-password
~~~

For deliberate automation, pass one password line through `--stdin`. Do not
put real passwords in command arguments or commit generated hashes to source
control:

~~~bash
printf '%s\n' 'change-this-password' | zelyra auth hash-password --stdin
~~~

~~~zelyra
auth users {
    table: users
    permissions: user_permissions
    roles: user_roles
    role_permissions: role_permissions
}

page "/admin" {
    requires auth
    permits "machines.manage"

    html {
        <h1>Machine management</h1>
    }
}
~~~

The optional `permissions` table contains `user_id` and `permission` for
direct grants. Role groups are enabled with `roles` and `role_permissions`:
the first table contains `user_id` and `role`, and the second contains `role`
and `permission`. Effective permissions are the union of direct grants and
all permissions inherited from the user's roles. In the built-in administration
view, the permission revocation form transmits the valid CSRF token and
reliably removes selected permissions.

The same guards protect typed API handlers:

~~~zelyra
api GET "/api/machines/{id}" {
    handler get_machine
    requires auth
    permits "machines.view"
    input { id: MachineId }
    output Machine
    errors { 404 NotFound }
}
~~~

Protected API failures use JSON with an error `code` and `message`. A handler
can return `Err("NotFound")` to select a matching status from the declared
`errors` block; undeclared errors become 500 responses. Richer domain-error
values remain future work. JSON arrays can be bound to typed fields such as
`Int[]` or `MachineId[]`. Nested JSON input uses declared records:

~~~zelyra
struct Address { city: String }
struct CustomerInput { name: String address: Address }

api POST "/customers" {
    handler echo_customer
    input { customer: CustomerInput }
    output CustomerInput
}
~~~

Unknown record fields and missing required fields are rejected. In the language
core, arrays support literals, indexing, `len`, `append`, `contains`, `first`,
`last`, and concatenation with `+`. Record literals and checked field access
are available for nested values:

~~~zelyra
customer = CustomerInput {
    name: "Anna"
    address: Address { city: "Berlin" }
}

print(customer.address.city)
~~~

Array iteration uses `for ... in`; the loop variable is immutable and scoped to
the loop body. `break` and `continue` are supported.

Generate a browser or Node-compatible TypeScript client from the same API
declarations:

~~~bash
zelyra doc examples/api_records.zyl --typescript > customer-client.ts
~~~

The generated client uses standard `fetch`, includes declared records and
tables as TypeScript types, and handles path/query parameters, JSON bodies,
bearer tokens, response types, and HTTP errors. Declared API error names are
available through `ZelyraApiErrorCode`; `ZelyraApiError.fromResponse` extracts
the status, code, and server message while preserving the raw response body.

API errors may also carry a checked payload. Add the payload type after a
colon and use the same type as the error side of the handler's `Result`:

~~~zelyra
struct ValidationProblem {
    field: String
    message: String
}

api POST "/customers/validate" {
    handler validate_customer
    output Result<String, ValidationProblem>
    errors { 422 ValidationError: ValidationProblem }
}
~~~

The response retains `error.code` and `error.message` and adds the serialized
payload as `error.details`. OpenAPI includes the details schema, while the
generated client exposes it through `ZelyraApiErrorPayloads` and the generic
`ZelyraApiError.details` field. Existing untyped API errors remain compatible.

Browser access is disabled by default. Enable exact origins in the project
configuration when a separate frontend needs to call an API:

~~~toml
[web]
allowed_origins = ["http://localhost:5173"]
allow_credentials = false
~~~

Zelyra answers API `OPTIONS` preflight requests automatically and adds CORS
headers only to declared API routes. Wildcard origins are rejected, and CORS
does not bypass authentication or permissions. Enable credentials only when
browser session cookies are required; the client must also use
`credentials: "include"`.

An origin must begin with `http://` or `https://`. Paths, queries, fragments,
wildcards, and a trailing slash are not allowed. Allowed responses receive
`Access-Control-Allow-Origin` and `Vary: Origin`; enabled credentials add
`Access-Control-Allow-Credentials: true`. A preflight response returns HTTP 204
with the allowed methods and requested headers. Forbidden origins or methods
return structured JSON errors; method errors include the `Allow` header.

A hidden button is not a security boundary. Authorization must be enforced on
the server-side action. Browsers become remarkably creative when trusted.

### API input and secure response defaults

API request bodies for methods other than `GET` and `DELETE` may currently use
`application/json` or `application/x-www-form-urlencoded`. Unsupported media
types return HTTP 415 as a structured JSON error. JSON bodies must be objects;
each declared field is then converted to and checked against its Zelyra type.

The HTTP parser checks `Content-Length`, reads complete bodies across multiple
network reads, and limits request bodies to 1 MiB. Headers are limited to 64
KiB. An oversized body is rejected with HTTP 413 before the handler runs.

All HTML, JSON, redirect, error, and preflight responses receive these secure
default headers:

~~~http
X-Content-Type-Options: nosniff
X-Frame-Options: DENY
Referrer-Policy: same-origin
~~~

These defaults do not replace TLS, authentication, authorization, CSRF
protection, or an appropriate Content-Security-Policy.

### Role management and tamper-evident audit

🧪 Roles and role permissions can be managed with the available CLI commands
when the `auth` definition configures the corresponding tables:

~~~zelyra
auth users {
    table: users
    sessions: auth_sessions
    permissions: user_permissions
    roles: user_roles
    role_permissions: role_permissions
    audit: auth_audit_log
    admin_path: "/admin/access"
    admin_permission: "auth.manage"
    admin_role: admin
}
~~~

~~~bash
DATABASE_URL='mariadb://user:password@127.0.0.1:3306/app' \
  zelyra auth role grant app.zyl 42 manager
DATABASE_URL='mariadb://user:password@127.0.0.1:3306/app' \
  zelyra auth role-permission grant app.zyl manager customers.edit
~~~

`revoke` removes the respective assignment. The commands bind values as SQL
parameters and check the project schema first. Do not use a real password in
the example value.

With `audit: auth_audit_log`, authentication, role, and CRUD events can be
inspected or exported:

~~~bash
DATABASE_URL='mariadb://user:password@127.0.0.1:3306/app' \
  zelyra audit inspect app.zyl --limit 100
DATABASE_URL='mariadb://user:password@127.0.0.1:3306/app' \
  zelyra audit export app.zyl --format json > audit.json
zelyra audit verify app.zyl
~~~

For visible tamper detection, enable chaining explicitly:

~~~zelyra
auth users {
    table: users
    audit: auth_audit_log
    audit_chain: true
}
~~~

The audit table then needs `id`, `previous_hash`, and `entry_hash`, usually
`String(64)`. Zelyra stores lowercase SHA-256 hex values. The hash covers the
canonical pipe-separated sequence
`previous_hash|actor_user_id|event|target_user_id|details|created_at`.
`zelyra audit verify` checks both links and hashes. Pruning is deliberately
disabled for chained logs because deleting an entry would break the chain.
Non-chained old entries can be deliberately removed with
`zelyra audit prune ... --before ... --confirm`.

An optional browser administration page is enabled by `admin_path`,
`admin_permission`, and `admin_role`. It can manage users, passwords, active
status, roles, and role permissions; its forms are CSRF-protected. The last
active administrator remains protected.

## 14. Capabilities

✅ External powers are declared explicitly:

~~~zelyra
fn load_machines() -> Machine[]
    uses Database
{
    return sql<Machine[]> {
        SELECT id, number, name FROM machines
    }
}
~~~

Known capabilities:

~~~text
Database Network FileSystem Environment Process Clock Random Console
~~~

Calling functions must propagate required capabilities. Projects grant them
through `zelyra.toml`:

~~~toml
[capabilities]
database = true
network = false
~~~

Static checking and runtime enforcement at function, native-SQL, forms, CRUD,
and authentication database boundaries are implemented when project grants are
supplied. A complete operating-system sandbox for every capability is not yet
available.

Two safe host APIs are implemented:

~~~zelyra
fn runtime_timestamp() -> Timestamp uses Clock {
    return now()
}

fn configured_mode() -> String? uses Environment {
    return env("ZELYRA_MODE")
}
~~~

now() requires Clock and returns Unix-epoch milliseconds. env(name) requires
Environment and returns String?; a missing variable becomes None. Values are
not logged or exposed automatically. Network, file-system, process, and random
host APIs are implemented, but each requires its own resource grant and
remains deliberately limited in this first version.

The Random capability provides secure integer generation:

~~~zelyra
fn dice_roll() -> Int uses Random {
    return random_int(1, 6)
}
~~~

The range is inclusive on both sides. Invalid ranges fail at runtime, and
random values are not emitted implicitly. Process execution is available only
through the explicitly bounded API below.

The first Network host API is `http_get`:

~~~zelyra
fn load_status(url: String) -> String uses Network {
    return http_get(url)
}
~~~

Project execution uses an exact host allowlist and bounded resources:

~~~toml
[network]
allowed_hosts = ["127.0.0.1:8080", "api.example.com"]
timeout_ms = 5000
max_response_bytes = 1048576
~~~

Without `[network]`, a project allows no hosts. The transport supports
`http://` and `https://` with Rustls certificate verification enabled by
default. It does not follow redirects and returns only successful UTF-8 GET
response bodies within the configured limits. The `http_get` helper remains
the simple GET-only convenience API; use `http_request` when request headers,
request bodies, or response status are needed.

Typed requests use `http_request`:

~~~zelyra
fn create_customer(url: String) -> HttpResponse uses Network {
    return http_request(
        "POST",
        url,
        ["Content-Type: application/json"],
        Some("{\"name\":\"Anna\"}")
    )
}
~~~

The method accepts `GET`, `POST`, `PUT`, `PATCH`, `DELETE`, and `HEAD`.
Headers are `Name: value` strings, the body is `String?`, and the typed result
contains `status: Int`, `headers: String[]`, and `body: String`. GET/HEAD
requests cannot carry a body.

JSON values can be converted to and from checked Zelyra values. Records and
their nested fields are validated against the declared schema:

~~~zelyra
struct Customer { name: String tags: String[] nickname: String? }

fn decode_customer(body: String) -> Customer {
    return json_decode<Customer>(body)
}

fn encode_customer(customer: Customer) -> String {
    return json_encode(customer)
}
~~~

`json_decode<Type>(text)` requires one target type argument and supports
records, nested records, arrays, options, and scalar values. `json_encode`
serializes the same value families. Malformed JSON, type mismatches, unknown
record fields, and missing required fields are explicit runtime errors.

For a complete typed JSON request/response flow, use `http_json` with separate
request and response type arguments:

~~~zelyra
struct CustomerCreate { name: String }
struct Customer { id: Int name: String }

fn create_customer(url: String, payload: CustomerCreate) -> Customer uses Network {
    return http_json<CustomerCreate, Customer>("POST", url, [], Some(payload))
}
~~~

The request record is serialized automatically and the response body is
decoded into the response record. If no `Content-Type` header is supplied,
`application/json` is added. Non-2xx responses are explicit runtime errors;
the helper returns the decoded value rather than response headers.

When response metadata must remain available, use `http_result`:

~~~zelyra
fn submit(url: String, payload: CustomerCreate) -> HttpResult<Customer> uses Network {
    return http_result<CustomerCreate, Customer>("POST", url, [], Some(payload))
}
~~~

`HttpResult<Response>` contains `status: Int`, `headers: String[]`,
`body: String`, `data: Response?`, and `error: HttpError?`. Successful 2xx
responses populate `data`; non-2xx responses populate `error` with status,
headers, body, and message. Transport failures and invalid success JSON remain
runtime errors.

The Process capability exposes a shell-free command API:

~~~zelyra
fn render_report(input: String) -> String uses Process {
    return run_process("/usr/bin/printf", ["%s", input])
}
~~~

Project execution requires an exact command allowlist:

~~~toml
[process]
allowed_commands = ["/usr/bin/printf"]
timeout_ms = 5000
max_output_bytes = 1048576
~~~

Without `[process]`, no command may run. The child environment is cleared,
stdin is closed, timeouts terminate the process, and stdout/stderr are
bounded. Shell execution, environment forwarding, working directories, and
pipelines are future work.

The FileSystem host APIs are:

~~~zelyra
fn read_source(path: String) -> String uses FileSystem {
    return read_text(path)
}

fn write_note(path: String, content: String) uses FileSystem {
    write_text(path, content)
}

fn entries(path: String) -> String[] uses FileSystem {
    return list_dir(path)
}

fn remove_note(path: String) uses FileSystem {
    delete_file(path)
}
~~~

All four APIs require FileSystem. Reads and directory listings use read_roots;
writes and deletes use write_roots. Relative paths are resolved from the
project directory, and existing symlink targets are canonicalized before
access. Without a filesystem section, project reads are limited to the
project directory while writes and deletes are denied:

~~~toml
[filesystem]
read_roots = ["."]
write_roots = ["data"]
~~~

The configured directories must already exist. A new write target must have an
existing parent directory.

### Structured concurrency

An initial, limited concurrency flow is available through `parallel` and
`await`:

~~~zelyra
parallel {
    customer = await load_customer()
    orders = await load_orders()
}
~~~

Each branch binds its result with `await`. Branches receive an immutable
snapshot of surrounding values and are merged in source order before execution
continues. An error in one branch fails the whole block after the started
branches finish. The type checker rejects `await` outside a `parallel` block.
The current runtime uses one worker thread per branch; cancellation and
database connection-pool integration are not implemented yet.

## 15. Contracts and verification

🧪 Preconditions and postconditions:

~~~zelyra
fn reserve(stock: Int, amount: Int) -> Int
    requires {
        amount > 0
        stock >= amount
    }
    ensures {
        result >= 0
        result == stock - amount
    }
{
    return stock - amount
}
~~~

`requires` runs before the body; `ensures` runs afterward, with `result`
representing the returned value.

~~~bash
zelyra verify examples/contracts.zyl
~~~

Statuses:

~~~text
PROVEN
RUNTIME_CHECK
UNPROVEN
FAILED
~~~

Only `PROVEN` means proven. `RUNTIME_CHECK` does not wear a fake moustache and
claim to be mathematics.

The verifier also summarizes bounded function calls. A callee with multiple
return paths, such as an absolute-value function, contributes its path
conditions when a caller returns that call. Callee `requires` clauses are
checked after argument substitution; caller `requires` clauses are assumptions
when proving caller `ensures`. Complex, recursive, or unresolved cases remain
`RUNTIME_CHECK`.

Local state is included in these summaries. Both `next: Int = value + 1` and
the concise `next = value + 1`, followed by `return next`, are analyzed like a
direct return. Simple linear mutable assignments such as `next = next + 1` are
also tracked. Statically bounded loops with a linear counter are unfolded;
`break` exits the current loop and `continue` starts its next iteration as
separate symbolic paths. Nonlinear assignments and unbounded loops without a
proven invariant remain conservative.

### Loop invariants

A `while` or unconditional `loop` may declare one or more explicit invariants:

~~~zelyra
while current > 0
    invariant { current >= 0 }
{
    current = current - 1
}
~~~

The verifier checks the invariant at entry and after supported body paths. A
proven invariant can summarize an otherwise unbounded linear `while` loop; an
unconditional `loop` can use it with a modeled `break` exit. Runtime execution
checks it before and after each iteration. Unsupported or unproven invariants
remain conservative and do not produce `PROVEN` results.

`zelyra verify` reports each declared invariant separately, after a function's
`ensures` results. Invariant indexes are zero-based:

~~~text
PROVEN [V-001]: reduce.ensures[0] (src/reduce.zyl:3:5-3:21)
PROVEN [V-001]: reduce.invariant[0] (src/reduce.zyl:7:21-7:33)
FAILED [V-004]: reduce.invariant[1] (src/reduce.zyl:8:21-8:34)
~~~

Every result includes a stable code and a source range as
`(file.zyl:start-line:start-column-end-line:end-column)`. The codes are
`V-001` (`PROVEN`), `V-002` (`RUNTIME_CHECK`), `V-003` (`UNPROVEN`), and
`V-004` (`FAILED`). For IDEs and CI, use `zelyra verify app.zyl --json`; the
JSON output contains the same result data, a human-readable `message`, an
optional `counterexample` object, and a structured `location` object. A
counterexample is emitted only when a bounded search verifies a small linear
integer witness. The current search covers up to three linear variables in the
range `-32..=32`, including failed loop-invariant checks; otherwise it is
`null`. Text output also shows an explanation and a source-line excerpt with a
caret marker for every result.

`FAILED` means that the invariant is false on a feasible analyzed path or is
not preserved by the loop body. `RUNTIME_CHECK` means that runtime checking
is required because the symbolic verifier cannot complete the proof. Only
`PROVEN` is a mathematical proof.

## 16. Configuration and secrets

Project configuration belongs in `zelyra.toml`; secrets do not:

~~~toml
[project]
name = "machine-management"
version = "0.1.50"
zelyra = "0.1"

[capabilities]
database = true
network = false
~~~

Connections and secrets are supplied via protected environment variables:

~~~bash
export DATABASE_URL='mariadb://user:password@127.0.0.1:3306/zelyra_demo'
~~~

Rules:
- never commit `.env` to version control;
- never place production credentials in code examples;
- do not log secrets;
- separate development, test, and production databases;
- never run destructive tests against production.

### Simple defaults, optional feature switches

The beginner path does not require a feature configuration. Advanced project surfaces can be selected in `zelyra.toml`, while environment-specific, non-secret overrides can be placed in `.env` or the process environment:

~~~toml
[features]
web = true
api = true
crud = true
auth = true
audit = true
~~~

| Switch | `.env` / Process variable | Default | Meaning |
|---|---|---:|---|
| `web` | `ZELYRA_FEATURE_WEB` | `true` | Pages, forms, and web resources |
| `api` | `ZELYRA_FEATURE_API` | `true` | `api` declarations and API surface |
| `crud` | `ZELYRA_FEATURE_CRUD` | `true` | `crud` declarations and generated CRUD surface |
| `auth` | `ZELYRA_FEATURE_AUTH` | `true` | `auth` declarations and authentication surface |
| `audit` | `ZELYRA_FEATURE_AUDIT` | `true` | Audit trail configuration |

Precedence order:
```text
Process environment → .env → zelyra.toml → safe defaults
```

When source code uses a disabled surface, the compiler reports `E-FEATURE-001`. Feature switches cannot disable type checking, SQL validation, capabilities, contracts, CSRF protection, or security rules.

Inspect effective configuration safely without exposing secrets:

~~~bash
zelyra config main.zyl
zelyra config main.zyl --format=json
~~~

### Complete `.env` reference for the current code

| Variable | Default in generated project | Usage | Secret |
|---|---:|---|---|
| `ZELYRA_WEB_PORT` | `3000` | Port of internal web server inside container | no |
| `ZELYRA_HOST_PORT` | `3000` (or auto-selected free port) | Locally published web port | no |
| `ZELYRA_DB_HOST_PORT` | `3306` (or auto-selected free port) | Locally published MariaDB port | no |
| `DATABASE_URL` | project-dependent | MariaDB connection URI (`mariadb://user:pass@host:port/db`) | yes |
| `MARIADB_DATABASE` | `zelyra_app` | Compose: database name | no |
| `MARIADB_USER` | `zelyra` | Compose: application user | no |
| `MARIADB_PASSWORD` | randomly generated | Compose: user password | yes |
| `MARIADB_ROOT_PASSWORD` | randomly generated | Compose: root password | yes |
| `ZELYRA_AUTH_TOKEN` | none | Optional local bearer token for protected endpoints | yes |
| `ZELYRA_AUTH_PERMISSIONS` | empty list | Comma-separated local permission allowlist | no |

### Test and CI variables

Variables prefixed with `ZELYRA_INSTALL_ROOT`, `ZELYRA_BIN`, `*_E2E_*`, and `GENERATED_*` serve internal CI and local integration tests (such as `tests/generated-project-docker-e2e.sh`, `tests/sqlite-e2e.sh`). They are not application configuration and must never contain production secrets.

### Environment access inside the language

Via the built-in `env(name)` function, Zelyra code can read environment variables if `uses Environment` and `[capabilities] environment = true` are declared:

~~~zelyra
fn configured_mode() -> String? uses Environment {
    return env("ZELYRA_MODE")
}
~~~

`DATABASE_URL` and sensitive secrets must never be exposed via `env(...)` to unverified code.

## 17. Diagnostics and troubleshooting

Zelyra aims to explain errors without requiring an archaeological excavation
through a stack trace.

### Test the connection independently

Test MariaDB without Zelyra first. The password is requested interactively and
is not written into shell history:

~~~bash
mariadb \
    --host=127.0.0.1 \
    --port=3307 \
    --user=zelyra \
    --password \
    address_book
~~~

Then run `zelyra doctor src/main.zyl --json` (optionally with `--env-file .env`
and `--port 18080`), the implemented Zelyra readiness test. There is no
`zelyra db check` command. `doctor` checks source code, schema, DB
connectivity, Docker Compose, and host ports. Without `DATABASE_URL`, `doctor`
reports a warning; with an unreachable URL it reports a failure. A running
database container alone does not prove that host, port, user, and database
match.

### Safe diagnostic commands

~~~bash
pwd
ls -la
docker compose ps
docker compose logs mariadb
ss -ltn
mariadb --version
~~~

In PowerShell, use `Get-Location`, `Get-ChildItem`, `docker compose ps`, and
`mariadb --version` as the first checks. Never print `DATABASE_URL` or a
password in diagnostics.

### Common failures

| Error | Likely cause | Fix |
|---|---|---|
| `Permission denied` | insufficient file permissions or no access to the client | inspect `ls -la`, use `chmod 600 .env`, and check the install path |
| `Access denied for user` | wrong password or user is granted for another host | inspect `SHOW GRANTS FOR 'zelyra'@'127.0.0.1';` and rotate the password |
| `Connection refused` | no service listens on the host/port | inspect `docker compose ps`, `ss -ltn`, and the published port |
| `Can't connect to server` | wrong host/port or container is not ready | inspect `docker compose logs mariadb`; use `127.0.0.1:3307` from the host and `mariadb:3306` inside Compose |
| `Unknown database` | URL and MariaDB use different database names | run `SHOW DATABASES;` and correct `DATABASE_URL` |
| wrong port | confused host `3307` with container `3306` | host uses `3307`; a Compose service uses `3306` |
| MariaDB container not started | Compose error, occupied port, or unhealthy volume | inspect `docker compose ps` and `docker compose logs mariadb` |
| user granted only for another host | `'zelyra'@'localhost'` is not always `'zelyra'@'127.0.0.1'` | grant the exact host and inspect grants |
| missing environment variable | `DATABASE_URL` was not exported | load `.env` or set the process variable; Zelyra does not load it itself |
| `.env` not found | wrong working directory or assumed automatic loader | use `pwd`, `ls -la`, and export the variable explicitly |
| invalid numeric port | URI port is not numeric | use digits such as `3307`; otherwise URL parsing fails |
| wrong charset | database uses another charset/collation | inspect the database; `db setup` uses `utf8mb4`/`utf8mb4_unicode_ci` |
| TLS error | unsupported TLS parameters were appended to the URL | use the current URL form; TLS configuration is planned |
| test database rejected for safety | a protection mechanism is expected but not implemented | Zelyra does not automatically prevent production access in tests; check variables manually |
| PostgreSQL SQL sent to MariaDB | wrong backend or incompatible DDL | inspect the `.zyl` backend and read `zelyra db create` output before applying |

If `mariadb` cannot be started at all, Zelyra reports the external process
start error. The CLI does not contain its own MariaDB driver.

~~~bash
zelyra check app.zyl
~~~

Common error classes include unknown names and types, mutation of immutable
values, incomplete pattern matches, unknown tables or columns, missing SQL
parameters, incompatible result mappings, missing capabilities, invalid form
fields, failed contracts, and incomplete typed holes (`_`).

During development, you can place `_` as a placeholder for an unfinished
expression (typed hole). While `zelyra check` intentionally rejects incomplete
code for compilation, it reports helpful contextual diagnostics: the expected
type, visible variables and functions, active capabilities, contract obligations,
and source spans.

Language-only checks work without `DATABASE_URL`. Database operations report a
missing connection in a controlled way. Without the variable, `run` uses the
pure runtime; `serve` can start routes, but database-backed pages return a
controlled error.

## 18. Testing and contributing

Before every commit:

~~~bash
cargo fmt --all
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
~~~

A language feature is complete only when it has documented syntax, AST/HIR
support, static checks, useful diagnostics, positive and negative tests, a
runnable example, and synchronized German and English documentation.

Tests that are green only because they never ran are decoration.

## 19. What comes next

Major planned areas include:

- fully customizable typed view components;
- email templates and SMTP;
- an in-application notification center;
- background jobs and a transactional outbox;
- activity, audit, and technical logs;
- typed connections and secret providers;
- ODBC and external read-only databases;
- richer domain-error values beyond typed API payloads and advanced runtime
  request/response processing;
- cancellation and database-pool integration for structured concurrency;
- broader formal verification;
- optimization models for real planning problems.

Zelyra keeps the common case short without locking the door when the special
case arrives:

> **Generated when possible. Custom where needed. Verified everywhere.**

Now build a table, read the SQL, and check the backup. In that order.

## 20. Zelyra compared with Rust

### The most important statement first

Zelyra is developed with Rust. The Zelyra compiler, language crates, and parts
of the runtime are Rust crates in the compiler repository. That does not mean
that Zelyra changes Rust or that Zelyra applications are Rust programs.

> **Zelyra is not modified Rust. Zelyra is an independent language whose
> compiler and runtime are developed in Rust.**

The Rust compiler is not forked or extended with Zelyra keywords. Zelyra is not
a Rust library and not a preprocessor that rewrites ordinary Rust code. A
`.zyl` file is read by Zelyra's own lexer and parser, converted into its own
AST/HIR structures, type-checked, and then processed by the Zelyra runtime.

Similar spelling in `fn`, braces, `if`, `match`, or static typing is a design
choice, not proof of identity. Grammar, semantics, and the programming model
are what matter. Zelyra is still an experimental prototype; its independence
grows as its own type system, SQL checking, runtime, and language constructs
are implemented.

The statements in this chapter were checked against the current source:
the lexer and `TokenKind` define Zelyra tokens, the parser produces its own
AST structures, and later modules perform resolution, type checking, contract
checking, SQL analysis, web processing, and runtime execution. The CLI and
database modules were reviewed as well. Where a feature exists only as a token,
AST node, or target design, it is not presented as a fully executable language
feature.

### General comparison

| Area | Rust | Zelyra | Essential difference | Zelyra status |
|---|---|---|---|---|
| Language category | general-purpose systems and application language | independent declarative language for business and web applications | different grammar and semantics | 🧪 |
| Main use | systems, services, CLI, embedded, WebAssembly, web | database-backed business and web applications | Zelyra bundles application layers | 🧪 |
| Compiler | `rustc` and Cargo ecosystem | its own Rust program in the Zelyra repository | Rust compiles the compiler; `rustc` does not compile `.zyl` | ✅ |
| Runtime | native Rust or a chosen runtime | its own Zelyra runtime, implemented in Rust | Zelyra executes its own values and rules | 🧪 |
| Memory management | ownership, borrowing, lifetimes | largely automatic/hidden for application code | no Rust borrow checker in `.zyl` code | 🧪 |
| Ownership | central Rust semantics | no equivalent `.zyl` construct | memory rules are not the same | 🗺️ |
| Borrowing | references and borrow checker | no equivalent `.zyl` construct | no Rust reference syntax | 🗺️ |
| Lifetimes | explicit or inferred lifetimes | no lifetime syntax | Zelyra does not expose this layer yet | 🗺️ |
| Static typing | mature, generic, trait-based | own checker with `Int`, `String`, `Option`, records, and more | Zelyra types are not Rust types | ✅ |
| Nullability | `Option<T>` | `T?`, such as `Email?` | Zelyra can derive schema/form rules | ✅ |
| Error handling | `Result<T, E>`, `Option<T>`, `?` operator | `Result<T, E>`, `Some`/`None`, runtime diagnostics | no Rust `?` operator syntax in Zelyra | ✅ |
| Database integration | external crates such as SQLx, Diesel, or SeaORM | intended as part of language, CLI, and runtime | different integration model; external client today | 🧪 |
| SQL | libraries, macros, or strings | native `sql<T> { ... }` with schema/parameter checks | Zelyra knows SQL as an expression | ✅ |
| MariaDB schema | not a Rust-language responsibility | tables are described in Zelyra | generator prints visible SQL | 🧪 |
| Forms | connect framework, templates, and validation yourself | `form` construct and table rules | the standard path is part of the model | 🧪 |
| CRUD | must be programmed or generated by a framework | declarative `crud Name -> table` with shared view field profile | current runtime serves CRUD routes | 🧪 |
| Views | external libraries or frameworks | `page`/`html`, named `view` layouts, typed `component` blocks, and named slots with fallbacks exist | no Rust equivalent; free styling components are still missing | 🧪 |
| Code formatting | `cargo fmt`, `rustfmt` | `zelyra fmt <file.zyl> [--check]` | deterministic Zelyra formatter, protects SQL and HTML | ✅ |
| Refactoring/impact | `rust-analyzer`, compiler APIs | `zelyra impact`, `zelyra edit` | versioned atomic JSON machine interfaces | 🧪 |
| Authentication | external web/auth crates | auth definition, sessions, and permission checks in web crate | Zelyra bundles the standard case | 🧪 |
| Permissions | user-designed types and middleware | `requires auth`, `permits`, and CRUD action permissions | declarative rules are checked server-side | 🧪 |
| Contracts | manual or library-based | native `requires {}` and `ensures {}` plus `verify` | contract syntax belongs to Zelyra | ✅ |
| Capabilities | APIs and libraries regulate effects | `uses Database`, `uses Network`, and more | effect declarations are language syntax | ✅ |
| Email | external SMTP/mail crates | no integrated email construct | do not invent `uses Email` | ❌ |
| Jobs | external job/queue systems | no background-job construct | no stable job syntax | ❌ |
| Audit | external logging/audit crates | audit table, CLI inspection, and optional hash chain | tied to auth/CRUD and still experimental | 🧪 |
| Deployment | Cargo, containers, CI, and infrastructure are free choices | generated Docker/Compose template exists | template is a development start, not a production platform | 🧪 |
| Production maturity | widely used in production | Zelyra compiler 0.2.0 is experimental | maturity and ecosystem are not comparable | 🧪 |
| Ecosystem | very large: crates, tools, frameworks | small repository and few integrations | Zelyra cannot directly import Rust crates | 🧪 |

Rust is the technical foundation, not the application language behind Zelyra. A
Rust `Customer` struct and a Zelyra `customers` table may describe similar data,
but they do not create the same behavior.

### Detailed syntax comparison

In this chapter, `✅` means that the construct is present in the current source
and was checked with the installed Rust toolchain or Zelyra CLI. `🧪`, `🗺️`, and
`❌` continue to identify limited, planned, or currently unavailable features.

| Language feature | Zelyra syntax | Rust syntax | Semantic difference | Zelyra status |
|---|---|---|---|---|
| File extension | `app.zyl` | `main.rs` | own lexer and file type | ✅ |
| Program entry | `fn main() { ... }` | `fn main() { ... }` | same spelling, different language | ✅ |
| Function definition | `fn add(a: Int) -> Int { ... }` | `fn add(a: i64) -> i64 { ... }` | own AST types | ✅ |
| Parameters | `name: String` | `name: String` | similar position, different type semantics | ✅ |
| Return type | `-> Int` | `-> i64` | Zelyra abstracts a business type | ✅ |
| Return value | `return value` | `value` or `return value;` | final Rust expression returns a value | ✅ |
| Immutable variable | `value = 1` | `let value = 1;` | Zelyra is immutable without `mutable` | ✅ |
| Mutable variable | `mutable value = 1` | `let mut value = 1;` | different marker for mutation | ✅ |
| Integer | `Int` or `UInt` | `i32`, `i64`, `u32`, `u64` | Rust requires a concrete width; Zelyra abstracts it today | ✅ |
| Decimal | `Float` or `Decimal` | `f64` or `f32` | Zelyra width and exact overflow rules are not fully specified | 🧪 |
| Boolean | `true`, `false`, `Bool` | `true`, `false`, `bool` | own base-type model | ✅ |
| String | `String` and character literals | `String`, `&str`, character literals | Rust distinguishes ownership and borrowing | ✅ |
| Optional value | `Email?` or `Option<String>` | `Option<String>` | `?` is Zelyra's Option shorthand | ✅ |
| Missing value | `None` | `None` | same name in different enum models | ✅ |
| Lists/arrays | `Int[]`, `[1, 2, 3]` | `Vec<i64>`, `vec![1, 2, 3]` | no Rust macro syntax in Zelyra | ✅ |
| Named data types | `type CustomerId = Id`, `struct Customer { ... }` | `type CustomerId = u64`, `struct Customer { ... }` | records and structs are not interchangeable | ✅ |
| Conditions | `if ok { ... }` | `if ok { ... }` | block and expression rules differ | ✅ |
| `else` | `else { ... }` | `else { ... }` | similar control-flow form | ✅ |
| `match` | `match value { Some(x) => ... None => ... }` | `match value { Some(x) => ..., None => ... }` | both require exhaustive cases | ✅ |
| Loops | `for item in items`, `while`, `loop` | `for item in items`, `while`, `loop` | no Rust iterator traits in Zelyra | ✅ |
| Function call | `add(1, 2)` | `add(1, 2)` | same surface, different name resolution | ✅ |
| Output | `print(value)` | `println!("{}", value);` | Rust uses a macro with `!` | ✅ |
| Comments | `// comment` | `// comment`, `/* ... */` | current Zelyra lexer has line comments | ✅ |
| Newlines | usually separators; continuation after operators | usually whitespace | parser rules are independent | ✅ |
| Semicolons | accepted as statement separators, not required | often statement separators | Zelyra is not semicolon-dependent | ✅ |
| Blocks | `{ ... }` | `{ ... }` | braces determine blocks in both | ✅ |
| Indentation | readability, not block semantics | readability, not block semantics | whitespace is not Python-style structure | ✅ |
| Error handling | `Result<T, E>`, `Some`/`None` | `Result<T, E>`, `?`, `panic!` | Zelyra has no Rust `?` operator | ✅ |
| String interpolation | HTML bodies may use `{name}` | `format!("{name}")` or `println!("{}", name)` | no general Zelyra string interpolation | 🧪 |
| Modules | Not yet specified | `mod name {}`, files and modules | no `mod` syntax in parser | ❌ |
| Imports | Not yet specified | `use crate::module::Item;` | no import syntax | ❌ |
| Generics | `Option<T>`, `Result<T, E>`, limited built-in type arguments | general generics and traits | no user-defined Zelyra generics | 🧪 |
| Async functions | no `async fn`; `await`/`parallel` are limited | `async fn`, `.await`, futures | no stable Zelyra async model | 🧪 |
| Tables | `table customers { ... }` | no language construct | Zelyra connects table and schema | ✅ |
| Database types | `Id`, `String(100)`, `Email`, `Bool` | Rust types and external mapping crates | Zelyra emits SQL types from a table | ✅ |
| Relationships | `department: Department required` | field plus query/mapping logic | Zelyra derives foreign keys | ✅ |
| SQL queries | `sql<Customer[]> { SELECT ... }` | string/macro from a DB crate | Zelyra checks schema, parameters, result | ✅ |
| Forms | `form CustomerCreate -> customers { ... }` | no native form | web framework and validation required | ✅ |
| CRUD | `crud Customer -> customers` | no native CRUD | Zelyra runtime serves standard routes | 🧪 |
| Views | `page`, `view SiteShell`, and `component Badge` | no native view construct | named views/components exist; `input`/`render`/`??` remain target syntax | 🧪 |
| Preconditions | `requires { amount > 0 }` | no built-in equivalent | contract is part of the Zelyra function | ✅ |
| Postconditions | `ensures { result >= 0 }` | no built-in equivalent | verifier and runtime know Zelyra contracts | ✅ |
| Old values | Not yet specified; `old(...)` is not parsed | no general built-in contract standard | do not invent `old` syntax | ❌ |
| Capabilities | `uses Database` | no identical language construct | effects are declared visibly in Zelyra | ✅ |
| Email | Not yet specified | external crate/API | no `Email` capability key | ❌ |
| Background jobs | Not yet specified | external queue/runtime crate | no job syntax | ❌ |
| Audit | `auth users { audit: auth_audit_log }` | external logging/audit crate | Zelyra ties audit to auth and CRUD events | 🧪 |
| API definitions | `api GET "/customers" { ... }` | router, handler, and types separately | Zelyra bundles route and contract | ✅ |

### Functions: same idea, different language

Both snippets express a small function in their own language. The Zelyra form
uses function syntax processed by the parser and was checked with the current
CLI.

~~~zelyra
fn add(a: Int, b: Int) -> Int {
    return a + b
}
~~~

~~~rust
fn add(a: i64, b: i64) -> i64 {
    a + b
}
~~~

Rust uses concrete integer types such as `i32`, `i64`, `u32`, or `u64`.
Zelyra offers `Int` for typical business logic; its definitive size, overflow
behavior, and database mapping still need complete specification.

### Fibonacci

~~~zelyra
fn fibonacci(n: Int) -> Int {
    if n <= 1 {
        return n
    }

    return fibonacci(n - 1) + fibonacci(n - 2)
}

fn main() {
    print(fibonacci(10))
}
~~~

~~~rust
fn fibonacci(n: u64) -> u64 {
    if n <= 1 {
        return n;
    }

    fibonacci(n - 1) + fibonacci(n - 2)
}

fn main() {
    println!("{}", fibonacci(10));
}
~~~

✅ The Zelyra version uses `Int`, `print`, and explicit `return`; Rust uses
`u64`, the `println!` macro, and the final expression as the return value. Rust
macros carry `!`. Semicolons are not required in Zelyra. Braces determine block
structure in both examples; indentation is for readability. A newline after `+`
continues the expression because the parser skips it there. Recursion works in
Zelyra only because function resolution and the runtime support it. This code
was not executed in this work run.

### Optional values

~~~zelyra
email: Email?
~~~

~~~rust
email: Option<String>
~~~

`Email?` is shorter and also expresses the domain type. Rust uses the general
generic `Option<T>`. Zelyra can account for `Email?` in table, form, and SQL
checking; a general automatic view/validation derivation is not available for
every surface.

### Tables

~~~zelyra
table customers {
    id: Id primary auto
    name: String(100) required
    email: Email?
    active: Bool default true
}
~~~

~~~rust
struct Customer {
    id: u64,
    name: String,
    email: Option<String>,
    active: bool,
}
~~~

The Rust struct creates no database table, SQL columns, validation, form, or
CRUD interface. Rust needs additional crates, macros, queries, handlers, and
templates. The Zelyra table is included in schema derivation and, where the web
features exist, in forms and CRUD.

### CRUD

~~~zelyra
crud Customer -> customers
~~~

Rust has no native equivalent. A typical Rust implementation combines a web
framework, routing, database crate, data model, queries, request types,
validation, handlers, templates or a frontend, error handling, and authorization.
Zelyra parses this declarative definition and the current runtime serves CRUD
routes, forms, search, filters, and CSRF-protected actions. This is experimental;
it is not a static frontend generator.

### SQL

The target syntax from the request included `with { ... }`. That is not in the
current parser. Parameters currently arrive as function parameters:

~~~zelyra
struct Customer { id: Int name: String email: Email? active: Bool }

fn active_customers(active: Bool) -> Customer[]
    uses Database
{
    return sql<Customer[]> {
        SELECT id, name, email, active
        FROM customers
        WHERE active = :active
        ORDER BY name
    }
}
~~~

~~~rust
let customers = sqlx::query_as!(
    Customer,
    r#"
        SELECT id, name, email, active
        FROM customers
        WHERE active = ?
        ORDER BY name
    "#,
    true
)
.fetch_all(&pool)
.await?;
~~~

Rust does not have SQL as a language feature; SQLx and other crates can provide
additional compile-time checks. Zelyra's current SQL checker validates, where
schema and types are known, tables, columns, aliases, parameters, nullability,
result mapping, and the `Database` capability. This does not automatically make
Zelyra better than SQLx. The current MariaDB generator and its missing table
options remain documented in section 7.

### Views: current syntax, not the target model

The target syntax with `view`, `input`, `render`, components, and `??` is not
current parser syntax. The current web core uses:

~~~zelyra
page "/customers/{name}" {
    html {
        <h1>Customer {name}</h1>
    }
}
~~~

Rust has no built-in HTML or component syntax; templates and web frameworks are
added externally. Zelyra's `page`/`html` form, named views, and typed components
exist. Multiple slots, nested components, and the target `input`/`render`
syntax are not stable yet; `??` is not an implemented Zelyra operator.

### Contracts

The target syntax `require amount > 0` and `old(...)` is not current syntax. The
parser accepts `requires {}` and `ensures {}`:

~~~zelyra
fn reserve(stock: Int, amount: Int) -> Int
    requires {
        amount > 0
        stock >= amount
    }
    ensures {
        result >= 0
        result == stock - amount
    }
{
    return stock - amount
}
~~~

Rust has no direct built-in equivalent. `requires` describes preconditions and
`ensures` postconditions. Runtime checks and formal verification differ:
`zelyra verify` can report `PROVEN`, `RUNTIME_CHECK`, `UNPROVEN`, or `FAILED`.
Accessing an old value with `old(...)` is not implemented.

### Capabilities

~~~zelyra
fn load_customers() -> Customer[] uses Database {
    return sql<Customer[]> {
        SELECT id, name, email, active FROM customers
    }
}
~~~

`uses` makes allowed side effects visible in a signature. `Database`, `Network`,
`FileSystem`, `Environment`, `Process`, `Clock`, `Random`, and `Console` are known
capabilities in the current runtime code. Rust has no identical built-in
capability system; access is usually organized through types, values, and
library APIs. `Email` is not a Zelyra capability.

### What Rust developers should not look for in Zelyra

For typical business applications, Zelyra is not intended to require explicit
lifetimes in ordinary application code, borrowing debug sessions for simple
forms, choosing among many integer widths, or assembling a web framework from
many crates. That is an abstraction, not a claim that memory and runtime
concerns disappear.

Zelyra is inspired by static typing, useful diagnostics, safe defaults, explicit
mutation, pattern matching, records/enums, clear boundaries, reproducible
tooling, and formal checks.

### What makes Zelyra its own language

| Zelyra feature | Benefit | Status |
|---|---|---|
| One domain definition | fewer contradictory duplicate definitions | 🧪 |
| native checked SQL | catch database errors before execution where possible | ✅ |
| declarative CRUD | standard administration with little code | 🧪 |
| `page`/`html` web core | simple typed path values and HTML responses | 🧪 |
| visible MariaDB SQL | traceable schema changes | 🧪 |
| `requires` and `ensures` | state business rules explicitly | ✅ |
| capabilities | make allowed effects visible | ✅ |
| integrated audit | trace changes | 🧪 |

### Honest conclusion

> Zelyra looks similar to Rust in places because both are modern, statically
> typed languages with braces and clear function signatures. Zelyra nevertheless
> follows a different programming model: database, SQL, forms, CRUD, views, and
> business rules are intended to be parts of one language system. Rust is the
> technical foundation of the compiler—not the language Zelyra application
> developers write.

> **Status:** Zelyra is currently an experimental language prototype. Some
> language features shown here describe the binding target and are not fully
> implemented. The status next to each example shows what exists in current
> source and was checked in this work run.

Further reading: [Introduction](#1-what-makes-zelyra-different), [language
basics](#5-variables-types-and-functions), [MariaDB](#7-mariadb-and-tables),
[SQL](#9-native-sql), [forms](#11-forms), [views/web pages](#10-web-pages),
[CRUD](#12-crud), [contracts](#15-contracts-and-verification),
[capabilities](#14-capabilities), [implementation status](#17-diagnostics-and-troubleshooting),
and [roadmap](#22-roadmap-from-the-current-repository). The live repository status is also shown on
the [status page](https://siedelmann.com/en/status).

## 21. Positioning and current development status

The current `docs/positioning.md` describes Zelyra as an independent language
for database-backed business applications. It does not replace compiler tests;
it explains how the implemented pieces are intended to fit together.

### What makes Zelyra distinct

1. **One source of truth:** schema, types, SQL, forms, CRUD, views, and APIs
   should come from definitions that can be checked together.
2. **SQL stays first-class:** SQL is not hidden behind an ORM abstraction; the
   program can check tables, parameters, and result shapes.
3. **Business features are language primitives:** tables, forms, CRUD, pages,
   authentication, permissions, and contracts belong to one model.
4. **Secure defaults are visible:** HTML escaping, parameterized SQL, CSRF
   protection, null safety, and server-side permissions are more than advice.
5. **Proof claims are honest:** `PROVEN`, `RUNTIME_CHECK`, `UNPROVEN`, and
   `FAILED` distinguish static proofs from runtime checks and open cases.
6. **A short path and a full language:** the declarative standard case is short,
   while custom functions and native SQL remain available for domain logic.
7. **A small starting stack:** the built-in server and CLI aim to support local
   learning and development without Apache, PHP, or a mandatory framework set.

The project is nevertheless an experimental prototype. The status marks and
the roadmap therefore matter more than a broad product claim.

## 22. Roadmap from the current repository

This summary comes from `docs/ROADMAP.md` in the current Zelyra repository. It
is a development plan, not a release-date promise.

| Area | Current focus | Open expansion |
|---|---|---|
| Beginner experience and distribution | source and release installers (Linux/Windows x86_64 via SHA-256), `zelyra new/init` with starter templates (`minimal`, `mariadb-crud`, `mariadb-auth`, `mariadb-business`), Docker/DB ports, `zelyra setup`, `zelyra doctor`, E2E tests | signed binaries, interactive connection assistant, reverse-proxy automation |
| Language and compiler | lexer, parser, AST/HIR, type checking, `Option`, `Result`, pattern matching, expression typed holes (`_`), canonical `zelyra fmt` | modules, imports, generics, declaration holes, and complete formal verification |
| Database platform | MariaDB, SQLite, and PostgreSQL schema CLI; typed SQL | broader schema coverage and stronger production workflows |
| Views and web | pages, named views, components, default and named slots with fallback content, safe output | themes, view inheritance, free-form styling components |
| Forms and CRUD | validation, CSRF, search, filters, pagination, actions, soft delete, shared CRUD view field profile (`view.fields`) | permanent deletion, retention, archiving, and broader view customization |
| Authentication and audit | login, sessions, roles, permissions, browser admin, audit, and hash chains | self-service, broader policy management, and archive strategies |
| APIs and integration | typed APIs, OpenAPI, TypeScript client, and CORS | versioning, rate limits, and OAuth/integration features |
| Verification and operations | contracts, capability checks, and first concurrency building blocks | cancellation, timeouts, database-pool integration, and production performance |
| AI-native interfaces | `zelyra fmt` (Stage B ✅), expression holes `_` (Stage C 🧪), `zelyra impact` with `--symbol` (Stage D 🧪), `zelyra edit` rename (Stage E 🧪) | declaration holes, schema/runtime impact, complex edit operations, AI benchmark |
| Quality and governance | tests, documentation, and reproducible checks | broader acceptance applications and production hardening |

Do not document email or background-job systems, complete modules/imports,
multi-slot free-form `view` components, or automatic production migrations as
available. Each remains 🗺️ until the current CLI and runtime implement it.

For a learning project, the useful order is:

1. `check` and `run` for language basics;
2. `db create`, `db inspect`, `db plan`, and a reviewed `db apply`;
3. a small `page`, `form`, or `crud` application;
4. only then authentication, roles, audit, and API integration.

## 23. AI-native development

The current architecture and specification documents add an important
principle:

> **The AI writes. Zelyra checks.**

Zelyra should be useful to humans and AI systems alike while remaining fully
independent of AI. The compiler and tests are the trust boundary; a plausible
model explanation is not proof of correctness. Human-written and generated
code go through the same lexer, parser, name, type, SQL, capability, contract,
test, and runtime checks.

### Machine interfaces available today

🧪 Tool-facing JSON uses a common envelope with `schema_version: "1"`. Human
output remains the default; request JSON explicitly with `--format=json`:

~~~json
{
    "schema_version": "1",
    "command": "check",
    "success": false,
    "diagnostics": []
}
~~~

Output is deterministic. `schema_version` is mandatory; new optional fields
may be added within a version, while incompatible changes require a new
version. JSON goes only to `stdout`, technical messages to `stderr`. Source
spans use zero-based UTF-8 byte offsets, one-based line/byte columns, and a
half-open interval. Secrets, timestamps, random IDs, machine-dependent
absolute paths, and live database contents do not belong in these outputs.

#### Canonical code formatting

✅ `zelyra fmt <file.zyl>` produces deterministic code formatting after
successful lexing and parsing. `zelyra fmt <file.zyl> --check` writes no files
and returns an error code if reformatting is required, allowing CI to enforce
canonical source formatting.

~~~bash
zelyra fmt examples/fibonacci.zyl
zelyra fmt examples/fibonacci.zyl --check
~~~

The formatter preserves line comments and treats SQL and HTML blocks as opaque
source. It is idempotent: formatting an already formatted document yields
byte-for-byte identical output.

#### Typed holes

✅ Expression typed holes with `_` are available as an initial safe stage. The
compiler reports expected contextual types, visible values and functions, active
capabilities, contract obligations, and source spans:

~~~zelyra
fn double(x: Int) -> Int {
    return _
}
~~~

`zelyra check` reports diagnosis `E-HOLE-001` with the expected type `Int` and
visible identifiers. Buildable commands (`build`, `run`, `serve`) reject
incomplete code before lowering and execution. Declaration-level holes remain
planned.

#### Structured project context

✅ `context` is read-only: it does not connect to MariaDB, use the network, send
email, run jobs, or expose secrets:

~~~bash
zelyra context examples/auth_crud_api.zyl --format=json
~~~

It reports declared functions, tables, SQL queries, CRUD resources, forms, APIs,
and source spans.

#### Deterministic impact analysis

🧪 Source dependencies can be deterministically analyzed:

~~~bash
zelyra impact examples/auth_crud_api.zyl --format=json
zelyra impact examples/auth_crud_api.zyl --symbol table:customers --format=json
~~~

The impact response reports source-based tables, SQL, forms, CRUD resources,
views, APIs, permissions, contracts, and a deterministic `references` edge
list for known relationships. Each known edge includes source, target, kind,
and source span. Email, job, test, and live schema impacts remain explicitly
empty or unavailable; the command never connects to MariaDB.

Using `--symbol <kind:name>` focuses the output on a known node such as
`table:customers`. The focused response contains only directly connected
references and related node IDs. Unknown nodes return `E-IMPACT-001` and a
non-zero exit code.

#### Atomic semantic edits

🧪 A validated symbol rename can be previewed without modifying source:

~~~json
{
  "schema_version": "1",
  "entry": "examples/fibonacci.zyl",
  "expected_source_fingerprint": "fnv1a64:18f35ecb3e2f99c4",
  "operations": [
    {"kind": "rename", "symbol": "function", "from": "fibonacci", "to": "fib"}
  ]
}
~~~

Save this as `change.json` and run:

~~~bash
zelyra edit --format=json change.json
~~~

The request is versioned and must point to an existing `.zyl` file inside the
resolved Zelyra project root. Source code before and after the change must pass
all compiler checks. The result reports exact token spans and a deterministic
source fingerprint.

When applying changes with `--apply`, the request must include the fingerprint
from the preview to prevent overwriting concurrently modified files (stale-source
protection). Without the explicit `--apply` flag, it remains a preview:

~~~bash
zelyra edit --format=json --apply change.json
~~~

Before atomic replacement, the source is re-parsed and fully validated; an
invalid or semantically unsafe change will not be written.

Renames of functions, types, and records are AST-based: declarations and known
references are updated while local bindings of the same name remain untouched.
Table, view, form, and CRUD declarations and their structured references are
also supported. Table renames update checked SQL table positions (`FROM`,
`JOIN`, `INTO`, `UPDATE`) while preserving literals, comments, parameters, and
HTML. Component renames update both the declaration and known opening and
closing component tags in HTML bodies.

### Safety boundary and benchmark

AI tools must not silently add capabilities, widen permissions, execute
destructive SQL, weaken diagnostics, disable tests, or reveal secrets.
Destructive schema changes and security-sensitive changes require visible human
approval. Zelyra does not automatically send source code to external AI
services; planned integrations should be open, local-capable, vendor-neutral,
and versioned.

The new AI-authoring benchmark is a specification in
`docs/benchmarks/ai-authoring.md`. With versioned fixtures and identical tasks,
it is intended to measure first-pass compilation, correction loops, time to
passing tests, tokens, security failures, missed dependencies, unsafe schema
changes, and human review effort. No comparative results have been published.
A secret leak or unapproved destructive change is a security failure and cannot
be balanced by claimed productivity.

# APPENDICES

---

## 24. Authoritative Sources and Compiler Verification (Source Authority)

> **Principle:** Zelyra is its own language. The parser and verified tests decide what exists.

When sources disagree, report the disagreement explicitly and use this authoritative order:

1. **[Formal Language Specification](specification.md) and phase documents**
2. **Compiler implementation code:** Lexer, AST, parser, name resolution, type checking, and semantic compiler code across `lexer`, `parser`, `ast`, `hir`, `cli`, `runtime`, `database`, `web`, and `forms`
3. **Official automated language and integration tests:** Workspace tests (`cargo test --workspace`), machine-interface tests, and E2E scripts in `tests/`
4. **Official standard library:** (once formally introduced)
5. **Official Zelyra examples:** `.zyl` files in `examples/` verified by the current compiler
6. **Documentation and handbook explanations**

### Invariants for developers and AI agents

- **Roadmap is planning, not syntax:** Future phase proposals become valid syntax only after implementation in lexer and parser.
- **Rust is the implementation language, not Zelyra:** Zelyra is developed in Rust, but Rust syntax in a `.zyl` file is invalid unless Zelyra's grammar explicitly defines it.
- **No invented commands:** Every CLI command must exist in `cli/src/main.rs`.
- **Quality gate:** Every feature undergoes formatting (`cargo fmt`), workspace checks (`cargo check`), clippy linter (`cargo clippy`), and test suite execution (`cargo test`).

---

## Appendix A: Quickstart / Cheat Sheet (Syntax Cheat Sheet)

### Basic Syntax
```zelyra
// Functions with contracts
fn sum(a: Int, b: Int) -> Int
    requires { a >= 0 && b >= 0 }
    ensures { result >= 0 }
{
    return a + b
}

// Entry point and variables
fn main() {
    x = 10                  // Type inference (immutable)
    mutable counter = 0     // Mutable
    name: String = "Zelyra" // Explicit type

    print(sum(3, 7))
}
```

### Types
- Numbers: `Int` (64-bit signed integer), `UInt` (unsigned integer), `Float`, `Decimal` (fixed-point arithmetic)
- Text & Characters: `String`, `Char`
- Booleans: `Bool` (`true`, `false`)
- Collections: `Int[]`, `String[]`
- Optionality: `Option<T>` (`Some(x)`, `None`), shorthand `T?`
- Error Handling: `Result<T, E>` (`Ok(x)`, `Err(e)`)
- System & Chronology: `Timestamp`, `Date`, `Time`, `Duration`

### Control Flow
```zelyra
fn control_flow(x: Int) {
    if x > 10 {
        print("Large")
    } else {
        print("Small")
    }

    match x {
        1 => { print("One") }
        2 => { print("Two") }
        _ => { print("Other") }
    }

    mutable i = 0
    while i < 3 invariant { i >= 0 } {
        i = i + 1
    }

    for n in [1, 2, 3] {
        print(n)
    }
}

fn main() {
    control_flow(1)
}
```

### Database & Web
```zelyra
database main {
    engine: mariadb
    database: "app"
}

table items {
    id: Id primary auto
    description: String required
}

page "/items" {
    html {
        <h1>Item List</h1>
    }
}
```

---

## Appendix B: Complete Zelyra Diagnostic & Error Code Reference

| Error Code | Category | Description | Typical Fix |
| :--- | :--- | :--- | :--- |
| `E-LEX-001` | Lexer | Unexpected character / lexical error | Remove typographical or illegal special characters |
| `E-PARSE-001` | Parser | Syntax error (e.g. missing brace, invalid token) | Correct syntax per Zelyra grammar |
| `E-NAME-001` | Resolution | Unknown name / variable not declared | Check declaration or correct typo |
| `E-TYPE-001` | Type checking | Type mismatch (e.g. String assigned to Int) | Align types or add conversion |
| `E-FEATURE-001` | Feature switch | Access to disabled language surface (`web`, `api`, `crud`, `auth`, `audit`) | Enable feature in `zelyra.toml` or `.env` |
| `E-CAP-001` / `E-CAP-002` | Capabilities | Missing capability permission (e.g. `database`, `network`) | Grant capability in `zelyra.toml` under `[capabilities]` |
| `E-POLICY-001` / `002` | Policy | Security or audit policy violation | Review security policy declarations |
| `E-DB-001` - `E-DB-005` | Database | Database connection or driver error | Verify `DATABASE_URL`, ensure MariaDB is running |
| `E-SQL-001` - `E-SQL-004` | SQL | Invalid SQL / schema mismatch / unknown column | Check SQL statement against table schema |
| `E-VIEW-001` - `E-VIEW-009` | Views & Pages | Error in view interpolation, slots, or data binding | Check slot names and binding data types |
| `E-VIEW-010` - `E-VIEW-015` | Query controls | Invalid search, sort, pagination, or filter fields | Check declared whitelist (`search`, `sort`, `filter`) |
| `E-FORM-001` - `E-FORM-004` | Forms | Validation error or invalid form field types | Review form declaration and input payload |
| `E-CRUD-001` - `E-CRUD-006` | CRUD | Invalid CRUD resource, schema conflict, or layout error | Verify table binding and layout slots |
| `E-AUTH-001` - `E-AUTH-028` | Authentication | Session, password, or permission conflict | Check roles (`permits`), `requires auth`, and password hashes |
| `E-AUDIT-001` - `E-AUDIT-010` | Audit trail | Failure in cryptographic hash chain of audit log | Validate checksums and audit table integrity |
| `E-SETUP-001` - `E-SETUP-006` | Setup flow | Port conflict, socket error, or Compose failure | Choose free ports, check Docker socket permissions |
| `E-SETUP-WEB-001` | Web setup | Invalid or expired setup token | Restart setup assistant and use tokenized URL |
| `E-IMPACT-001` | Impact analysis | Cyclical or invalid dependencies | Untangle code dependencies |
| `E-RUNTIME-001` | Runtime | Unhandled runtime error | Check contracts (`requires`, `ensures`) or error values |

---

## Appendix C: Zelyra CLI Command Reference

| Command | Option / Flag | Description |
| :--- | :--- | :--- |
| `zelyra --version` | | Outputs full compiler and package version |
| `zelyra new <dir>` | `--template minimal\|mariadb-crud\|...` | Creates a new Zelyra project with template |
| | `--mariadb` | Generates MariaDB project with Compose, Dockerfile, and `.env` |
| | `--web-port <p> --host-port <p> --db-host-port <p>` | Configures container and host ports |
| `zelyra init` | `[--mariadb]` | Initializes current directory as Zelyra project |
| `zelyra check <file.zyl>` | `[--format json]` | Statically checks syntax, types, contracts, and capabilities |
| `zelyra run <file.zyl>` | | Compiles and executes a Zelyra program |
| `zelyra serve <file.zyl>` | `[host:port]` | Starts the built-in HTTP web server |
| `zelyra setup` | `[--database]` | Starts Docker Compose / MariaDB |
| | `[--schema]` | Starts environment and applies database schema |
| | `[--all]` | Executes configuration, container startup, and schema migration |
| | `[--host-port <p>] [--db-host-port <p>]` | Enforces exact host ports for new `.env` |
| `zelyra setup --web` | `[--port <p>]` | Starts local token-protected browser setup assistant |
| `zelyra config <file.zyl>` | `[--format json]` | Safely displays effective configuration and feature switches |
| `zelyra doctor <file.zyl>` | `[--port <p>] [--json]` | Checks toolchain, MariaDB, Docker, and ports non-destructively |
| | `[--env-file <file>]` | Reads `DATABASE_URL` from specified file |
| `zelyra fmt <file.zyl>` | `[--check]` | Formats source code according to official standards |
| `zelyra verify <file.zyl>` | | Performs formal contract verification |
| `zelyra doc <file.zyl>` | `--openapi` | Generates OpenAPI 3.0 specifications |
| | `--typescript` | Generates typed dependency-free TypeScript client |
| `zelyra db init <file.zyl>` | | Initializes database and base tables |
| `zelyra db setup <file.zyl>` | | Sets up MariaDB database initially |
| `zelyra db apply <file.zyl>` | | Applies schema migrations safely |
| `zelyra auth hash-password` | `[--stdin]` | Generates secure Argon2 password hashes |
| `zelyra form validate <file> <Form>` | | Tests forms with sample values on console |
| `zelyra context <file.zyl>` | `[--format json]` | Emits semantic source context for developer tools |

---

## Appendix D: Standard Library Overview

### Pure Core Functions (Zero Capabilities Required)
- `print(value)`: Prints any value to standard output.
- `len(array)`: Returns the number of elements in an array as an `Int`.
- `append(array, element)`: Produces a new array with the appended value.
- `contains(array, element)` -> `Bool`: Checks whether an item is present in an array.
- `first(array)` -> `Option<T>`: Returns the first item wrapped in `Some`, or `None`.
- `last(array)` -> `Option<T>`: Returns the final item wrapped in `Some`, or `None`.
- `get(map, key)` -> `Option<V>`: Looks up a key within a `Map<K, V>`.
- `put(map, key, value)` -> `Map<K, V>`: Inserts or updates a key-value pair functionally.
- `keys(map)` -> `K[]`: Returns all keys of a map as an array.
- `values(map)` -> `V[]`: Returns all values of a map as an array.
- `Some(value)` / `None`: Value constructors for the `Option<T>` type.
- `Ok(value)` / `Err(error)`: Value constructors for the `Result<T, E>` type.
- `json_encode(value)` -> `String`: Serializes typed data into a JSON string.
- `json_decode<T>(text)` -> `T`: Parses typed JSON; invalid data are reported as runtime errors.

### Capability-Guarded Functions
- `uses Console`:
  - `read_console(prompt: String)` -> `String?`: Displays the prompt and reads one line; `None` represents EOF.
- `uses Clock`:
  - `now()` -> `Timestamp`: Returns current system timestamp.
- `uses Random`:
  - `random_int(min: Int, max: Int)` -> `Int`: Generates an integer in the given range.
- `uses Environment`:
  - `env(name: String)` -> `Option<String>`: Reads a host environment variable.
- `uses FileSystem`:
  - `read_text(path: String)` -> `String`: Reads file contents as text.
  - `write_text(path: String, content: String)`: Writes text content to file.
  - `delete_file(path: String)`: Deletes a target file.
  - `list_dir(dir: String)` -> `String[]`: Lists directory filenames.
- `uses Database`:
  - `sql<T[]> { SELECT ... }`: Executes type-checked SQL queries returning entity records.
  - `transaction { ... }`: Wraps multiple SQL operations inside an atomic transaction.

---

## Appendix E: SQL Cheat Sheet for Zelyra Developers

SQL statements embedded directly within Zelyra are executed with `sql<T[]>` or `sql`:

```zelyra
database main {
    engine: mariadb
    database: "app"
}

table tasks {
    id: Id primary auto
    name: String required
    completed: Bool default false
}

fn sql_examples() uses Database {
    // 1. SELECT with typed return type and safe parameter
    status = false
    filtered = sql<Task[]> {
        SELECT id, name, completed
        FROM tasks
        WHERE completed = :status
    }

    // 2. INSERT within a transaction
    text = "New task"
    transaction {
        sql {
            INSERT INTO tasks (name, completed)
            VALUES (:text, false)
        }
    }

    // 3. UPDATE
    target_id = 1
    transaction {
        sql {
            UPDATE tasks
            SET completed = true
            WHERE id = :target_id
        }
    }
}

fn main() uses Database {
    print("SQL cheat sheet validated.")
}
```

---

## Appendix F: HTML and Web Reference in Zelyra

### Web structures and declarations

| Element | Declaration | Purpose |
| :--- | :--- | :--- |
| **Page** | `page "/path/{param}" { ... }` | Defines an HTTP GET route with path parameters and HTML response |
| **View Layout** | `view LayoutName { html { ... <slot /> ... } }` | Reusable layout with default and named slots |
| **Component** | `component Name { props { ... } html { ... } }` | Reusable HTML component with typed properties |
| **Named Slot** | `<slot name="header">Fallback</slot>` | Placeholder in layout/component with optional default content |
| **Slot Injection** | `<slot name="header">Content</slot>` | Passes child content into matching slot |
| **Data Loading** | `load item = sql<Item> { SELECT ... }` | Typed loading of single record with field access `{item.field}` |
| **Collection Loop**| `for item in items { <li>{item.name}</li> }` | Typed server-side iteration over loaded collection |
| **Search Control** | `search { col1 col2 }` | Whitelist-checked URL search with parameterized `LIKE` query |
| **Sort Control** | `sort { col1 col2 }` | Typed sorting via `?sort=col&order=asc\|desc` |
| **Pagination** | `paginated 25` | Pagination with `LIMIT`/`OFFSET`, `page`, `pages`, and `total` |
| **Filter Control** | `filter { col1 col2 }` | Typed filter operators (`eq`, `contains`, `starts_with`, `gt`, `lte` etc.) |
| **CRUD Layout** | `crud Res { table tbl layout: LayoutName }` | Embeds CRUD views into slots `title`, `nav`, `content`, `actions` |

---

## Appendix G: Glossary of Technical Terms

- **AST (Abstract Syntax Tree):** The hierarchical tree data structure representing the syntactic elements of your source code following lexical and grammatical analysis.
- **Capability:** An explicit permission marker (`uses FileSystem`, `uses Database`, etc.) required on a function's signature before it is permitted to touch guarded external system resources.
- **Design by Contract:** A formal software design methodology where functions establish strict preconditions (`requires`) and postconditions (`ensures`) enforced by static analysis and runtime checks.
- **Immutable:** Unchangeable after initial assignment. In Zelyra, all variables are immutable by default unless explicitly declared with the `mutable` keyword.
- **Invariant:** A logical condition (such as in a loop) that is guaranteed to evaluate to true before and after every execution cycle.
- **Option:** A algebraic data type (`Some(v)` or `None`) representing the potential absence of a value—Zelyra's robust replacement for dreaded null-pointer exceptions.
- **Result:** A type (`Ok(v)` or `Err(e)`) that encapsulates the outcome of an operation that might fail, treating errors as first-class recoverable values rather than untyped exceptions.
- **Typed Hole (`_`):** A compiler-supported placeholder indicating incomplete code, which signals the compiler and AI tooling to display the required type, local scope, and applicable contracts for that location.

---

## Appendix H: Solutions to Chapter Exercises

### Chapter 1: Greeting
```zelyra
fn main() {
    print("Hello World from Zelyra!")
}
```

### Chapter 5: Calculate Discount Price
```zelyra
fn calculate_discount(original: Float, percent: Float) -> Float {
    return original * (1.0 - (percent / 100.0))
}

fn main() {
    print(calculate_discount(100.0, 20.0))
}
```

### Chapter 11: Double Numbers
```zelyra
fn double_number(number: Int) -> Int {
    return number * 2
}

fn main() {
    print(double_number(21))
}
```

### Chapter 12: Pre- and Postconditions
```zelyra
fn clamp(value: Int, min_val: Int, max_val: Int) -> Int
    requires { min_val <= max_val }
    ensures { result >= min_val && result <= max_val }
{
    if value < min_val { return min_val }
    if value > max_val { return max_val }
    return value
}

fn main() {
    print(clamp(120, 0, 100))
}
```

### Chapter 13: Sum an Array
```zelyra
fn sum_array(numbers: Int[]) -> Int {
    mutable total = 0
    for n in numbers {
        total = total + n
    }
    return total
}

fn main() {
    items: Int[] = [1, 2, 3, 4, 5]
    print(sum_array(items))
}
```

---

## Appendix I: Frequently Asked Questions (FAQ)

**Question: Why is there no `import` statement in Zelyra 0.2.0?**
*Answer:* In version 0.2.0, the Zelyra compiler analyzes all `.zyl` source
files within the project context as a single unified compilation unit. A
fine-grained module and import system remains future roadmap work.

**Question: Can I build command-line applications with Zelyra?**
*Answer:* Yes. `print()` emits values. `read_console("Prompt: ")` reads a line and returns `String?`; the function needs `uses Console` and the project may also need `console = true`.

**Question: Why does Zelyra emphasize MariaDB as its primary database engine?**
*Answer:* MariaDB provides exceptional transactional performance, open-source licensing, robust cloud compatibility, and rock-solid reliability for enterprise web applications.

---

## Appendix J: Next Resources and Community

- **Official GitHub Repository:** [https://github.com/sf1976/zelyra](https://github.com/sf1976/zelyra)
- **Documentation & Online Handbook:** [https://siedelmann.com/handbook](https://siedelmann.com/handbook) / [https://siedelmann.com/handbuch](https://siedelmann.com/handbuch)
- **Examples & Templates:** Check the `examples/` directory in the official repository for runnable templates covering authentication, CRUD views, REST APIs, and database migrations.
