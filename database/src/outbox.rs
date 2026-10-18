//! Transactional MariaDB event outbox primitives.
//!
//! The schema setup is an explicit operator action. Event insertion accepts a
//! [`MariaDbTransaction`] so callers can commit it atomically with business
//! data. An explicitly invoked one-event dispatcher can deliver to application
//! handlers. It does not start background workers or guarantee exactly-once
//! delivery; handlers must be idempotent.

use crate::{execute_mariadb_query, DatabaseError, MariaDbTransaction, Query, QueryValue};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};

/// Maximum encoded JSON payload accepted by the current outbox slice.
/// The larger bound accommodates base64 encoded PDF attachments while keeping
/// transactional mail events explicitly bounded.
pub const MAX_EVENT_PAYLOAD_BYTES: usize = 3 * 1024 * 1024;
/// Hard stop for delivery attempts in the current local outbox slice.
pub const MAX_DELIVERY_ATTEMPTS: u8 = 8;
/// Maximum number of metadata rows returned by one inspection query.
pub const MAX_OUTBOX_INSPECTION_LIMIT: usize = 200;
/// Offset cap prevents a local inspection request from scanning unbounded pages.
pub const MAX_OUTBOX_INSPECTION_OFFSET: usize = 1_000_000;
/// Stable event type used for native declared schedule occurrences.
pub const SCHEDULE_JOB_EVENT_TYPE: &str = "zelyra.schedule.command";
const MIN_LEASE_SECONDS: u64 = 5;
const MAX_LEASE_SECONDS: u64 = 3_600;
const RETRY_BASE_SECONDS: u64 = 5;
const RETRY_MAX_SECONDS: u64 = 3_600;

/// A validated, stable event envelope ready for transactional insertion.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutboxEvent {
    pub id: String,
    pub event_type: String,
    pub payload_json: String,
}

/// Event data claimed by one worker until its lease expires.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClaimedOutboxEvent {
    pub event: OutboxEvent,
    pub attempts: u8,
    pub lease_token: String,
}

/// Safe operator-visible metadata for one event. Payloads are intentionally
/// omitted because they may contain application data or secrets.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutboxEventSummary {
    pub id: String,
    pub event_type: String,
    pub attempts: u8,
    pub status: String,
    pub created_at: String,
    pub available_at: String,
    pub lease_until: Option<String>,
    pub last_error: Option<String>,
    pub delivered_at: Option<String>,
    pub exhausted_at: Option<String>,
    pub cancelled_at: Option<String>,
    /// Safe schedule metadata; absent for non-schedule events.
    pub schedule_name: Option<String>,
    /// UTC due-minute timestamp; absent for non-schedule events.
    pub scheduled_at: Option<String>,
    pub recipient: Option<String>,
    pub attachment_name: Option<String>,
}

/// One bounded page of metadata; payload bodies and attachment bytes are excluded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutboxInspectionPage {
    pub total: u64,
    pub offset: usize,
    pub events: Vec<OutboxEventSummary>,
}

/// An application handler error with no caller-provided text. The outbox
/// stores only a fixed redacted failure code.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct OutboxHandlerFailure;

/// Context supplied to a handler so it can renew its current live lease.
pub struct OutboxHandlerContext<'a> {
    database_url: &'a str,
    event_id: &'a str,
    lease_token: &'a str,
    lease_seconds: u64,
    cancelled: &'a AtomicBool,
}

impl OutboxHandlerContext<'_> {
    /// Extend this handler's live lease by the worker's configured duration.
    /// A false result means the lease expired or ownership was lost.
    pub fn renew_lease(&self) -> Result<bool, DatabaseError> {
        renew_mariadb_event_lease(
            self.database_url,
            self.event_id,
            self.lease_token,
            self.lease_seconds,
        )
    }

    /// Mark the claimed event as intentionally cancelled after the handler
    /// determines that its work is stale.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }
}

type OutboxHandler = dyn for<'a> Fn(&OutboxEvent, &OutboxHandlerContext<'a>) -> Result<(), OutboxHandlerFailure>
    + Send
    + Sync;

/// Application handlers keyed by stable event type.
#[derive(Default)]
pub struct OutboxHandlerRegistry {
    handlers: HashMap<String, Box<OutboxHandler>>,
}

impl OutboxHandlerRegistry {
    /// Register one handler for a lowercase dot-separated event type.
    /// Duplicate registrations are rejected instead of silently replacing a
    /// handler.
    pub fn register<F>(
        &mut self,
        event_type: impl Into<String>,
        handler: F,
    ) -> Result<(), DatabaseError>
    where
        F: for<'a> Fn(&OutboxEvent, &OutboxHandlerContext<'a>) -> Result<(), OutboxHandlerFailure>
            + Send
            + Sync
            + 'static,
    {
        let event_type = event_type.into();
        validate_event_type(&event_type)?;
        if self.handlers.contains_key(&event_type) {
            return Err(DatabaseError {
                message: "an outbox handler is already registered for this event type".into(),
            });
        }
        self.handlers.insert(event_type, Box::new(handler));
        Ok(())
    }
}

/// Result of explicitly processing at most one event from the MariaDB outbox.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OutboxWorkerResult {
    Idle,
    Delivered { event_id: String, attempts: u8 },
    RetryScheduled { event_id: String, attempts: u8 },
    Exhausted { event_id: String, attempts: u8 },
    Cancelled { event_id: String, attempts: u8 },
    LeaseLost { event_id: String, attempts: u8 },
}

/// Claim and dispatch at most one due event. This function never starts a
/// background thread or loop; callers explicitly decide when to invoke it.
/// `lease_token` must be unpredictable and unique for this claim attempt.
pub fn run_mariadb_outbox_once(
    database_url: &str,
    handlers: &OutboxHandlerRegistry,
    lease_token: &str,
    lease_seconds: u64,
) -> Result<OutboxWorkerResult, DatabaseError> {
    run_mariadb_outbox_once_scoped(database_url, handlers, lease_token, lease_seconds, None)
}

/// Claim and handle one event of the registered types and only for the given
/// project identity. Schedule services must use this to isolate shared DBs.
pub fn run_mariadb_outbox_once_for_project(
    database_url: &str,
    handlers: &OutboxHandlerRegistry,
    lease_token: &str,
    lease_seconds: u64,
    project_id: &str,
) -> Result<OutboxWorkerResult, DatabaseError> {
    validate_project_identity(project_id)?;
    run_mariadb_outbox_once_scoped(
        database_url,
        handlers,
        lease_token,
        lease_seconds,
        Some(project_id),
    )
}

fn run_mariadb_outbox_once_scoped(
    database_url: &str,
    handlers: &OutboxHandlerRegistry,
    lease_token: &str,
    lease_seconds: u64,
    project_id: Option<&str>,
) -> Result<OutboxWorkerResult, DatabaseError> {
    let mut event_types = handlers.handlers.keys().cloned().collect::<Vec<_>>();
    event_types.sort();
    let Some(claim) = claim_mariadb_event_for_types(
        database_url,
        lease_token,
        lease_seconds,
        Some(&event_types),
        project_id,
    )?
    else {
        return Ok(OutboxWorkerResult::Idle);
    };
    let event_id = claim.event.id.clone();
    let attempts = claim.attempts;
    let cancelled = AtomicBool::new(false);
    let context = OutboxHandlerContext {
        database_url,
        event_id: &event_id,
        lease_token: &claim.lease_token,
        lease_seconds,
        cancelled: &cancelled,
    };
    let handled = handlers
        .handlers
        .get(&claim.event.event_type)
        .is_some_and(|handler| handler(&claim.event, &context).is_ok());
    if handled {
        if cancelled.load(Ordering::Acquire) {
            if cancel_mariadb_event(database_url, &event_id, &claim.lease_token)? {
                return Ok(OutboxWorkerResult::Cancelled { event_id, attempts });
            }
            return Ok(OutboxWorkerResult::LeaseLost { event_id, attempts });
        }
        if acknowledge_mariadb_event(database_url, &event_id, &claim.lease_token)? {
            return Ok(OutboxWorkerResult::Delivered { event_id, attempts });
        }
        return Ok(OutboxWorkerResult::LeaseLost { event_id, attempts });
    }
    if !fail_mariadb_event(database_url, &event_id, &claim.lease_token)? {
        return Ok(OutboxWorkerResult::LeaseLost { event_id, attempts });
    }
    if attempts >= MAX_DELIVERY_ATTEMPTS {
        Ok(OutboxWorkerResult::Exhausted { event_id, attempts })
    } else {
        Ok(OutboxWorkerResult::RetryScheduled { event_id, attempts })
    }
}

impl OutboxEvent {
    /// Construct an event from a JSON value, enforcing stable identifier,
    /// event type, and encoded payload limits before any database work.
    pub fn new(
        id: impl Into<String>,
        event_type: impl Into<String>,
        payload: &serde_json::Value,
    ) -> Result<Self, DatabaseError> {
        let id = id.into();
        let event_type = event_type.into();
        validate_event_id(&id)?;
        validate_event_type(&event_type)?;
        let payload_json = serde_json::to_string(payload).map_err(|_| DatabaseError {
            message: "outbox event payload could not be encoded as JSON".into(),
        })?;
        if payload_json.len() > MAX_EVENT_PAYLOAD_BYTES {
            return Err(DatabaseError {
                message: format!(
                    "outbox event payload exceeds the {} byte limit",
                    MAX_EVENT_PAYLOAD_BYTES
                ),
            });
        }
        Ok(Self {
            id,
            event_type,
            payload_json,
        })
    }
}

/// Explicitly create the local MariaDB outbox table if it does not exist.
/// This DDL must be run during setup or migration, not implicitly per request.
pub fn ensure_mariadb_outbox(database_url: &str) -> Result<(), DatabaseError> {
    execute_mariadb_query(
        database_url,
        "CREATE TABLE IF NOT EXISTS `_zelyra_outbox` (\
            `event_id` VARCHAR(128) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,\
            `event_type` VARCHAR(128) CHARACTER SET ascii COLLATE ascii_bin NOT NULL,\
            `payload` LONGTEXT CHARACTER SET utf8mb4 NOT NULL,\
            `created_at` DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),\
            `attempts` SMALLINT UNSIGNED NOT NULL DEFAULT 0,\
            `available_at` DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),\
            `lease_token` VARCHAR(128) CHARACTER SET ascii COLLATE ascii_bin NULL,\
            `lease_until` DATETIME(6) NULL,\
            `last_error` VARCHAR(512) NULL,\
            `delivered_at` DATETIME(6) NULL,\
            `exhausted_at` DATETIME(6) NULL,\
            `cancelled_at` DATETIME(6) NULL,\
            PRIMARY KEY (`event_id`),\
            KEY `idx_zelyra_outbox_pending` (`delivered_at`, `available_at`, `created_at`),\
            CONSTRAINT `chk_zelyra_outbox_payload_json` CHECK (JSON_VALID(`payload`))\
        ) ENGINE=InnoDB",
        Vec::new(),
    )?;
    let column = execute_mariadb_query(
        database_url,
        "SELECT COUNT(*) AS `count` FROM information_schema.COLUMNS \
         WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = '_zelyra_outbox' \
           AND COLUMN_NAME = 'exhausted_at'",
        Vec::new(),
    )?;
    if column
        .rows
        .first()
        .and_then(|row| row.first())
        .map(String::as_str)
        == Some("0")
    {
        execute_mariadb_query(
            database_url,
            "ALTER TABLE `_zelyra_outbox` ADD COLUMN `exhausted_at` DATETIME(6) NULL AFTER `delivered_at`",
            Vec::new(),
        )?;
    }
    let column = execute_mariadb_query(
        database_url,
        "SELECT COUNT(*) AS `count` FROM information_schema.COLUMNS \
         WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = '_zelyra_outbox' \
           AND COLUMN_NAME = 'cancelled_at'",
        Vec::new(),
    )?;
    if column
        .rows
        .first()
        .and_then(|row| row.first())
        .map(String::as_str)
        == Some("0")
    {
        execute_mariadb_query(
            database_url,
            "ALTER TABLE `_zelyra_outbox` ADD COLUMN `cancelled_at` DATETIME(6) NULL AFTER `exhausted_at`",
            Vec::new(),
        )?;
    }
    Ok(())
}

/// Insert an event on the caller's existing MariaDB transaction. If the
/// business transaction rolls back, the event row rolls back with it.
pub fn enqueue_mariadb_event(
    transaction: &mut MariaDbTransaction<'_>,
    event: &OutboxEvent,
) -> Result<(), DatabaseError> {
    validate_event_id(&event.id)?;
    validate_event_type(&event.event_type)?;
    if event.payload_json.len() > MAX_EVENT_PAYLOAD_BYTES
        || zelyra_ast::json::parse(&event.payload_json).is_err()
    {
        return Err(DatabaseError {
            message: format!(
                "outbox event payload must be valid JSON within the {} byte limit",
                MAX_EVENT_PAYLOAD_BYTES
            ),
        });
    }
    transaction.execute(&Query {
        sql: "INSERT INTO `_zelyra_outbox` (`event_id`, `event_type`, `payload`, `created_at`, `available_at`) VALUES (:event_id, :event_type, :payload, UTC_TIMESTAMP(6), UTC_TIMESTAMP(6))".into(),
        params: vec![
            ("event_id".into(), QueryValue::String(event.id.clone())),
            ("event_type".into(), QueryValue::String(event.event_type.clone())),
            ("payload".into(), QueryValue::String(event.payload_json.clone())),
        ],
    })?;
    Ok(())
}

/// Idempotently insert an event under a deterministic event ID. Reusing an ID
/// with a different type or payload is rejected so a collision or caller bug
/// cannot silently change the meaning of queued work.
pub fn enqueue_mariadb_event_once(
    database_url: &str,
    event: &OutboxEvent,
) -> Result<(), DatabaseError> {
    validate_event_id(&event.id)?;
    validate_event_type(&event.event_type)?;
    if event.payload_json.len() > MAX_EVENT_PAYLOAD_BYTES
        || zelyra_ast::json::parse(&event.payload_json).is_err()
    {
        return Err(DatabaseError {
            message: format!(
                "outbox event payload must be valid JSON within the {} byte limit",
                MAX_EVENT_PAYLOAD_BYTES
            ),
        });
    }
    crate::with_mariadb_transaction(database_url, |transaction| {
        transaction.execute(&Query {
            sql: "INSERT INTO `_zelyra_outbox` (`event_id`, `event_type`, `payload`, `created_at`, `available_at`) \
                 VALUES (:event_id, :event_type, :payload, UTC_TIMESTAMP(6), UTC_TIMESTAMP(6)) \
                 ON DUPLICATE KEY UPDATE `event_id` = `event_id`"
                .into(),
            params: vec![
                ("event_id".into(), QueryValue::String(event.id.clone())),
                ("event_type".into(), QueryValue::String(event.event_type.clone())),
                ("payload".into(), QueryValue::String(event.payload_json.clone())),
            ],
        })?;
        let existing = transaction.execute(&Query {
            sql: "SELECT `event_type`, `payload` FROM `_zelyra_outbox` WHERE `event_id` = :event_id FOR UPDATE".into(),
            params: vec![("event_id".into(), QueryValue::String(event.id.clone()))],
        })?;
        let Some(row) = existing.rows.first() else {
            return Err(DatabaseError {
                message: "database did not retain the deterministic outbox event".into(),
            });
        };
        if row.len() != 2 || row[0] != event.event_type || row[1] != event.payload_json {
            return Err(DatabaseError {
                message: "deterministic outbox event ID is already bound to different event data"
                    .into(),
            });
        }
        Ok(())
    })
}

/// Atomically claim the oldest due event whose lease is absent or expired.
/// `lease_token` must be unique per claim attempt and must not be reused while
/// any previous claim with that token could still be active. Callers should
/// use an unpredictable token so a stale or unrelated worker cannot settle a
/// claim it does not own.
pub fn claim_mariadb_event(
    database_url: &str,
    lease_token: &str,
    lease_seconds: u64,
) -> Result<Option<ClaimedOutboxEvent>, DatabaseError> {
    claim_mariadb_event_for_types(database_url, lease_token, lease_seconds, None, None)
}

fn claim_mariadb_event_for_types(
    database_url: &str,
    lease_token: &str,
    lease_seconds: u64,
    event_types: Option<&[String]>,
    project_id: Option<&str>,
) -> Result<Option<ClaimedOutboxEvent>, DatabaseError> {
    validate_lease_token(lease_token)?;
    validate_lease_duration(lease_seconds)?;
    if event_types.is_some_and(|types| types.is_empty()) {
        return Ok(None);
    }
    if let Some(types) = event_types {
        for event_type in types {
            validate_event_type(event_type)?;
        }
    }
    if let Some(project_id) = project_id {
        validate_project_identity(project_id)?;
    }
    let event_type_filter = event_types.map_or_else(String::new, |types| {
        let placeholders = types
            .iter()
            .enumerate()
            .map(|(index, _)| format!(":event_type_{index}"))
            .collect::<Vec<_>>()
            .join(", ");
        format!(" AND `event_type` IN ({placeholders})")
    });
    let mut event_type_filter = event_type_filter;
    let mut event_type_params = event_types.map_or_else(Vec::new, |types| {
        types
            .iter()
            .enumerate()
            .map(|(index, event_type)| {
                (
                    format!("event_type_{index}"),
                    QueryValue::String(event_type.clone()),
                )
            })
            .collect::<Vec<_>>()
    });
    if let Some(project_id) = project_id {
        event_type_filter
            .push_str(" AND JSON_UNQUOTE(JSON_EXTRACT(`payload`, '$.project_id')) = :project_id");
        event_type_params.push(("project_id".into(), QueryValue::String(project_id.into())));
    }

    crate::with_mariadb_transaction(database_url, |transaction| {
        let terminal = transaction.execute(&Query {
            sql: format!(
                "SELECT `event_id` FROM `_zelyra_outbox` WHERE `delivered_at` IS NULL \
                   AND `exhausted_at` IS NULL AND `cancelled_at` IS NULL \
                   AND `attempts` >= {MAX_DELIVERY_ATTEMPTS} \
                   AND (`lease_until` IS NULL OR `lease_until` <= UTC_TIMESTAMP(6)) \
                 {event_type_filter} ORDER BY `available_at`, `created_at`, `event_id` LIMIT 1 FOR UPDATE SKIP LOCKED"
            ),
            params: event_type_params.clone(),
        })?;
        if let Some(row) = terminal.rows.first() {
            if let Some(event_id) = row.first() {
                transaction.execute(&Query {
                    sql: "UPDATE `_zelyra_outbox` SET `exhausted_at` = UTC_TIMESTAMP(6), \
                          `last_error` = 'attempt_limit' WHERE `event_id` = :event_id \
                          AND `delivered_at` IS NULL AND `exhausted_at` IS NULL AND `cancelled_at` IS NULL"
                        .into(),
                    params: vec![("event_id".into(), QueryValue::String(event_id.clone()))],
                })?;
            }
        }
        let result = transaction.execute(&Query {
            sql: format!(
                "SELECT `event_id`, `event_type`, `payload`, `attempts` FROM `_zelyra_outbox` \
                 WHERE `delivered_at` IS NULL AND `exhausted_at` IS NULL AND `cancelled_at` IS NULL \
                   AND `attempts` < {MAX_DELIVERY_ATTEMPTS} \
                   AND `available_at` <= UTC_TIMESTAMP(6) \
                   AND (`lease_until` IS NULL OR `lease_until` <= UTC_TIMESTAMP(6)) \
                 {event_type_filter} ORDER BY `available_at`, `created_at`, `event_id` LIMIT 1 FOR UPDATE SKIP LOCKED"
            ),
            params: event_type_params,
        })?;
        let Some(row) = result.rows.first() else {
            return Ok(None);
        };
        if row.len() != 4 {
            return Err(DatabaseError {
                message: "database returned an invalid outbox claim row".into(),
            });
        }
        let event = OutboxEvent {
            id: row[0].clone(),
            event_type: row[1].clone(),
            payload_json: row[2].clone(),
        };
        validate_event_id(&event.id)?;
        validate_event_type(&event.event_type)?;
        if event.payload_json.len() > MAX_EVENT_PAYLOAD_BYTES
            || zelyra_ast::json::parse(&event.payload_json).is_err()
        {
            return Err(DatabaseError {
                message: "stored outbox event payload is invalid".into(),
            });
        }
        let attempts = row[3]
            .parse::<u8>()
            .ok()
            .filter(|attempts| *attempts < MAX_DELIVERY_ATTEMPTS)
            .ok_or_else(|| DatabaseError {
                message: "database returned an invalid outbox attempt count".into(),
            })?
            + 1;
        transaction.execute(&Query {
            sql: "UPDATE `_zelyra_outbox` SET `lease_token` = :lease_token, \
                  `lease_until` = DATE_ADD(UTC_TIMESTAMP(6), INTERVAL :lease_seconds SECOND), \
                  `attempts` = :attempts, `last_error` = NULL \
                  WHERE `event_id` = :event_id AND `delivered_at` IS NULL AND `cancelled_at` IS NULL"
                .into(),
            params: vec![
                ("lease_token".into(), QueryValue::String(lease_token.into())),
                ("lease_seconds".into(), QueryValue::UInt(lease_seconds)),
                ("attempts".into(), QueryValue::UInt(u64::from(attempts))),
                ("event_id".into(), QueryValue::String(event.id.clone())),
            ],
        })?;
        Ok(Some(ClaimedOutboxEvent {
            event,
            attempts,
            lease_token: lease_token.into(),
        }))
    })
}

/// Acknowledge delivery only while the caller still owns a live lease.
/// Returns `false` for a missing event, expired lease, or stale token.
pub fn acknowledge_mariadb_event(
    database_url: &str,
    event_id: &str,
    lease_token: &str,
) -> Result<bool, DatabaseError> {
    validate_event_id(event_id)?;
    validate_lease_token(lease_token)?;
    crate::with_mariadb_transaction(database_url, |transaction| {
        if !owns_live_lease(transaction, event_id, lease_token)? {
            return Ok(false);
        }
        transaction.execute(&Query {
            sql: "UPDATE `_zelyra_outbox` SET `delivered_at` = UTC_TIMESTAMP(6), \
                  `lease_token` = NULL, `lease_until` = NULL, `last_error` = NULL \
                  WHERE `event_id` = :event_id AND `lease_token` = :lease_token \
                    AND `lease_until` > UTC_TIMESTAMP(6) AND `cancelled_at` IS NULL"
                .into(),
            params: vec![
                ("event_id".into(), QueryValue::String(event_id.into())),
                ("lease_token".into(), QueryValue::String(lease_token.into())),
            ],
        })?;
        let result = transaction.execute(&Query {
            sql: "SELECT COUNT(*) AS `count` FROM `_zelyra_outbox` WHERE `event_id` = :event_id \
                 AND `delivered_at` IS NOT NULL AND `lease_token` IS NULL"
                .into(),
            params: vec![("event_id".into(), QueryValue::String(event_id.into()))],
        })?;
        Ok(result
            .rows
            .first()
            .and_then(|row| row.first())
            .map(String::as_str)
            == Some("1"))
    })
}

/// Extend a live event lease owned by `lease_token`.
///
/// Lease renewal does not change the delivery attempt count. It returns
/// `false` when the event is missing, the lease has expired, or another
/// worker owns the event. Callers must still tolerate duplicate delivery if
/// the process fails after the handler runs and before acknowledgement.
pub fn renew_mariadb_event_lease(
    database_url: &str,
    event_id: &str,
    lease_token: &str,
    lease_seconds: u64,
) -> Result<bool, DatabaseError> {
    validate_event_id(event_id)?;
    validate_lease_token(lease_token)?;
    validate_lease_duration(lease_seconds)?;
    crate::with_mariadb_transaction(database_url, |transaction| {
        if !owns_live_lease(transaction, event_id, lease_token)? {
            return Ok(false);
        }
        transaction.execute(&Query {
            sql: "UPDATE `_zelyra_outbox` SET \
                  `lease_until` = GREATEST(`lease_until`, DATE_ADD(UTC_TIMESTAMP(6), INTERVAL :lease_seconds SECOND)) \
                  WHERE `event_id` = :event_id AND `lease_token` = :lease_token \
                    AND `lease_until` > UTC_TIMESTAMP(6) AND `delivered_at` IS NULL \
                    AND `exhausted_at` IS NULL AND `cancelled_at` IS NULL"
                .into(),
            params: vec![
                ("lease_seconds".into(), QueryValue::UInt(lease_seconds)),
                ("event_id".into(), QueryValue::String(event_id.into())),
                ("lease_token".into(), QueryValue::String(lease_token.into())),
            ],
        })?;
        let result = transaction.execute(&Query {
            sql: "SELECT COUNT(*) AS `count` FROM `_zelyra_outbox` WHERE `event_id` = :event_id \
                 AND `lease_token` = :lease_token AND `lease_until` > UTC_TIMESTAMP(6) \
                 AND `delivered_at` IS NULL AND `exhausted_at` IS NULL AND `cancelled_at` IS NULL"
                .into(),
            params: vec![
                ("event_id".into(), QueryValue::String(event_id.into())),
                ("lease_token".into(), QueryValue::String(lease_token.into())),
            ],
        })?;
        Ok(result
            .rows
            .first()
            .and_then(|row| row.first())
            .map(String::as_str)
            == Some("1"))
    })
}

/// Record a failed delivery using a fixed, redacted status code and bounded
/// exponential retry. After eight claims the event becomes exhausted and is
/// retained for operator inspection.
pub fn fail_mariadb_event(
    database_url: &str,
    event_id: &str,
    lease_token: &str,
) -> Result<bool, DatabaseError> {
    validate_event_id(event_id)?;
    validate_lease_token(lease_token)?;
    crate::with_mariadb_transaction(database_url, |transaction| {
        let attempts = live_lease_attempts(transaction, event_id, lease_token)?;
        let Some(attempts) = attempts else {
            return Ok(false);
        };
        if attempts >= MAX_DELIVERY_ATTEMPTS {
            transaction.execute(&Query {
                sql: "UPDATE `_zelyra_outbox` SET `exhausted_at` = UTC_TIMESTAMP(6), \
                      `lease_token` = NULL, `lease_until` = NULL, `last_error` = 'attempt_limit' \
                      WHERE `event_id` = :event_id AND `lease_token` = :lease_token \
                        AND `lease_until` > UTC_TIMESTAMP(6) AND `cancelled_at` IS NULL"
                    .into(),
                params: vec![
                    ("event_id".into(), QueryValue::String(event_id.into())),
                    ("lease_token".into(), QueryValue::String(lease_token.into())),
                ],
            })?;
        } else {
            let exponent = u32::from(attempts.saturating_sub(1));
            let retry_seconds = RETRY_BASE_SECONDS
                .saturating_mul(1_u64.checked_shl(exponent).unwrap_or(u64::MAX))
                .min(RETRY_MAX_SECONDS);
            transaction.execute(&Query {
                sql: "UPDATE `_zelyra_outbox` SET \
                      `available_at` = DATE_ADD(UTC_TIMESTAMP(6), INTERVAL :retry_seconds SECOND), \
                      `lease_token` = NULL, `lease_until` = NULL, `last_error` = 'handler_failure' \
                      WHERE `event_id` = :event_id AND `lease_token` = :lease_token \
                        AND `lease_until` > UTC_TIMESTAMP(6) AND `cancelled_at` IS NULL"
                    .into(),
                params: vec![
                    ("retry_seconds".into(), QueryValue::UInt(retry_seconds)),
                    ("event_id".into(), QueryValue::String(event_id.into())),
                    ("lease_token".into(), QueryValue::String(lease_token.into())),
                ],
            })?;
        }
        let result = transaction.execute(&Query {
            sql: "SELECT COUNT(*) AS `count` FROM `_zelyra_outbox` WHERE `event_id` = :event_id \
                 AND `lease_token` IS NULL AND `delivered_at` IS NULL AND `cancelled_at` IS NULL"
                .into(),
            params: vec![("event_id".into(), QueryValue::String(event_id.into()))],
        })?;
        Ok(result
            .rows
            .first()
            .and_then(|row| row.first())
            .map(String::as_str)
            == Some("1"))
    })
}

/// Cancel a currently leased event only while the caller still owns its live
/// lease. Cancelled work is retained for audit and never becomes claimable.
pub fn cancel_mariadb_event(
    database_url: &str,
    event_id: &str,
    lease_token: &str,
) -> Result<bool, DatabaseError> {
    validate_event_id(event_id)?;
    validate_lease_token(lease_token)?;
    crate::with_mariadb_transaction(database_url, |transaction| {
        if !owns_live_lease(transaction, event_id, lease_token)? {
            return Ok(false);
        }
        transaction.execute(&Query {
            sql: "UPDATE `_zelyra_outbox` SET `cancelled_at` = UTC_TIMESTAMP(6), \
                  `lease_token` = NULL, `lease_until` = NULL, `last_error` = NULL \
                  WHERE `event_id` = :event_id AND `lease_token` = :lease_token \
                    AND `lease_until` > UTC_TIMESTAMP(6) AND `delivered_at` IS NULL \
                    AND `exhausted_at` IS NULL AND `cancelled_at` IS NULL"
                .into(),
            params: vec![
                ("event_id".into(), QueryValue::String(event_id.into())),
                ("lease_token".into(), QueryValue::String(lease_token.into())),
            ],
        })?;
        let result = transaction.execute(&Query {
            sql: "SELECT COUNT(*) AS `count` FROM `_zelyra_outbox` WHERE `event_id` = :event_id \
                 AND `cancelled_at` IS NOT NULL AND `lease_token` IS NULL"
                .into(),
            params: vec![("event_id".into(), QueryValue::String(event_id.into()))],
        })?;
        Ok(result
            .rows
            .first()
            .and_then(|row| row.first())
            .map(String::as_str)
            == Some("1"))
    })
}

/// Cancel pending, currently unleased schedule occurrences for one canonical
/// project identity. Leased work is left to the worker's source revalidation.
pub fn cancel_pending_mariadb_schedule_jobs(
    database_url: &str,
    project_id: &str,
) -> Result<(), DatabaseError> {
    validate_project_identity(project_id)?;
    let table = execute_mariadb_query(
        database_url,
        "SELECT COUNT(*) AS `count` FROM information_schema.TABLES \
         WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = '_zelyra_outbox'",
        Vec::new(),
    )?;
    if table
        .rows
        .first()
        .and_then(|row| row.first())
        .map(String::as_str)
        != Some("1")
    {
        // Removing schedule declarations from a project that never used the
        // outbox must not create an internal table outside its source schema.
        return Ok(());
    }
    // Upgrade an existing outbox before using the cancellation state. This is
    // deliberately after the existence check so setup for schedule-free
    // projects does not create a table the schema planner will later remove.
    ensure_mariadb_outbox(database_url)?;
    execute_mariadb_query(
        database_url,
        "UPDATE `_zelyra_outbox` SET `cancelled_at` = UTC_TIMESTAMP(6), `last_error` = NULL \
         WHERE `event_type` = :event_type AND `delivered_at` IS NULL \
           AND `exhausted_at` IS NULL AND `cancelled_at` IS NULL \
           AND `lease_token` IS NULL \
           AND JSON_UNQUOTE(JSON_EXTRACT(`payload`, '$.project_id')) = :project_id",
        vec![
            (
                "event_type".into(),
                QueryValue::String(SCHEDULE_JOB_EVENT_TYPE.into()),
            ),
            ("project_id".into(), QueryValue::String(project_id.into())),
        ],
    )?;
    Ok(())
}

/// Explicitly release an exhausted event for operator-approved recovery. Its
/// attempt counter is reset so the bounded retry policy applies again.
pub fn requeue_exhausted_mariadb_event(
    database_url: &str,
    event_id: &str,
) -> Result<bool, DatabaseError> {
    validate_event_id(event_id)?;
    crate::with_mariadb_transaction(database_url, |transaction| {
        let result = transaction.execute(&Query {
            sql: "SELECT `event_id` FROM `_zelyra_outbox` WHERE `event_id` = :event_id \
                 AND `exhausted_at` IS NOT NULL AND `delivered_at` IS NULL FOR UPDATE"
                .into(),
            params: vec![("event_id".into(), QueryValue::String(event_id.into()))],
        })?;
        if result.rows.is_empty() {
            return Ok(false);
        }
        transaction.execute(&Query {
            sql: "UPDATE `_zelyra_outbox` SET `exhausted_at` = NULL, `attempts` = 0, \
                  `available_at` = UTC_TIMESTAMP(6), `last_error` = NULL \
                  WHERE `event_id` = :event_id AND `exhausted_at` IS NOT NULL"
                .into(),
            params: vec![("event_id".into(), QueryValue::String(event_id.into()))],
        })?;
        Ok(true)
    })
}

/// List bounded operator-visible event metadata without returning payloads.
pub fn inspect_mariadb_outbox(
    database_url: &str,
    limit: usize,
) -> Result<Vec<OutboxEventSummary>, DatabaseError> {
    inspect_mariadb_outbox_page(database_url, limit, 0).map(|page| page.events)
}

/// Read a bounded, ordered metadata page. Count and rows are separate queries,
/// so a concurrent worker may change the count between them.
pub fn inspect_mariadb_outbox_page(
    database_url: &str,
    limit: usize,
    offset: usize,
) -> Result<OutboxInspectionPage, DatabaseError> {
    inspect_mariadb_outbox_page_filtered(database_url, limit, offset, None, None)
}

/// Inspect only durable native schedule jobs without exposing stored payloads.
pub fn inspect_mariadb_schedule_jobs_page(
    database_url: &str,
    project_id: &str,
    limit: usize,
    offset: usize,
) -> Result<OutboxInspectionPage, DatabaseError> {
    validate_project_identity(project_id)?;
    inspect_mariadb_outbox_page_filtered(
        database_url,
        limit,
        offset,
        Some(SCHEDULE_JOB_EVENT_TYPE),
        Some(project_id),
    )
}

fn inspect_mariadb_outbox_page_filtered(
    database_url: &str,
    limit: usize,
    offset: usize,
    event_type: Option<&str>,
    project_id: Option<&str>,
) -> Result<OutboxInspectionPage, DatabaseError> {
    if !(1..=MAX_OUTBOX_INSPECTION_LIMIT).contains(&limit) {
        return Err(DatabaseError {
            message: format!(
                "outbox inspection limit must be between 1 and {MAX_OUTBOX_INSPECTION_LIMIT}"
            ),
        });
    }
    if offset > MAX_OUTBOX_INSPECTION_OFFSET {
        return Err(DatabaseError {
            message: format!("outbox inspection offset exceeds {MAX_OUTBOX_INSPECTION_OFFSET}"),
        });
    }
    let where_clause = match (event_type.is_some(), project_id.is_some()) {
        (true, true) => " WHERE `event_type` = :event_type AND JSON_UNQUOTE(JSON_EXTRACT(`payload`, '$.project_id')) = :project_id",
        (true, false) => " WHERE `event_type` = :event_type",
        (false, true) => " WHERE JSON_UNQUOTE(JSON_EXTRACT(`payload`, '$.project_id')) = :project_id",
        (false, false) => "",
    };
    let mut filter_params = event_type
        .map(|event_type| vec![("event_type".into(), QueryValue::String(event_type.into()))])
        .unwrap_or_default();
    if let Some(project_id) = project_id {
        filter_params.push(("project_id".into(), QueryValue::String(project_id.into())));
    }
    let count = execute_mariadb_query(
        database_url,
        &format!("SELECT COUNT(*) AS `count` FROM `_zelyra_outbox`{where_clause}"),
        filter_params.clone(),
    )?;
    let total = count
        .rows
        .first()
        .and_then(|row| row.first())
        .and_then(|cell| cell.parse::<u64>().ok())
        .ok_or_else(|| DatabaseError {
            message: "database returned an invalid outbox count".into(),
        })?;
    let rows_sql = format!(
        "SELECT `event_id`, `event_type`, `attempts`, \
            CASE WHEN `cancelled_at` IS NOT NULL THEN 'cancelled' \
                 WHEN `delivered_at` IS NOT NULL THEN 'delivered' \
                 WHEN `exhausted_at` IS NOT NULL THEN 'exhausted' \
                 WHEN `lease_until` > UTC_TIMESTAMP(6) THEN 'leased' \
                 WHEN `available_at` > UTC_TIMESTAMP(6) THEN 'retry_wait' \
                 ELSE 'pending' END AS `status`, \
            DATE_FORMAT(`created_at`, '%Y-%m-%d %H:%i:%s.%f') AS `created_at`, \
            DATE_FORMAT(`available_at`, '%Y-%m-%d %H:%i:%s.%f') AS `available_at`, \
            COALESCE(DATE_FORMAT(`lease_until`, '%Y-%m-%d %H:%i:%s.%f'), '') AS `lease_until`, \
            COALESCE(`last_error`, '') AS `last_error`, \
            COALESCE(DATE_FORMAT(`delivered_at`, '%Y-%m-%d %H:%i:%s.%f'), '') AS `delivered_at`, \
            COALESCE(DATE_FORMAT(`exhausted_at`, '%Y-%m-%d %H:%i:%s.%f'), '') AS `exhausted_at`, \
            COALESCE(DATE_FORMAT(`cancelled_at`, '%Y-%m-%d %H:%i:%s.%f'), '') AS `cancelled_at`, \
            CASE WHEN `event_type` = 'zelyra.schedule.command' THEN \
                LEFT(COALESCE(JSON_UNQUOTE(JSON_EXTRACT(`payload`, '$.schedule_name')), ''), 128) \
                ELSE '' END AS `schedule_name`, \
            CASE WHEN `event_type` = 'zelyra.schedule.command' THEN \
                LEFT(COALESCE(JSON_UNQUOTE(JSON_EXTRACT(`payload`, '$.scheduled_at')), ''), 32) \
                ELSE '' END AS `scheduled_at`, \
            CASE WHEN `event_type` = 'zelyra.mail.send' THEN \
                LEFT(COALESCE(JSON_UNQUOTE(JSON_EXTRACT(`payload`, '$.to')), ''), 254) \
                ELSE '' END AS `recipient`, \
            CASE WHEN `event_type` = 'zelyra.mail.send' THEN \
                LEFT(COALESCE(JSON_UNQUOTE(JSON_EXTRACT(`payload`, '$.attachments[0].filename')), ''), 255) \
                ELSE '' END AS `attachment_name` \
         FROM `_zelyra_outbox`{where_clause} ORDER BY `created_at` DESC, `event_id` DESC \
         LIMIT :limit OFFSET :offset"
    );
    let result = execute_mariadb_query(database_url, &rows_sql, {
        let mut params = filter_params;
        params.extend([
            ("limit".into(), QueryValue::UInt(limit as u64)),
            ("offset".into(), QueryValue::UInt(offset as u64)),
        ]);
        params
    })?;
    let events = result
        .rows
        .into_iter()
        .map(|row| {
            if row.len() != 15 {
                return Err(DatabaseError {
                    message: "database returned an invalid outbox inspection row".into(),
                });
            }
            validate_event_id(&row[0])?;
            validate_event_type(&row[1])?;
            let attempts = row[2]
                .parse::<u8>()
                .ok()
                .filter(|attempts| *attempts <= MAX_DELIVERY_ATTEMPTS)
                .ok_or_else(|| DatabaseError {
                    message: "database returned an invalid outbox attempt count".into(),
                })?;
            Ok(OutboxEventSummary {
                id: row[0].clone(),
                event_type: row[1].clone(),
                attempts,
                status: row[3].clone(),
                created_at: row[4].clone(),
                available_at: row[5].clone(),
                lease_until: (!row[6].is_empty()).then(|| row[6].clone()),
                last_error: safe_last_error(&row[7]),
                delivered_at: (!row[8].is_empty()).then(|| row[8].clone()),
                exhausted_at: (!row[9].is_empty()).then(|| row[9].clone()),
                cancelled_at: (!row[10].is_empty()).then(|| row[10].clone()),
                schedule_name: safe_schedule_name(&row[11]),
                scheduled_at: safe_schedule_time(&row[12]),
                recipient: (!row[13].is_empty()).then(|| row[13].clone()),
                attachment_name: (!row[14].is_empty()).then(|| row[14].clone()),
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(OutboxInspectionPage {
        total,
        offset,
        events,
    })
}

fn validate_project_identity(project_id: &str) -> Result<(), DatabaseError> {
    if project_id.len() != 64
        || !project_id
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(DatabaseError {
            message: "schedule project identity must be a lowercase SHA-256 digest".into(),
        });
    }
    Ok(())
}

fn safe_schedule_name(value: &str) -> Option<String> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
    {
        return None;
    }
    Some(value.to_owned())
}

fn safe_schedule_time(value: &str) -> Option<String> {
    let bytes = value.as_bytes();
    if bytes.len() != 20
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes[10] != b'T'
        || bytes[13] != b':'
        || bytes[16..] != *b":00Z"
        || ![0..4, 5..7, 8..10, 11..13, 14..16]
            .into_iter()
            .all(|range| bytes[range].iter().all(u8::is_ascii_digit))
    {
        return None;
    }
    let number = |range: std::ops::Range<usize>| {
        std::str::from_utf8(&bytes[range]).ok()?.parse::<u32>().ok()
    };
    if !(1..=12).contains(&number(5..7)?)
        || !(1..=31).contains(&number(8..10)?)
        || number(11..13)? > 23
        || number(14..16)? > 59
    {
        return None;
    }
    Some(value.to_owned())
}

fn owns_live_lease(
    transaction: &mut crate::MariaDbTransaction<'_>,
    event_id: &str,
    lease_token: &str,
) -> Result<bool, DatabaseError> {
    Ok(live_lease_attempts(transaction, event_id, lease_token)?.is_some())
}

fn live_lease_attempts(
    transaction: &mut crate::MariaDbTransaction<'_>,
    event_id: &str,
    lease_token: &str,
) -> Result<Option<u8>, DatabaseError> {
    let result = transaction.execute(&Query {
        sql: "SELECT `attempts` FROM `_zelyra_outbox` WHERE `event_id` = :event_id \
             AND `lease_token` = :lease_token AND `lease_until` > UTC_TIMESTAMP(6) \
             AND `delivered_at` IS NULL AND `exhausted_at` IS NULL \
             AND `cancelled_at` IS NULL FOR UPDATE"
            .into(),
        params: vec![
            ("event_id".into(), QueryValue::String(event_id.into())),
            ("lease_token".into(), QueryValue::String(lease_token.into())),
        ],
    })?;
    result
        .rows
        .first()
        .and_then(|row| row.first())
        .map(|attempts| {
            attempts.parse::<u8>().map_err(|_| DatabaseError {
                message: "database returned an invalid outbox attempt count".into(),
            })
        })
        .transpose()
}

fn validate_event_id(id: &str) -> Result<(), DatabaseError> {
    if id.is_empty()
        || id.len() > 128
        || !id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
    {
        return Err(DatabaseError {
            message: "outbox event id must contain 1 to 128 ASCII letters, digits, `-`, or `_`"
                .into(),
        });
    }
    Ok(())
}

fn safe_last_error(value: &str) -> Option<String> {
    match value {
        "" => None,
        "handler_failure" => Some("handler_failure".into()),
        "attempt_limit" => Some("attempt_limit".into()),
        _ => Some("unknown".into()),
    }
}

fn validate_event_type(event_type: &str) -> Result<(), DatabaseError> {
    let mut parts = event_type.split('.');
    let valid_segment = |segment: &str| {
        !segment.is_empty()
            && segment.len() <= 64
            && segment.bytes().all(|byte| {
                byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_' || byte == b'-'
            })
    };
    if event_type.len() > 128 || !parts.all(valid_segment) {
        return Err(DatabaseError {
            message:
                "outbox event type must be lowercase dot-separated ASCII names (up to 128 bytes)"
                    .into(),
        });
    }
    Ok(())
}

fn validate_lease_token(token: &str) -> Result<(), DatabaseError> {
    validate_event_id(token).map_err(|_| DatabaseError {
        message: "outbox worker lease token must be a valid event id".into(),
    })
}

fn validate_lease_duration(seconds: u64) -> Result<(), DatabaseError> {
    if !(MIN_LEASE_SECONDS..=MAX_LEASE_SECONDS).contains(&seconds) {
        return Err(DatabaseError {
            message: format!(
                "outbox lease duration must be between {MIN_LEASE_SECONDS} and {MAX_LEASE_SECONDS} seconds"
            ),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Mutex;

    static NEXT_TEST_ID: AtomicU64 = AtomicU64::new(0);
    // These integration tests share the configured MariaDB outbox. Keeping
    // their write-and-page/claim sequences serialized prevents one test from
    // changing the result set while another is asserting offset pagination
    // or claiming its event.
    static MARIADB_OUTBOX_TEST_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn event_envelope_accepts_stable_ids_names_and_json() {
        let event = OutboxEvent::new(
            "evt_01hxyz-9",
            "orders.created",
            &serde_json::json!({"order_id": 42}),
        )
        .unwrap();
        assert_eq!(event.id, "evt_01hxyz-9");
        assert_eq!(event.event_type, "orders.created");
        assert_eq!(event.payload_json, r#"{"order_id":42}"#);
    }

    #[test]
    fn event_envelope_rejects_invalid_ids_and_types() {
        let payload = serde_json::json!({});
        for id in ["", "bad id", "non/ascii-é"] {
            assert!(OutboxEvent::new(id, "orders.created", &payload).is_err());
        }
        for event_type in ["Orders.created", ".created", "orders.", "orders;drop"] {
            assert!(OutboxEvent::new("evt-1", event_type, &payload).is_err());
        }
    }

    #[test]
    fn event_envelope_enforces_payload_limit() {
        let payload = serde_json::json!({"large": "x".repeat(MAX_EVENT_PAYLOAD_BYTES)});
        assert!(OutboxEvent::new("evt-1", "test.payload", &payload)
            .unwrap_err()
            .message
            .contains(&format!("{} byte limit", MAX_EVENT_PAYLOAD_BYTES)));
    }

    #[test]
    fn lease_configuration_is_bounded_and_tokens_are_validated() {
        assert!(validate_lease_duration(MIN_LEASE_SECONDS).is_ok());
        assert!(validate_lease_duration(MAX_LEASE_SECONDS).is_ok());
        assert!(validate_lease_duration(MIN_LEASE_SECONDS - 1).is_err());
        assert!(validate_lease_duration(MAX_LEASE_SECONDS + 1).is_err());
        assert!(validate_lease_token("worker-01").is_ok());
        assert!(validate_lease_token("worker 01").is_err());
    }

    #[test]
    fn handler_registry_validates_and_rejects_duplicate_event_types() {
        let mut handlers = OutboxHandlerRegistry::default();
        handlers.register("orders.created", |_, _| Ok(())).unwrap();
        assert!(handlers.register("Orders.created", |_, _| Ok(())).is_err());
        assert!(handlers.register("orders.created", |_, _| Ok(())).is_err());
    }

    #[test]
    fn worker_with_no_registered_handlers_stays_idle_without_connecting() {
        assert_eq!(
            run_mariadb_outbox_once("", &OutboxHandlerRegistry::default(), "worker-empty", 30),
            Ok(OutboxWorkerResult::Idle)
        );
    }

    #[test]
    fn outbox_inspection_limit_is_bounded_before_database_access() {
        assert!(inspect_mariadb_outbox("", 0).is_err());
        assert!(inspect_mariadb_outbox("", MAX_OUTBOX_INSPECTION_LIMIT + 1).is_err());
        assert!(inspect_mariadb_outbox_page("", 25, MAX_OUTBOX_INSPECTION_OFFSET + 1).is_err());
    }

    #[test]
    fn mariadb_inspection_pages_mail_metadata_and_requeues_without_delivery() {
        let Ok(database_url) = env::var("ZELYRA_DB_TIMEOUT_TEST_URL") else {
            eprintln!("skipping MariaDB outbox inspection test: test URL is not configured");
            return;
        };
        let _guard = MARIADB_OUTBOX_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let suffix = format!(
            "{}-{}",
            std::process::id(),
            NEXT_TEST_ID.fetch_add(1, Ordering::Relaxed)
        );
        let id = format!("outbox-studio-{suffix}");
        let other_id = format!("outbox-studio-other-{suffix}");
        ensure_mariadb_outbox(&database_url).unwrap();
        let event = OutboxEvent::new(
            &id,
            "zelyra.mail.send",
            &serde_json::json!({
                "to":"customer@example.test",
                "subject":"Test",
                "text":"Never send this fixture",
                "attachments":[{"filename":"invoice.pdf","content_type":"application/pdf","content":"JVBERi0xLjQ="}]
            }),
        )
        .unwrap();
        let other_event = OutboxEvent::new(
            &other_id,
            "orders.created",
            &serde_json::json!({"order_id":42}),
        )
        .unwrap();
        crate::with_mariadb_transaction(&database_url, |transaction| {
            enqueue_mariadb_event(transaction, &event)?;
            enqueue_mariadb_event(transaction, &other_event)
        })
        .unwrap();
        let page = inspect_mariadb_outbox_page(&database_url, 200, 0).unwrap();
        let inspected = page.events.iter().find(|item| item.id == id).unwrap();
        assert_eq!(
            inspected.recipient.as_deref(),
            Some("customer@example.test")
        );
        assert_eq!(inspected.attachment_name.as_deref(), Some("invoice.pdf"));
        assert_eq!(inspected.status, "pending");
        assert!(page.total >= 2);
        let first = inspect_mariadb_outbox_page(&database_url, 1, 0).unwrap();
        let second = inspect_mariadb_outbox_page(&database_url, 1, 1).unwrap();
        assert_eq!(first.events.len(), 1);
        assert_eq!(second.events.len(), 1);
        assert_ne!(first.events[0].id, second.events[0].id);
        execute_mariadb_query(
            &database_url,
            "UPDATE `_zelyra_outbox` SET `exhausted_at` = UTC_TIMESTAMP(6), `attempts` = 8 WHERE `event_id` = :event_id",
            vec![("event_id".into(), QueryValue::String(id.clone()))],
        )
        .unwrap();
        assert!(requeue_exhausted_mariadb_event(&database_url, &id).unwrap());
        let result = execute_mariadb_query(
            &database_url,
            "SELECT `delivered_at` IS NULL, `attempts`, `exhausted_at` IS NULL FROM `_zelyra_outbox` WHERE `event_id` = :event_id",
            vec![("event_id".into(), QueryValue::String(id.clone()))],
        )
        .unwrap();
        assert_eq!(result.rows[0][1], "0");
        assert_eq!(result.rows[0][0], "1");
        assert_eq!(result.rows[0][2], "1");
        execute_mariadb_query(
            &database_url,
            "DELETE FROM `_zelyra_outbox` WHERE `event_id` IN (:event_id, :other_id)",
            vec![
                ("event_id".into(), QueryValue::String(id)),
                ("other_id".into(), QueryValue::String(other_id)),
            ],
        )
        .unwrap();
    }

    #[test]
    fn mariadb_schedule_jobs_deduplicate_cancel_and_inspect_without_payloads() {
        let Ok(database_url) = env::var("ZELYRA_DB_TIMEOUT_TEST_URL") else {
            eprintln!("skipping MariaDB schedule-job test: test URL is not configured");
            return;
        };
        let _guard = MARIADB_OUTBOX_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let suffix = format!(
            "{}-{}",
            std::process::id(),
            NEXT_TEST_ID.fetch_add(1, Ordering::Relaxed)
        );
        let id = format!("schedule-test-{suffix}");
        let pending_id = format!("schedule-pending-{suffix}");
        let foreign_id = format!("schedule-foreign-{suffix}");
        let worker_id = format!("schedule-worker-test-{suffix}");
        let project_id = "b".repeat(64);
        ensure_mariadb_outbox(&database_url).unwrap();
        let event = OutboxEvent::new(
            &id,
            SCHEDULE_JOB_EVENT_TYPE,
            &serde_json::json!({
                "project_id": project_id,
                "schedule_name": "weekly_report",
                "scheduled_at": "2026-10-17T12:00:00Z",
                "fingerprint": "a".repeat(64),
                "secret": "must never appear in inspection",
            }),
        )
        .unwrap();
        enqueue_mariadb_event_once(&database_url, &event).unwrap();
        enqueue_mariadb_event_once(&database_url, &event).unwrap();
        let pending = OutboxEvent::new(
            &pending_id,
            SCHEDULE_JOB_EVENT_TYPE,
            &serde_json::json!({
                "project_id": project_id,
                "schedule_name": "removed_schedule",
                "scheduled_at": "2026-10-17T12:01:00Z",
            }),
        )
        .unwrap();
        enqueue_mariadb_event_once(&database_url, &pending).unwrap();
        let foreign = OutboxEvent::new(
            &foreign_id,
            SCHEDULE_JOB_EVENT_TYPE,
            &serde_json::json!({
                "project_id": "c".repeat(64),
                "schedule_name": "another_project",
                "scheduled_at": "2026-10-17T12:02:00Z",
            }),
        )
        .unwrap();
        enqueue_mariadb_event_once(&database_url, &foreign).unwrap();
        cancel_pending_mariadb_schedule_jobs(&database_url, &project_id).unwrap();
        let conflicting = OutboxEvent::new(
            &id,
            SCHEDULE_JOB_EVENT_TYPE,
            &serde_json::json!({"schedule_name": "other"}),
        )
        .unwrap();
        assert!(enqueue_mariadb_event_once(&database_url, &conflicting).is_err());

        let page = inspect_mariadb_schedule_jobs_page(&database_url, &project_id, 200, 0).unwrap();
        let summary = page.events.iter().find(|item| item.id == id).unwrap();
        assert!(page.events.iter().all(|item| item.id != foreign_id));
        assert_eq!(summary.schedule_name.as_deref(), Some("weekly_report"));
        assert_eq!(
            summary.scheduled_at.as_deref(),
            Some("2026-10-17T12:00:00Z")
        );
        assert_eq!(summary.status, "cancelled");
        assert!(!requeue_exhausted_mariadb_event(&database_url, &id).unwrap());
        assert_eq!(
            page.events
                .iter()
                .find(|item| item.id == pending_id)
                .unwrap()
                .status,
            "cancelled"
        );

        let worker_event = OutboxEvent::new(
            &worker_id,
            SCHEDULE_JOB_EVENT_TYPE,
            &serde_json::json!({
                "project_id": project_id,
                "schedule_name": "weekly_report",
                "scheduled_at": "2026-10-17T12:00:00Z",
            }),
        )
        .unwrap();
        enqueue_mariadb_event_once(&database_url, &worker_event).unwrap();
        let mut handlers = OutboxHandlerRegistry::default();
        handlers
            .register(SCHEDULE_JOB_EVENT_TYPE, |_, context| {
                context.cancel();
                Ok(())
            })
            .unwrap();
        assert_eq!(
            run_mariadb_outbox_once_for_project(
                &database_url,
                &handlers,
                &format!("worker-{suffix}"),
                30,
                &project_id,
            )
            .unwrap(),
            OutboxWorkerResult::Cancelled {
                event_id: worker_id.clone(),
                attempts: 1,
            }
        );
        let page = inspect_mariadb_schedule_jobs_page(&database_url, &project_id, 200, 0).unwrap();
        assert_eq!(
            page.events
                .iter()
                .find(|item| item.id == worker_id)
                .unwrap()
                .status,
            "cancelled"
        );
        assert_eq!(
            run_mariadb_outbox_once_for_project(
                &database_url,
                &handlers,
                &format!("worker-{suffix}-again"),
                30,
                &project_id
            )
            .unwrap(),
            OutboxWorkerResult::Idle
        );
        execute_mariadb_query(
            &database_url,
            "DELETE FROM `_zelyra_outbox` WHERE `event_id` IN (:event_id, :pending_id, :foreign_id, :worker_id)",
            vec![
                ("event_id".into(), QueryValue::String(id)),
                ("pending_id".into(), QueryValue::String(pending_id)),
                ("foreign_id".into(), QueryValue::String(foreign_id)),
                ("worker_id".into(), QueryValue::String(worker_id)),
            ],
        )
        .unwrap();
    }

    #[test]
    fn outbox_inspection_redacts_unrecognized_error_values() {
        assert_eq!(safe_last_error(""), None);
        assert_eq!(
            safe_last_error("handler_failure"),
            Some("handler_failure".into())
        );
        assert_eq!(
            safe_last_error("secret\n\u{1b}[31m"),
            Some("unknown".into())
        );
    }

    #[test]
    fn schedule_inspection_rejects_untrusted_payload_metadata() {
        assert_eq!(
            safe_schedule_name("weekly_report").as_deref(),
            Some("weekly_report")
        );
        assert_eq!(safe_schedule_name("bad\u{1b}name"), None);
        assert_eq!(
            safe_schedule_time("2026-10-17T12:00:00Z").as_deref(),
            Some("2026-10-17T12:00:00Z")
        );
        assert_eq!(
            safe_schedule_time("2026-10-17T12:01:00Z").as_deref(),
            Some("2026-10-17T12:01:00Z")
        );
        assert_eq!(safe_schedule_time("2026-10-17T12:01:30Z"), None);
        assert_eq!(safe_schedule_time("2026-99-99T99:99:00Z"), None);
    }

    #[test]
    fn mariadb_outbox_insert_commits_and_rolls_back_with_business_data() {
        let Ok(database_url) = env::var("ZELYRA_DB_TIMEOUT_TEST_URL") else {
            eprintln!("skipping MariaDB outbox transaction test: test URL is not configured");
            return;
        };
        let _guard = MARIADB_OUTBOX_TEST_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let suffix = format!(
            "{}-{}",
            std::process::id(),
            NEXT_TEST_ID.fetch_add(1, Ordering::Relaxed)
        );
        let committed_id = format!("outbox-commit-{suffix}");
        let rolled_back_id = format!("outbox-rollback-{suffix}");
        ensure_mariadb_outbox(&database_url).unwrap();
        execute_mariadb_query(
            &database_url,
            "CREATE TABLE IF NOT EXISTS `_zelyra_outbox_atomicity_test` (\
                `event_id` VARCHAR(128) CHARACTER SET ascii COLLATE ascii_bin NOT NULL PRIMARY KEY,\
                `value` VARCHAR(32) NOT NULL\
            ) ENGINE=InnoDB",
            Vec::new(),
        )
        .unwrap();

        let event = OutboxEvent::new(
            &committed_id,
            "orders.created",
            &serde_json::json!({"order_id": 1}),
        )
        .unwrap();
        crate::with_mariadb_transaction(&database_url, |transaction| {
            transaction.execute(&Query {
                sql: "INSERT INTO `_zelyra_outbox_atomicity_test` (`event_id`, `value`) VALUES (:event_id, 'committed')".into(),
                params: vec![("event_id".into(), QueryValue::String(committed_id.clone()))],
            })?;
            enqueue_mariadb_event(transaction, &event)
        })
        .unwrap();

        let rollback_event = OutboxEvent::new(
            &rolled_back_id,
            "orders.created",
            &serde_json::json!({"order_id": 2}),
        )
        .unwrap();
        let rollback: Result<(), DatabaseError> = crate::with_mariadb_transaction(
            &database_url,
            |transaction| {
                transaction.execute(&Query {
                sql: "INSERT INTO `_zelyra_outbox_atomicity_test` (`event_id`, `value`) VALUES (:event_id, 'rolled-back')".into(),
                params: vec![("event_id".into(), QueryValue::String(rolled_back_id.clone()))],
            })?;
                enqueue_mariadb_event(transaction, &rollback_event)?;
                Err(DatabaseError {
                    message: "intentional rollback for outbox test".into(),
                })
            },
        );
        assert!(rollback.is_err());

        for (id, expected) in [(&committed_id, "1"), (&rolled_back_id, "0")] {
            for table in ["_zelyra_outbox_atomicity_test", "_zelyra_outbox"] {
                let result = execute_mariadb_query(
                    &database_url,
                    &format!(
                        "SELECT COUNT(*) AS `count` FROM `{table}` WHERE `event_id` = :event_id"
                    ),
                    vec![("event_id".into(), QueryValue::String(id.clone()))],
                )
                .unwrap();
                assert_eq!(
                    result.rows[0][0], expected,
                    "unexpected {table} count for {id}"
                );
            }
        }

        let mut claim = claim_mariadb_event(&database_url, "worker-first", 30)
            .unwrap()
            .expect("committed event should be claimable");
        assert_eq!(claim.event.id, committed_id);
        assert_eq!(claim.attempts, 1);
        assert!(!acknowledge_mariadb_event(&database_url, &committed_id, "worker-stale").unwrap());
        assert!(fail_mariadb_event(&database_url, &committed_id, &claim.lease_token).unwrap());

        for expected_attempt in 2..=MAX_DELIVERY_ATTEMPTS {
            execute_mariadb_query(
                &database_url,
                "UPDATE `_zelyra_outbox` SET `available_at` = UTC_TIMESTAMP(6) WHERE `event_id` = :event_id",
                vec![("event_id".into(), QueryValue::String(committed_id.clone()))],
            )
            .unwrap();
            claim = claim_mariadb_event(&database_url, &format!("worker-{expected_attempt}"), 30)
                .unwrap()
                .expect("failed event should become claimable after its retry time");
            assert_eq!(claim.attempts, expected_attempt);
            assert!(fail_mariadb_event(&database_url, &committed_id, &claim.lease_token).unwrap());
        }
        assert!(claim_mariadb_event(&database_url, "worker-exhausted", 30)
            .unwrap()
            .is_none());
        assert!(requeue_exhausted_mariadb_event(&database_url, &committed_id).unwrap());
        let recovered = claim_mariadb_event(&database_url, "worker-recovered", 30)
            .unwrap()
            .expect("operator-released event should be claimable");
        assert_eq!(recovered.attempts, 1);
        assert!(
            acknowledge_mariadb_event(&database_url, &committed_id, &recovered.lease_token)
                .unwrap()
        );
        assert!(
            !acknowledge_mariadb_event(&database_url, &committed_id, &recovered.lease_token)
                .unwrap()
        );

        let race_id = format!("outbox-race-{suffix}");
        let race_event = OutboxEvent::new(
            &race_id,
            "orders.created",
            &serde_json::json!({"order_id": 3}),
        )
        .unwrap();
        crate::with_mariadb_transaction(&database_url, |transaction| {
            enqueue_mariadb_event(transaction, &race_event)
        })
        .unwrap();
        let (first, second) = std::thread::scope(|scope| {
            let first_url = database_url.clone();
            let second_url = database_url.clone();
            let first = scope.spawn(move || claim_mariadb_event(&first_url, "worker-race-a", 30));
            let second = scope.spawn(move || claim_mariadb_event(&second_url, "worker-race-b", 30));
            (first.join().unwrap(), second.join().unwrap())
        });
        let claims = [first.unwrap(), second.unwrap()]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        assert_eq!(claims.len(), 1, "one active claim must win the race");
        assert_eq!(claims[0].event.id, race_id);
        assert!(
            renew_mariadb_event_lease(&database_url, &race_id, &claims[0].lease_token, 60).unwrap()
        );
        let lease_before_short_renewal = execute_mariadb_query(
            &database_url,
            "SELECT `lease_until` FROM `_zelyra_outbox` WHERE `event_id` = :event_id",
            vec![("event_id".into(), QueryValue::String(race_id.clone()))],
        )
        .unwrap()
        .rows[0][0]
            .clone();
        assert!(
            renew_mariadb_event_lease(&database_url, &race_id, &claims[0].lease_token, 5).unwrap()
        );
        let lease_after_short_renewal = execute_mariadb_query(
            &database_url,
            "SELECT `lease_until` FROM `_zelyra_outbox` WHERE `event_id` = :event_id",
            vec![("event_id".into(), QueryValue::String(race_id.clone()))],
        )
        .unwrap()
        .rows[0][0]
            .clone();
        assert_eq!(lease_after_short_renewal, lease_before_short_renewal);
        execute_mariadb_query(
            &database_url,
            "UPDATE `_zelyra_outbox` SET `lease_until` = DATE_SUB(UTC_TIMESTAMP(6), INTERVAL 1 SECOND) WHERE `event_id` = :event_id",
            vec![("event_id".into(), QueryValue::String(race_id.clone()))],
        )
        .unwrap();
        assert!(
            !renew_mariadb_event_lease(&database_url, &race_id, &claims[0].lease_token, 60)
                .unwrap()
        );
        let reclaimed = claim_mariadb_event(&database_url, "worker-race-recovered", 30)
            .unwrap()
            .expect("expired lease should be claimable again");
        assert_eq!(reclaimed.event.id, race_id);
        assert_eq!(reclaimed.attempts, 2);
        assert!(
            !acknowledge_mariadb_event(&database_url, &race_id, &claims[0].lease_token).unwrap()
        );
        assert!(
            acknowledge_mariadb_event(&database_url, &race_id, &reclaimed.lease_token).unwrap()
        );

        let worker_id = format!("outbox-worker-{suffix}");
        let unhandled_id = format!("outbox-unhandled-{suffix}");
        let unhandled_event = OutboxEvent::new(
            &unhandled_id,
            "worker.unregistered",
            &serde_json::json!({"ready": false}),
        )
        .unwrap();
        let worker_event = OutboxEvent::new(
            &worker_id,
            "worker.test",
            &serde_json::json!({"ready": true}),
        )
        .unwrap();
        crate::with_mariadb_transaction(&database_url, |transaction| {
            enqueue_mariadb_event(transaction, &unhandled_event)?;
            enqueue_mariadb_event(transaction, &worker_event)
        })
        .unwrap();
        let mut handlers = OutboxHandlerRegistry::default();
        let expected_worker_id = worker_id.clone();
        handlers
            .register("worker.test", move |event, context| {
                assert_eq!(event.id, expected_worker_id);
                assert!(context.renew_lease().unwrap());
                Ok(())
            })
            .unwrap();
        assert_eq!(
            run_mariadb_outbox_once(&database_url, &handlers, "worker-once", 30).unwrap(),
            OutboxWorkerResult::Delivered {
                event_id: worker_id.clone(),
                attempts: 1
            }
        );
        let unhandled = inspect_mariadb_outbox(&database_url, MAX_OUTBOX_INSPECTION_LIMIT)
            .unwrap()
            .into_iter()
            .find(|summary| summary.id == unhandled_id)
            .expect("unregistered event should remain visible");
        assert_eq!(unhandled.attempts, 0);
        assert_eq!(unhandled.status, "pending");

        execute_mariadb_query(
            &database_url,
            "DELETE FROM `_zelyra_outbox_atomicity_test` WHERE `event_id` IN (:committed_id, :rolled_back_id)",
            vec![
                ("committed_id".into(), QueryValue::String(committed_id.clone())),
                ("rolled_back_id".into(), QueryValue::String(rolled_back_id.clone())),
            ],
        )
        .unwrap();
        execute_mariadb_query(
            &database_url,
            "DELETE FROM `_zelyra_outbox` WHERE `event_id` IN (:committed_id, :rolled_back_id, :race_id, :worker_id, :unhandled_id)",
            vec![
                ("committed_id".into(), QueryValue::String(committed_id)),
                ("rolled_back_id".into(), QueryValue::String(rolled_back_id)),
                ("race_id".into(), QueryValue::String(race_id)),
                ("worker_id".into(), QueryValue::String(worker_id)),
                ("unhandled_id".into(), QueryValue::String(unhandled_id)),
            ],
        )
        .unwrap();
    }
}
