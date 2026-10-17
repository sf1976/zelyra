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
worktree, `zelyra setup --schema`, `--all`, and `zelyra serve` check schedules
in `main.zyl` and idempotently ensure the MariaDB outbox for scheduled
`Email` handlers. Setup does not generate cron rows; it removes old
Zelyra-managed `.zelyra/schedules.cron` files. Foreign files and symlinks are
protected. The installed and active Linux systemd user service starts the
scheduler inside the same `serve` process. It reloads and checks project source
at every minute boundary, so removed, changed, or currently invalid declarations
are not executed from stale startup state. Manually started `serve` does not run
schedules. Schedules use their IANA timezone, defaulting to UTC. At most four
different schedule handlers run concurrently; a schedule is skipped while its
previous invocation is still active. Errors and program output go to the
service's stdout/stderr (typically the systemd journal).

The explicit cron preview is only for externally managed execution and must not
be activated alongside the integrated scheduler. Zelyra does not install or
edit user crontabs. It cannot remove old manually installed crontab rows from a
crontab managed by someone else; the legacy trigger skips inside an active
service to prevent duplicate execution. The scheduler does not catch up minutes
missed during downtime and does not yet provide durable distributed leases or
retries; those belong to the next job-system milestone. `zelyra run` continues
to read project settings from `.env`.

The `cron` property uses five numeric fields for minute, hour, day of month,
month, and day of week. Numbers, `*`, lists, ranges, and `/step` are supported;
month and weekday names are not supported yet. The field syntax and ranges are
compiler-checked. The resident scheduler uses standard cron field ranges; when
both day-of-month and day-of-week are restricted, either match is sufficient.
If no timezone is declared, UTC is used. The service scheduler resolves the
IANA timezone at startup and refuses to start for an unknown zone.

`schedule trigger` is a legacy guard for previously exported cron rows. When
the active project service is running, it skips execution because the resident
scheduler owns those declarations. Do not put secrets in command lines or cron
output.
