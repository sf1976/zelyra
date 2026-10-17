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
schedules in `main.zyl`, create `.zelyra/schedules.cron` once, and ensure the MariaDB outbox
when a scheduled handler declares the `Email` capability. Zelyra refreshes its
own generated export when declarations change and refuses to overwrite an
unmanaged file. Scheduler-state or export symlinks are rejected. This bootstrap
does not activate a host cron job or execute schedules in `serve` yet. The
remaining 1.2.5 target is coordinated execution
through `serve`. The generated command contains no secret values; `zelyra run`
resolves project database and mail settings from the protected local `.env`.
Zelyra will not silently edit a user's crontab.

The `cron` property uses five numeric fields for minute, hour, day of month,
month, and day of week. Numbers, `*`, lists, ranges, and `/step` are supported;
month and weekday names are not supported yet. The field syntax and ranges are
compiler-checked. Actual trigger behavior belongs to the host scheduler.
If no timezone is declared, export uses UTC. The compiler checks the shape of
the timezone name, but does not verify that the target host has the entry.

`task run` starts Zelyra again with the declared handler. Normal project checks,
runtime budgets, and capability grants still apply. Do not put secrets in
command lines or cron output.
