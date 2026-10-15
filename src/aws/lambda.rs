use serde_json::{Value, json};

use crate::model::{Resource, Snapshot};

use super::{
    FetchContext,
    common::{array, optional_string, panel_error, resource, selected_id, sort_and_limit, string},
};

pub(super) fn fetch(cli: &FetchContext<'_>, selected: Option<&str>) -> Snapshot {
    let mut snapshot = Snapshot::empty(crate::model::Service::Lambda);
    let response = match cli.invoke(
        &mut snapshot.logs,
        &["lambda".into(), "list-functions".into()],
        true,
    ) {
        Ok(response) => response,
        Err(error) => {
            panel_error(&mut snapshot, 0, error);
            return snapshot;
        }
    };

    match functions(&response) {
        Ok(items) => snapshot.panels[0].items = items,
        Err(error) => panel_error(&mut snapshot, 0, error),
    }
    sort_and_limit(
        &mut snapshot.panels[0],
        &mut snapshot.logs,
        "Lambda functions",
    );
    let Some(function_arn) = selected_id(&snapshot.panels[0].items, selected) else {
        return snapshot;
    };

    let versions_command = vec![
        "lambda".into(),
        "list-versions-by-function".into(),
        "--function-name".into(),
        function_arn.clone(),
    ];
    match cli
        .invoke(&mut snapshot.logs, &versions_command, true)
        .and_then(|response| versions(&response))
    {
        Ok(items) => snapshot.panels[1].items = items,
        Err(error) => panel_error(&mut snapshot, 1, error),
    }
    sort_and_limit(
        &mut snapshot.panels[1],
        &mut snapshot.logs,
        "Lambda versions",
    );

    let aliases_command = vec![
        "lambda".into(),
        "list-aliases".into(),
        "--function-name".into(),
        function_arn.clone(),
    ];
    match cli
        .invoke(&mut snapshot.logs, &aliases_command, true)
        .and_then(|response| aliases(&response))
    {
        Ok(items) => snapshot.panels[2].items = items,
        Err(error) => panel_error(&mut snapshot, 2, error),
    }
    sort_and_limit(
        &mut snapshot.panels[2],
        &mut snapshot.logs,
        "Lambda aliases",
    );

    let tags_command = vec![
        "lambda".into(),
        "list-tags".into(),
        "--resource".into(),
        function_arn.clone(),
    ];
    match cli
        .invoke(&mut snapshot.logs, &tags_command, false)
        .and_then(|response| tags(&response))
    {
        Ok(items) => snapshot.panels[3].items = items,
        Err(error) => panel_error(&mut snapshot, 3, error),
    }
    sort_and_limit(&mut snapshot.panels[3], &mut snapshot.logs, "Lambda tags");
    snapshot
}

fn functions(response: &Value) -> Result<Vec<Resource>, String> {
    array(response, "Functions")?
        .iter()
        .map(|function| {
            let arn = string(function, "FunctionArn")?;
            let name = optional_string(function, "FunctionName")?.unwrap_or(arn);
            Ok(resource(
                arn,
                name,
                function_summary(function)?,
                function.clone(),
            ))
        })
        .collect()
}

fn function_summary(function: &Value) -> Result<String, String> {
    let runtime = optional_string(function, "Runtime")?.unwrap_or("");
    let package = optional_string(function, "PackageType")?.unwrap_or("");
    let base = if runtime.is_empty() {
        if package.is_empty() {
            "unknown".to_string()
        } else {
            package.to_string()
        }
    } else {
        runtime.to_string()
    };
    let memory = function
        .get("MemorySize")
        .and_then(Value::as_u64)
        .map(|size| format!("{size} MB"));
    let timeout = function
        .get("Timeout")
        .and_then(Value::as_u64)
        .map(|timeout| format!("{timeout}s"));
    let summary = [Some(base), memory, timeout]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" · ");
    Ok(summary)
}

fn versions(response: &Value) -> Result<Vec<Resource>, String> {
    array(response, "Versions")?
        .iter()
        .map(|version| {
            let arn = string(version, "FunctionArn")?;
            let number = optional_string(version, "Version")?.unwrap_or("$LATEST");
            let label = if number == "$LATEST" {
                "$LATEST".to_string()
            } else {
                format!("v{number}")
            };
            let modified = optional_string(version, "LastModified")?.unwrap_or("");
            let runtime = optional_string(version, "Runtime")?.unwrap_or("");
            let summary = [runtime, modified]
                .into_iter()
                .filter(|part| !part.is_empty())
                .collect::<Vec<_>>()
                .join(" · ");
            Ok(resource(arn, &label, summary, version.clone()))
        })
        .collect()
}

fn aliases(response: &Value) -> Result<Vec<Resource>, String> {
    array(response, "Aliases")?
        .iter()
        .map(|alias| {
            let arn = string(alias, "AliasArn")?;
            let name = string(alias, "Name")?;
            let version = optional_string(alias, "FunctionVersion")?.unwrap_or("");
            let description = optional_string(alias, "Description")?.unwrap_or("");
            let summary = if version.is_empty() {
                description.to_string()
            } else if description.is_empty() {
                format!("-> {version}")
            } else {
                format!("-> {version} · {description}")
            };
            Ok(resource(arn, name, summary, alias.clone()))
        })
        .collect()
}

fn tags(response: &Value) -> Result<Vec<Resource>, String> {
    let map = match response.get("Tags") {
        None | Some(Value::Null) => return Ok(Vec::new()),
        Some(Value::Object(map)) => map,
        Some(_) => return Err("AWS response `Tags` must be an object".into()),
    };
    let mut items = Vec::with_capacity(map.len());
    for (key, value) in map {
        let value = value
            .as_str()
            .ok_or("AWS response tag values must be strings")?;
        items.push(resource(
            key.clone(),
            key.clone(),
            value.to_string(),
            json!({"Key": key, "Value": value}),
        ));
    }
    items.sort_by(|left: &Resource, right: &Resource| left.id.cmp(&right.id));
    Ok(items)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_functions_versions_aliases_and_tag_map() {
        let response: Value =
            serde_json::from_str(include_str!("../../tests/fixtures/aws_lambda.json")).unwrap();
        let items = functions(&response["listFunctions"]).unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(
            items[0].id,
            "arn:aws:lambda:eu-west-1:123456789012:function:api-handler"
        );
        assert!(items[0].summary.contains("python3.12"));

        let version_items = versions(&response["listVersions"]).unwrap();
        assert_eq!(version_items.len(), 2);
        assert_eq!(version_items[0].label, "v3");
        assert_eq!(version_items[1].label, "$LATEST");

        let alias_items = aliases(&response["listAliases"]).unwrap();
        assert_eq!(alias_items.len(), 1);
        assert_eq!(alias_items[0].label, "prod");
        assert!(alias_items[0].summary.contains("3"));

        let tag_items = tags(&response["listTags"]).unwrap();
        assert_eq!(tag_items.len(), 2);
        assert_eq!(tag_items[0].id, "Environment");
        assert_eq!(tag_items[0].detail["Value"], "production");
    }

    #[test]
    fn rejects_missing_function_collection_and_non_string_tags() {
        assert!(functions(&Value::Object(Default::default())).is_err());
        assert!(versions(&Value::Object(Default::default())).is_err());
        assert!(aliases(&Value::Object(Default::default())).is_err());
        assert!(tags(&json!({"Tags": {"Team": 1}})).is_err());
        assert!(tags(&json!({"Tags": [1]})).is_err());
        assert!(tags(&json!({})).unwrap().is_empty());
    }
}
