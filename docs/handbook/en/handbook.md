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

- **[PART I – UNDERSTANDING ZELYRA AND PROGRAMMING](textbook-foundations.md#part-i-understanding-zelyra-and-programming)**
  - [Chapter 1: Welcome to Zelyra](textbook-foundations.md#chapter-1-welcome-to-zelyra)
  - [Chapter 2: How a Program Works](textbook-foundations.md#chapter-2-how-a-program-works)
  - [Chapter 3: Installing and Setting Up Zelyra](textbook-foundations.md#chapter-3-installing-and-setting-up-zelyra)
  - [Chapter 4: The First Zelyra Project](textbook-foundations.md#chapter-4-the-first-zelyra-project)
- **[PART II – LANGUAGE FUNDAMENTALS](textbook-foundations.md#part-ii-language-fundamentals)**
  - [Chapter 5: Values and Data Types](textbook-foundations.md#chapter-5-values-and-data-types)
  - [Chapter 6: Variables and Immutability](textbook-foundations.md#chapter-6-variables-and-immutability)
  - [Chapter 7: Operators and Expressions](textbook-foundations.md#chapter-7-operators-and-expressions)
  - [Chapter 8: Input and Output](textbook-foundations.md#chapter-8-input-and-output)
  - [Chapter 9: Decisions with Conditions](textbook-foundations.md#chapter-9-decisions-with-conditions)
  - [Chapter 10: Repetition and Loops](textbook-foundations.md#chapter-10-repetition-and-loops)
- **[PART III – STRUCTURING PROGRAMS](textbook-foundations.md#part-iii-structuring-programs)**
  - [Chapter 11: Functions and Procedures](textbook-foundations.md#chapter-11-functions-and-procedures)
  - [Chapter 12: Contracts and Preconditions (Design by Contract)](textbook-foundations.md#chapter-12-contracts-and-preconditions-design-by-contract)
  - [Chapter 13: Collections, Lists, and Dictionaries (Arrays & Maps)](textbook-foundations.md#chapter-13-collections-lists-and-dictionaries-arrays-and-maps)
  - [Chapter 14: Creating Custom Data Types (Records & Tables)](textbook-foundations.md#chapter-14-creating-custom-data-types-records-tables)
  - [Chapter 15: Modules and Code Organization](textbook-foundations.md#chapter-15-modules-and-code-organization)
- **[PART IV – SAFETY AND ERROR HANDLING](textbook-foundations.md#part-iv-safety-and-error-handling)**
  - [Chapter 16: Error Types and Their Causes](textbook-foundations.md#chapter-16-error-types-and-their-causes)
  - [Chapter 17: Errors as Values – The Result Pattern](textbook-foundations.md#chapter-17-errors-as-values-the-result-pattern)
  - [Chapter 18: Nothingness Does Not Exist – Working Safely with Option](textbook-foundations.md#chapter-18-nothingness-does-not-exist-working-safely-with-option)
  - [Chapter 19: Testing and Quality Assurance](textbook-foundations.md#chapter-19-testing-and-quality-assurance)
- **[PART V – PRACTICAL DATA PROCESSING](textbook-foundations.md#part-v-practical-data-processing)**
  - [Chapter 20: Working with Files](textbook-foundations.md#chapter-20-working-with-files)
  - [Chapter 21: Date, Time, Randomness, and Structured Data](textbook-foundations.md#chapter-21-date-time-randomness-and-structured-data)
  - [Chapter 22: Concurrency and Background Tasks](textbook-foundations.md#chapter-22-concurrency-and-background-tasks)
- **[PART VI – DATABASES WITH ZELYRA](textbook-applications.md#part-vi-databases-with-zelyra)**
  - [Chapter 23: Why Zelyra Understands Databases Directly](textbook-applications.md#chapter-23-why-zelyra-understands-databases-directly)
  - [Chapter 24: Defining Tables and Data Modeling](textbook-applications.md#chapter-24-defining-tables-and-data-modeling)
  - [Chapter 25: Querying and Modifying Data](textbook-applications.md#chapter-25-querying-and-modifying-data)
- **[PART VII – WEB APPLICATIONS AND FORMS](textbook-applications.md#part-vii-web-applications-and-forms)**
  - [Chapter 26: Rendering Web Pages](textbook-applications.md#chapter-26-rendering-web-pages)
  - [Chapter 27: Forms and User Inputs](textbook-applications.md#chapter-27-forms-and-user-inputs)
  - [Chapter 28: The Complete CRUD Pattern](textbook-applications.md#chapter-28-the-complete-crud-pattern)
  - [Chapter 29: Users, Passwords, and Sessions](textbook-applications.md#chapter-29-users-passwords-and-sessions)
  - [Chapter 30: APIs and Data Exchange](textbook-applications.md#chapter-30-apis-and-data-exchange)
- **[PART VIII – THE DISTINCTIVE FEATURES OF ZELYRA](textbook-applications.md#part-viii-the-distinctive-features-of-zelyra)**
  - [Chapter 31: Readability as the Highest Priority](textbook-applications.md#chapter-31-readability-as-the-highest-priority)
  - [Chapter 32: AI-Nativity – Why Zelyra Is Built for AI Assistants](textbook-applications.md#chapter-32-ai-nativity-why-zelyra-is-built-for-ai-assistants)
  - [Chapter 33: Safety through Capabilities](textbook-applications.md#chapter-33-safety-through-capabilities)
  - [Chapter 34: Zelyra in Comparison](textbook-applications.md#chapter-34-zelyra-in-comparison)
- **[PART IX – FROM DESIGN TO FINISHED APPLICATION](textbook-applications.md#part-ix-from-design-to-finished-application)**
  - [Chapter 35: Planning Software – From Idea to Design](textbook-applications.md#chapter-35-planning-software-from-idea-to-design)
  - [Chapter 36: Architecture and Clean Code Structure](textbook-applications.md#chapter-36-architecture-and-clean-code-structure)
  - [Chapter 37: Configuration and Environment Variables](textbook-applications.md#chapter-37-configuration-and-environment-variables)
  - [Chapter 38: Debugging and Optimization](textbook-applications.md#chapter-38-debugging-and-optimization)
  - [Chapter 39: Deployment and Operations](textbook-applications.md#chapter-39-deployment-and-operations)
- **[PART X – CAPSTONE PROJECT AND LOOKING AHEAD](textbook-applications.md#part-x-capstone-project-and-looking-ahead)**
  - [Chapter 40: The Grand Capstone Project: Complete Task Management](textbook-applications.md#chapter-40-the-grand-capstone-project-complete-task-management)
  - [Chapter 41: The Zelyra Roadmap (From 0.3.0 to 1.0)](textbook-applications.md#chapter-41-the-zelyra-roadmap-from-030-to-10)
  - [Chapter 42: Your Journey as a Zelyra Developer](textbook-applications.md#chapter-42-your-journey-as-a-zelyra-developer)

### Technical Reference Manual

- [1. What makes Zelyra different](technical-reference.md#1-what-makes-zelyra-different)
- [2. Installation](technical-reference.md#2-installation)
- [3. Your first program](technical-reference.md#3-your-first-program)
- [4. Create a project and use the CLI](technical-reference.md#4-create-a-project-and-use-the-cli)
- [5. Variables, types, and functions](technical-reference.md#5-variables-types-and-functions)
- [6. Option, Result, and pattern matching](technical-reference.md#6-option-result-and-pattern-matching)
- [7. MariaDB and tables](technical-reference.md#7-mariadb-and-tables)
- [8. Inspecting and applying schemas](technical-reference.md#8-inspecting-and-applying-schemas)
- [9. Native SQL](technical-reference.md#9-native-sql)
- [10. Web pages](technical-reference.md#10-web-pages)
- [11. Forms](technical-reference.md#11-forms)
- [12. CRUD](technical-reference.md#12-crud)
- [13. Authentication and permissions](technical-reference.md#13-authentication-and-permissions)
- [14. Capabilities](technical-reference.md#14-capabilities)
- [15. Contracts and verification](technical-reference.md#15-contracts-and-verification)
- [16. Configuration and secrets](technical-reference.md#16-configuration-and-secrets)
- [17. Diagnostics and troubleshooting](technical-reference.md#17-diagnostics-and-troubleshooting)
- [18. Testing and contributing](technical-reference.md#18-testing-and-contributing)
- [19. What comes next](technical-reference.md#19-what-comes-next)
- [20. Zelyra compared with Rust](technical-reference.md#20-zelyra-compared-with-rust)
- [21. Positioning and current development status](technical-reference.md#21-positioning-and-current-development-status)
- [22. Roadmap from the current repository](technical-reference.md#22-roadmap-from-the-current-repository)
- [23. AI-native development](technical-reference.md#23-ai-native-development)
- [24. Authoritative Sources and Compiler Verification (Source Authority)](appendices.md#24-authoritative-sources-and-compiler-verification-source-authority)

### Appendices

- [Appendix A: Quickstart / Cheat Sheet (Syntax Cheat Sheet)](appendices.md#appendix-a-quickstart-cheat-sheet-syntax-cheat-sheet)
- [Appendix B: Complete Zelyra Diagnostic & Error Code Reference](appendices.md#appendix-b-complete-zelyra-diagnostic-error-code-reference)
- [Appendix C: Zelyra CLI Command Reference](appendices.md#appendix-c-zelyra-cli-command-reference)
- [Appendix D: Standard Library Overview](appendices.md#appendix-d-standard-library-overview)
- [Appendix E: SQL Cheat Sheet for Zelyra Developers](appendices.md#appendix-e-sql-cheat-sheet-for-zelyra-developers)
- [Appendix F: HTML and Web Reference in Zelyra](appendices.md#appendix-f-html-and-web-reference-in-zelyra)
- [Appendix G: Glossary of Technical Terms](appendices.md#appendix-g-glossary-of-technical-terms)
- [Appendix H: Solutions to Chapter Exercises](appendices.md#appendix-h-solutions-to-chapter-exercises)
- [Appendix I: Frequently Asked Questions (FAQ)](appendices.md#appendix-i-frequently-asked-questions-faq)
- [Appendix J: Next Resources and Community](appendices.md#appendix-j-next-resources-and-community)
