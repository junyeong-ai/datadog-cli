use chrono::{DateTime, Utc};
use serde_json::{Value, json};

use crate::catalog::{Catalog, TimeEncoding};
use crate::error::{DatadogError, Result};
use crate::time::{TimeExpr, TimeRange};

pub fn apply_time_range(
    input: &mut Value,
    domain: &str,
    id: &str,
    from: Option<TimeExpr>,
    to: Option<TimeExpr>,
    now: DateTime<Utc>,
) -> Result<Option<Value>> {
    let Some(from) = from else {
        if to.is_some() {
            return Err(DatadogError::InvalidInput("--to requires --from".into()));
        }
        return Ok(None);
    };
    let operation = Catalog::load()?.query(domain, id)?;
    let contract = operation.time_range.as_ref().ok_or_else(|| {
        DatadogError::InvalidInput(
            "This operation has no reviewed time-range mapping; use its native input schema".into(),
        )
    })?;
    if input.pointer(&contract.from).is_some() || input.pointer(&contract.to).is_some() {
        return Err(DatadogError::InvalidInput(
            "Range flags conflict with time fields in --input".into(),
        ));
    }
    let range = TimeRange::resolve(from, to.unwrap_or(TimeExpr::Ago(0)), now)?;
    let (from, to) = (range.from, range.to);
    let (from_date, to_date) = range.rfc3339()?;
    let (encoded_from, encoded_to) = match contract.encoding {
        TimeEncoding::UnixSeconds => (json!(from), json!(to)),
        TimeEncoding::UnixMilliseconds => {
            let convert = |value: i64| {
                value.checked_mul(1000).ok_or_else(|| {
                    DatadogError::InvalidInput(
                        "Time range exceeds milliseconds representation".into(),
                    )
                })
            };
            (json!(convert(from)?), json!(convert(to)?))
        }
        TimeEncoding::Rfc3339 => (json!(from_date), json!(to_date)),
    };
    set_pointer(input, &contract.from, encoded_from)?;
    set_pointer(input, &contract.to, encoded_to)?;
    Ok(Some(
        json!({"from":from_date,"to":to_date,"encoding":contract.encoding}),
    ))
}

pub(super) fn set_pointer(document: &mut Value, pointer: &str, value: Value) -> Result<()> {
    let fields = pointer
        .strip_prefix('/')
        .ok_or_else(|| DatadogError::DecodeError("Invalid bundled request pointer".into()))?
        .split('/')
        .map(|field| field.replace("~1", "/").replace("~0", "~"))
        .collect::<Vec<_>>();
    let mut target = document;
    for (index, field) in fields.iter().enumerate() {
        let object = target.as_object_mut().ok_or_else(|| {
            DatadogError::InvalidInput("Request field ancestors must be objects".into())
        })?;
        if index + 1 == fields.len() {
            object.insert(field.clone(), value);
            return Ok(());
        }
        target = object.entry(field.clone()).or_insert_with(|| json!({}));
    }
    Err(DatadogError::DecodeError(
        "Invalid bundled request pointer".into(),
    ))
}
