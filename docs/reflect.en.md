# Export project metadata

`zelyra reflect main.zyl --format=json` checks the project through the regular
project loader and compiler path, then writes deterministic JSON. The top-level
`reflect_schema_version` versions this export independently of the CLI's
general diagnostics schema.

The current export contains:

- `source_files` and `modules`, including project-local imports and exports,
- `symbols` for functions, records, enums, aliases, views, and components,
- `tables` with column types and structural properties,
- `routes`, including generated form, CRUD, and authentication routes,
- `apis` with input/output types, error statuses, and permission metadata,
- `forms` with field types, validation rules, and action metadata,
- source locations relative to the project.

Example:

```sh
zelyra reflect main.zyl --format=json > project-reflection.json
```

The command does not execute project code, connect to a database, or read a
`.env` file. It omits UI text, SQL query text, defaults, database contents, and
runtime values. `completeness.live_database_inspected` is therefore `false`.

This is a static inventory, not a runtime or live-schema proof. Markdown and
OpenAPI exports are not implemented yet. Consumers should check
`reflect_schema_version` when the export shape changes.
