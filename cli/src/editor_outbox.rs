//! Bounded, authenticated Studio access to existing MariaDB outbox operations.
use super::*;

const PAGE_SIZE: usize = 25;

fn database_url(state: &EditorState) -> Result<Option<String>, (u16, String)> {
    let entry = state.root.join("main.zyl");
    let schema = database_cli::load_schema(&entry.to_string_lossy())
        .map_err(|_| (422, "project database declaration could not be read".into()))?;
    if schema.backend() != Backend::MariaDb {
        return Err((
            422,
            "Studio outbox currently supports MariaDB projects only".into(),
        ));
    }
    super::super::database_url_for_project(
        &state.root,
        schema
            .database
            .as_ref()
            .map(|database| database.name.as_str()),
    )
    .map_err(|_| {
        (
            422,
            "project database configuration could not be read".into(),
        )
    })
}

pub(super) fn list(
    state: &EditorState,
    request: &Request,
) -> Result<(u16, &'static str, String), (u16, String)> {
    let query = request
        .target
        .split_once('?')
        .map(|(_, query)| query)
        .unwrap_or_default();
    let values = zelyra_web::parse_urlencoded(query).map_err(|error| (400, error.message))?;
    let offset = values
        .get("offset")
        .map(|value| value.parse::<usize>())
        .transpose()
        .map_err(|_| (400, "outbox offset must be a nonnegative integer".into()))?
        .unwrap_or(0);
    if offset > zelyra_database::outbox::MAX_OUTBOX_INSPECTION_OFFSET {
        return Err((400, "outbox offset exceeds the inspection limit".into()));
    }
    let Some(database_url) = database_url(state)? else {
        return Ok((
            200,
            "application/json; charset=utf-8",
            json!({"configured":false,"reason":"Configure the project MariaDB URL in .env or DATABASE_URL."}).to_string(),
        ));
    };
    let page = zelyra_database::outbox::inspect_mariadb_outbox_page(
        &database_url,
        PAGE_SIZE,
        offset,
    )
    .map_err(|_| {
        (
            502,
            "Outbox unavailable; check the MariaDB connection and run `zelyra outbox setup main.zyl`."
                .into(),
        )
    })?;
    let events = page
        .events
        .iter()
        .map(|event| {
            json!({
                "id":event.id,
                "type":event.event_type,
                "status":event.status,
                "attempts":event.attempts,
                "createdAt":event.created_at,
                "availableAt":event.available_at,
                "leaseUntil":event.lease_until,
                "deliveredAt":event.delivered_at,
                "exhaustedAt":event.exhausted_at,
                "lastError":event.last_error,
                "recipient":event.recipient,
                "attachmentName":event.attachment_name,
            })
        })
        .collect::<Vec<_>>();
    Ok((
        200,
        "application/json; charset=utf-8",
        json!({"configured":true,"total":page.total,"offset":page.offset,"pageSize":PAGE_SIZE,"events":events}).to_string(),
    ))
}

pub(super) fn requeue(
    state: &EditorState,
    value: &Value,
) -> Result<(u16, &'static str, String), (u16, String)> {
    if value.get("confirm").and_then(Value::as_bool) != Some(true) {
        return Err((400, "confirm requeue explicitly".into()));
    }
    let id = value
        .get("eventId")
        .and_then(Value::as_str)
        .ok_or((400, "outbox event ID is required".into()))?;
    let database_url = database_url(state)?.ok_or((
        422,
        "Configure the project MariaDB URL before requeueing an event.".into(),
    ))?;
    match zelyra_database::outbox::requeue_exhausted_mariadb_event(&database_url, id) {
        Ok(true) => Ok((
            200,
            "application/json; charset=utf-8",
            json!({"requeued":true,"eventId":id,"sent":false}).to_string(),
        )),
        Ok(false) => Err((409, "event is missing or is not exhausted".into())),
        Err(_) => Err((
            502,
            "outbox requeue failed; check the MariaDB connection".into(),
        )),
    }
}
