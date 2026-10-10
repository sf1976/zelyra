# Readable regex text editing

**Status:** design for the planned 0.8.0 milestone; not an implemented CLI contract.

Zelyra should make repeated source edits understandable at a glance. A change
plan names its purpose, target files, regular expression, replacement, and
expected match count. Preview is the default. Writing files requires an
explicit `--apply`.

## Example

A TOML plan keeps the explanation and the expression together:

```toml
[[change]]
name = "Show family name first"
files = ["src/**/*.zyl"]
find = '''(?x)
  (?P<given> [[:alpha:]]+ )
  \s+
  (?P<family> [[:alpha:]]+ )
'''
replace = "${family}, ${given}"
expect = 3
```

Run it from the project root:

```sh
zelyra text edit names.zedit
# The preview prints a SHA-256 plan id that covers the rules and input files.
zelyra text edit names.zedit --apply --plan-id sha256:<id-from-preview>
```

The first command prints each file, match count, proposed diff, and a SHA-256
plan id without changing files. The second applies that exact reviewed plan.
Zelyra recomputes the id from the rules and current file contents; if anything
has changed since preview, application stops. If a match count differs from
`expect`, application also stops and reports the files and counts. A
deliberately broad edit therefore cannot silently touch a new set of code
after the preview was reviewed. Applying without the printed plan id is an
error.

## Language and safety rules

- Use a documented, linear-time regular-expression engine. Named captures use
  `(?P<name>...)`; replacements refer to them as `${name}`. Backreferences in
  patterns, look-around, code evaluation, and shell interpolation are rejected.
- Keep regex flags in the pattern (`(?i)` for case-insensitive matching,
  `(?m)` for multiline anchors, and `(?x)` for whitespace and comments). This
  avoids hidden command-line modes.
- Interpret file globs beneath the detected project root. Never follow symlinks
  outside that root, and exclude `.git`, build output, and dependency caches by
  default. Report every included file before applying.
- Decode UTF-8 strictly, preserve each file's line-ending style, and reject
  binary or oversized inputs with a clear diagnostic. No match, an unexpected
  match count, overlapping plan rules, or a changed file since preview blocks
  application.
- Stage and validate all proposed contents before writing. Replace each file
  atomically, preserve permissions, and keep a recovery copy until the whole
  operation completes. Print a final per-file summary.
- Never call `sed`, Perl, or a user shell behind the scenes. The behavior must
  be the same on Windows, macOS, and Linux.

## Scope boundary

This is a guided source-editing command, not a general-purpose scripting
language or an IDE search-and-replace dialog. The initial version should target
UTF-8 text in a project directory and provide deterministic preview, explicit
application, and recoverable failure behavior. It does not rewrite syntax
semantics; compiler validation can be an optional post-apply check for Zelyra
source files.
