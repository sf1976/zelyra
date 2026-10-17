# Commands and schedules in Zelyra

Commands and schedules are declarations in Zelyra source. The compiler checks
command names, handlers, schedule references, and the supported numeric
five-field cron syntax.

```zelyra
command process_recurring_invoices {
    handler: run_recurring_invoices
    description: "Create invoices for active subscriptions"
}

schedule recurring_invoices {
    command: process_recurring_invoices
    cron: "0 4 * * 1-5"
    timezone: "Europe/Berlin"
}

fn run_recurring_invoices() {
    // Business logic; database and other effects remain capability-checked.
}

fn main() { }
```

A schedule may reference only a command with a zero-argument handler.
Commands can have handler arguments. `task run --args` passes them as a JSON
array; Zelyra checks values against the function parameters before invoking the
normal runtime.

```sh
zelyra check main.zyl
zelyra task list main.zyl
zelyra task list main.zyl --json
zelyra task run main.zyl process_recurring_invoices
zelyra schedule list main.zyl --json
zelyra schedule export main.zyl --format cron
zelyra schedule export main.zyl --format cron --output deploy/zelyra.cron
```

Without `--output`, export prints a cron preview to stdout. With `--output`, it
writes a new project-local file; it refuses to replace an existing file and
rejects symlinked directories that resolve outside the project. The target
directory must already exist. Neither form installs or enables a scheduler.
The preview uses `CRON_TZ`, but cron implementations differ in
timezone, daylight-saving, and environment-variable behavior; verify these on
the target host. systemd timers and platform schedulers are also possible
deployment hosts. Zelyra does not start a resident scheduler by default.

This describes the released 1.1.5 behavior. In the current unreleased 1.2.5
worktree, `zelyra setup --schema`, `--all`, and `zelyra serve` discover checked
schedules in `main.zyl`, create `.zelyra/schedules.cron`, and idempotently
ensure the MariaDB outbox for scheduled `Email` handlers. Changed declarations
refresh only Zelyra's own export; when no schedules remain, that managed export
is removed. Foreign files and symlinks are protected.
When schedules exist, `serve` also creates `.zelyra/serve.json` with its
loopback address and a random session token (directory mode `0700`, file mode
`0600` on Unix). Exported cron entries call `zelyra schedule trigger`. That
command makes an authenticated local request to verify that this exact project
is running under its installed, active Zelyra OS service. A manually started
`serve` process is not sufficient. The trigger skips the job when the service
or its server is down. After a valid probe, Zelyra starts the compiler-checked
command handler as a child process. A host cron must still be explicitly
installed; Zelyra never silently edits a user's crontab. Removing a schedule
declaration does not delete an already manually installed host crontab row;
the trigger revalidates the source and refuses to run that removed schedule.
A scheduler integrated directly into `serve` is the next scheduler milestone
and is intended to make external cron entries unnecessary. Duplicate-run
coordination is not yet guaranteed. Secrets are absent from the cron entry and
command line; `zelyra run` reads project settings from `.env`.

The `cron` property uses five numeric fields for minute, hour, day of month,
month, and day of week. Numbers, `*`, lists, ranges, and `/step` are supported;
month and weekday names are not supported yet. The field syntax and ranges are
compiler-checked. Actual trigger behavior belongs to the host scheduler.
If no timezone is declared, export uses UTC. The compiler checks the shape of
the timezone name, but does not verify that the target host has the entry.

`schedule trigger` accepts only a declared schedule name and does nothing when
the authenticated server probe fails. It then starts Zelyra again with the
declared handler, preserving normal project checks, runtime budgets, and
capability grants. Do not put secrets in command lines or cron output.
