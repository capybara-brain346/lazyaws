use serde_json::Value;

use crate::{
    config::MAX_ITEMS,
    model::{Panel, Resource, Snapshot},
};

pub(super) fn array<'a>(value: &'a Value, field: &str) -> Result<&'a [Value], String> {
    value
        .get(field)
        .ok_or_else(|| format!("AWS response is missing `{field}`"))?
        .as_array()
        .map(Vec::as_slice)
        .ok_or_else(|| format!("AWS response `{field}` must be an array"))
}

pub(super) fn optional_array<'a>(value: &'a Value, field: &str) -> Result<&'a [Value], String> {
    match value.get(field) {
        None | Some(Value::Null) => Ok(&[]),
        Some(Value::Array(items)) => Ok(items),
        Some(_) => Err(format!("AWS response `{field}` must be an array")),
    }
}

pub(super) fn string<'a>(value: &'a Value, field: &str) -> Result<&'a str, String> {
    value
        .get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("AWS response is missing string `{field}`"))
}

pub(super) fn optional_string<'a>(
    value: &'a Value,
    field: &str,
) -> Result<Option<&'a str>, String> {
    match value.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(value)) => Ok(Some(value)),
        Some(_) => Err(format!("AWS response `{field}` must be a string")),
    }
}

pub(super) fn resource(
    id: impl Into<String>,
    label: impl Into<String>,
    summary: impl Into<String>,
    detail: Value,
) -> Resource {
    Resource {
        id: id.into(),
        label: label.into(),
        summary: summary.into(),
        detail,
    }
}

pub(super) fn selected_id(items: &[Resource], selected: Option<&str>) -> Option<String> {
    selected
        .and_then(|selected| items.iter().find(|item| item.id == selected))
        .or_else(|| items.first())
        .map(|item| item.id.clone())
}

pub(super) fn sort_and_limit(panel: &mut Panel, logs: &mut Vec<String>, name: &str) {
    panel.items.sort_by(|left, right| left.id.cmp(&right.id));
    if panel.items.len() > MAX_ITEMS {
        let total = panel.items.len();
        panel.items.truncate(MAX_ITEMS);
        logs.push(format!(
            "notice: {name} was limited to {MAX_ITEMS} of {total} items; results are incomplete"
        ));
    }
}

pub(super) fn set_panel(
    snapshot: &mut Snapshot,
    index: usize,
    result: Result<Vec<Resource>, String>,
    name: &str,
) {
    match result {
        Ok(items) => snapshot.panels[index].items = items,
        Err(error) => panel_error(snapshot, index, error),
    }
    sort_and_limit(&mut snapshot.panels[index], &mut snapshot.logs, name);
}

pub(super) fn panel_error(snapshot: &mut Snapshot, index: usize, error: impl AsRef<str>) {
    let panel = &mut snapshot.panels[index];
    match &mut panel.error {
        Some(existing) => {
            existing.push_str("; ");
            existing.push_str(error.as_ref());
        }
        None => panel.error = Some(error.as_ref().to_string()),
    }
}

pub(super) fn tag_resources(
    tags: &[Value],
    key_field: &str,
    value_field: &str,
    prefix: &str,
) -> Result<Vec<Resource>, String> {
    let mut resources = tags
        .iter()
        .map(|tag| {
            let key = string(tag, key_field)?;
            let value = optional_string(tag, value_field)?.unwrap_or("");
            Ok(resource(format!("{prefix}{key}"), key, value, tag.clone()))
        })
        .collect::<Result<Vec<_>, String>>()?;
    resources.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(resources)
}
