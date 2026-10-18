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

This section describes the durable scheduler introduced in 1.2.9. The normal
`zelyra setup --all` path creates or upgrades the MariaDB outbox for any
declared schedule. It is repeat-safe, stores no handler source or credentials,
and requires a project MariaDB URL. Setup removes old Zelyra-managed
`.zelyra/schedules.cron` files only after validating the outbox; user-managed
files and symlinks are protected.

Only the installed and active Linux systemd user service starts the resident
scheduler. It checks current project source at each minute boundary and inserts
each due occurrence under a deterministic ID based on project, schedule, and
UTC minute. Repeated ticks and restarts cannot insert a duplicate. A single
durable worker serializes occurrences, rechecks the current schedule before
running its handler, renews its database lease, and uses the configured
`scheduler_job_timeout_seconds` wall-clock limit. Removed, changed, or invalid
declarations are cancelled rather than run from stored handler text. Handler
failures use the outbox's bounded retry and exhausted states. Manually started
`serve` does not run schedules. Program output and worker errors go to the
service's stdout/stderr (typically the systemd journal).

Cron output is a reviewable preview only; it does not configure external
execution. Zelyra does not install or edit user crontabs and cannot remove old
rows in a crontab managed by someone else. The legacy trigger does not dispatch
work. The scheduler does not backfill minutes missed while the service was
stopped. Delivery is at least once: if a handler performs an external side
effect and the process fails before acknowledgement, a retry can repeat that
side effect. Use idempotent handlers or an application transaction for business
effects. `zelyra run` continues to read project settings from `.env`.

`zelyra schedule status main.zyl [--json]` reports compiler-checked schedule
declarations, the project service's installed/active state, and the resulting
scheduler state. It does not start or change anything. The version 1 JSON
output includes `platform_service_supported`, `declared_schedule_count`,
`service_installed`, `service_active`, `scheduler_active`, and schedule
metadata. On unsupported platforms the service is reported unavailable and
the scheduler inactive.

`zelyra schedule jobs main.zyl [--limit 50] [--offset 0] [--json]` displays a
bounded page of durable schedule metadata without returning job payloads. JSON
uses `schema_version: 1`. An exhausted occurrence can be released only through
the explicit `zelyra outbox requeue <event-id> main.zyl` operator action; this
does not execute the job immediately.

The `cron` property uses five numeric fields for minute, hour, day of month,
month, and day of week. Numbers, `*`, lists, ranges, and `/step` are supported;
month and weekday names are not supported yet. The field syntax and ranges are
compiler-checked. The resident scheduler uses standard cron field ranges; when
both day-of-month and day-of-week are restricted, either match is sufficient.
If no timezone is declared, UTC is used. The service scheduler resolves the
IANA timezone at startup and refuses to start for an unknown zone.

`schedule trigger` does not execute schedules. It rejects old exported cron
invocations and reports that the integrated scheduler owns declarations when
the project service is active. Schedules remain inactive without that service
as well. Do not put secrets in command lines or cron
output.
