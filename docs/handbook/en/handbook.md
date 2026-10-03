Warning: truncated output (original token count: 81285)
Total output lines: 8495

# The Zelyra Handbook

**From foundations to database-backed web applications. State intent. Prove correctness.**

English · [Deutsche Ausgabe](/handbuch)

Welcome to the complete Zelyra Handbook. It includes both the introductory textbook **"Learning Zelyra – Understandable Programming from the Foundations to Your Own Application"** (Parts I to X, Chapters 1 to 42), the **Technical Reference Manual** (Chapters 1 to 23), and comprehensive **Appendices** (A to J).

> **Project status:** Compiler 0.3.0 implements a tested, experimental subset of language line 0.1. Zelyra is not approved for production use.

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
  - [Chapter 41: The Zelyra Roadmap (From 0.3.0 to 1.0)](#chapter-41-the-zelyra-roadmap-from-030-to-10)
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
- How to install Zelyra from source (`./install.sh` / `install.ps1`) or the stable release archive (`--release v0.3.0`).
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
./install.sh --release v0.3.0
```
On Windows (PowerShell):
```powershell
git clone https://github.com/sf1976/zelyra.git
Set-Location zelyra
.\install.ps1 -Release v0.3.0
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
3. The stable release can be installed without Rust using `--release v0.3.0`.
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
In the real world, you cannot add apples to oranges. Without a type system, however, a computer would do precisely that: it would happily attempt to m…66285 tokens truncated…### Capabilities

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

**Question: Why is there no `import` statement in Zelyra 0.3.0?**
*Answer:* The module system has not been implemented yet. A CLI invocation
checks only the explicitly named `.zyl` source file; other files are neither
discovered automatically nor linked into the same program. Project imports
and module boundaries are planned for 0.4.0 and must not be assumed to work
today.

**Question: Can I build command-line applications with Zelyra?**
*Answer:* Yes. `print()` emits values. `read_console("Prompt: ")` reads a line and returns `String?`; the function needs `uses Console` and the project may also need `console = true`.

**Question: Why does Zelyra emphasize MariaDB as its primary database engine?**
*Answer:* MariaDB provides exceptional transactional performance, open-source licensing, robust cloud compatibility, and rock-solid reliability for enterprise web applications.

---

## Appendix J: Next Resources and Community

- **Official GitHub Repository:** [https://github.com/sf1976/zelyra](https://github.com/sf1976/zelyra)
- **Documentation & Online Handbook:** [https://siedelmann.com/handbook](https://siedelmann.com/handbook) / [https://siedelmann.com/handbuch](https://siedelmann.com/handbuch)
- **Examples & Templates:** Check the `examples/` directory in the official repository for runnable templates covering authentication, CRUD views, REST APIs, and database migrations.
