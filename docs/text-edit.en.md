# Plan safe text changes

`zelyra text edit` reads a TOML plan and previews it by default. It writes files
only after you explicitly confirm the same plan with its displayed SHA-256 ID.

```toml
[[change]]
name = "Show family name first"
files = ["src/**/*.zyl"]
find = '''(?P<first> [[:alpha:]]+ )\s+(?P<last> [[:alpha:]]+ )'''
replace = "${last}, ${first}"
expect = 3
```

From the project directory:

```sh
zelyra text edit names.zedit
zelyra text edit names.zedit --apply --plan-id sha256:<ID-from-preview>
```

The preview shows match counts per rule, affected files, and a unified diff. The
command stops if counts or input contents changed. Every rule must expect at
least one match. Rules cannot select the same file, which keeps the preview
unambiguous.

Plans and files must stay inside a Zelyra project. Patterns are relative and
support `*`, `?`, and `**`; `..`, absolute paths, and symbolic links are
rejected. `.git`, `target`, and `node_modules` are excluded. A plan can contain
up to 100 rules and 500 files; plans are limited to 256 KiB and each file to
2 MiB. Input is strict UTF-8. Rules use Rust's linear-time regex engine;
backreferences, look-around, and executable replacements are unsupported.

The preview does not check Zelyra semantics. Run `zelyra check` or `zelyra
build` after changing source code.
