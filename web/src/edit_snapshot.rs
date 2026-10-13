//! Native NULL-aware optimistic edit snapshots. Tokens keep their existing
//! HMAC/hex transport; SQL NULL is signed as JSON null, text as JSON strings.
use super::*;

#[derive(Default)]
pub(super) struct EditValues {
    pub(super) values: HashMap<String, String>,
    pub(super) nulls: HashSet<String>,
}

impl EditValues {
    pub(super) fn from_result(
        form: &FormRoute,
        result: &zelyra_database::QueryResult,
    ) -> Result<Option<Self>, zelyra_database::DatabaseError> {
        let invalid = || zelyra_database::DatabaseError {
            message: "edit form database result has inconsistent row/NULL metadata".into(),
        };
        if result.rows.len() > 1
            || result.rows.len() != result.nulls.len()
            || result.columns.len() != form.form.fields.len()
        {
            return Err(invalid());
        }
        let Some(row) = result.rows.first() else {
            return Ok(None);
        };
        let nulls = &result.nulls[0];
        if row.len() != form.form.fields.len() || nulls.len() != row.len() {
            return Err(invalid());
        }
        let mut values = HashMap::new();
        let mut null_fields = HashSet::new();
        for ((field, value), null) in form.form.fields.iter().zip(row).zip(nulls) {
            if values.insert(field.name.clone(), value.clone()).is_some() {
                return Err(invalid());
            }
            if *null {
                null_fields.insert(field.name.clone());
            }
        }
        Ok(Some(Self {
            values,
            nulls: null_fields,
        }))
    }

    pub(super) fn rendered_values(&self) -> HashMap<String, String> {
        let mut values = self.values.clone();
        for field in &self.nulls {
            values.insert(field.clone(), String::new());
        }
        values
    }
}

pub(super) fn sign_form_snapshot(csrf: &CsrfProtection, values: &EditValues) -> Option<String> {
    Some(hex_encode(&form_snapshot_signature(csrf, values)?))
}

fn form_snapshot_signature(csrf: &CsrfProtection, values: &EditValues) -> Option<Vec<u8>> {
    if values
        .nulls
        .iter()
        .any(|field| !values.values.contains_key(field))
    {
        return None;
    }
    let snapshot = values
        .values
        .iter()
        .map(|(field, value)| {
            (
                field,
                (!values.nulls.contains(field)).then_some(value.as_str()),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let payload = serde_json::to_vec(&snapshot).ok()?;
    let key = hmac::Key::new(hmac::HMAC_SHA256, csrf.token.as_bytes());
    Some(hmac::sign(&key, &payload).as_ref().to_vec())
}

pub(super) fn verify_form_snapshot(token: &str) -> Option<Vec<u8>> {
    if token.len() != 64 {
        return None;
    }
    let signature = hex_decode(token)?;
    (signature.len() == 32).then_some(signature)
}

pub(super) fn form_snapshot_matches(
    csrf: &CsrfProtection,
    expected_signature: &[u8],
    values: &EditValues,
) -> bool {
    let Some(snapshot) = form_snapshot_signature(csrf, values) else {
        return false;
    };
    constant_time_equal(expected_signature, &snapshot)
}
