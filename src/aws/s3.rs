use serde_json::{Value, json};

use crate::{
    config::MAX_ITEMS,
    model::{Resource, Snapshot},
};

use super::{
    FetchContext,
    common::{
        array, optional_array, optional_string, panel_error, resource, selected_id, sort_and_limit,
        string,
    },
};

pub(super) fn fetch(cli: &FetchContext<'_>, selected: Option<&str>) -> Snapshot {
    let mut snapshot = Snapshot::empty(crate::model::Service::S3);
    let response = match cli.invoke(
        &mut snapshot.logs,
        &[
            "s3api".into(),
            "list-buckets".into(),
            "--page-size".into(),
            MAX_ITEMS.to_string(),
        ],
        true,
    ) {
        Ok(response) => response,
        Err(error) => {
            panel_error(&mut snapshot, 0, error);
            return snapshot;
        }
    };

    match buckets(&response) {
        Ok(items) => snapshot.panels[0].items = items,
        Err(error) => panel_error(&mut snapshot, 0, error),
    }
    sort_and_limit(&mut snapshot.panels[0], &mut snapshot.logs, "S3 buckets");
    let Some(bucket_name) = selected_id(&snapshot.panels[0].items, selected) else {
        return snapshot;
    };
    let bucket = snapshot.panels[0]
        .items
        .iter()
        .find(|item| item.id == bucket_name)
        .map(|item| item.detail.clone())
        .unwrap_or(Value::Null);

    let region = match bucket_region(&bucket) {
        Ok(Some(region)) => Some(region),
        Ok(None) => match get_bucket_location(cli, &mut snapshot, &bucket_name) {
            Ok(region) => Some(region),
            Err(error) => {
                panel_error(&mut snapshot, 3, &error);
                panel_error(
                    &mut snapshot,
                    1,
                    format!("cannot load objects without the bucket Region: {error}"),
                );
                panel_error(
                    &mut snapshot,
                    2,
                    format!("cannot load prefixes without the bucket Region: {error}"),
                );
                None
            }
        },
        Err(error) => {
            panel_error(&mut snapshot, 3, &error);
            panel_error(
                &mut snapshot,
                1,
                format!("cannot load objects without the bucket Region: {error}"),
            );
            panel_error(
                &mut snapshot,
                2,
                format!("cannot load prefixes without the bucket Region: {error}"),
            );
            None
        }
    };

    if let Some(region) = region.as_deref() {
        snapshot.panels[3].items.push(resource(
            "location",
            "Location",
            region,
            json!({"region": region}),
        ));
        let objects_command = vec![
            "s3api".into(),
            "list-objects-v2".into(),
            "--bucket".into(),
            bucket_name.clone(),
        ];
        match cli
            .invoke_region(&mut snapshot.logs, &objects_command, true, Some(region))
            .and_then(|response| objects(&response))
        {
            Ok(items) => snapshot.panels[1].items = items,
            Err(error) => panel_error(&mut snapshot, 1, error),
        }

        let prefixes_command = vec![
            "s3api".into(),
            "list-objects-v2".into(),
            "--bucket".into(),
            bucket_name.clone(),
            "--delimiter".into(),
            "/".into(),
        ];
        match cli
            .invoke_region(&mut snapshot.logs, &prefixes_command, true, Some(region))
            .and_then(|response| prefixes(&response))
        {
            Ok(items) => snapshot.panels[2].items = items,
            Err(error) => panel_error(&mut snapshot, 2, error),
        }

        let versioning_command = vec![
            "s3api".into(),
            "get-bucket-versioning".into(),
            "--bucket".into(),
            bucket_name.clone(),
        ];
        match cli
            .invoke_region(&mut snapshot.logs, &versioning_command, false, Some(region))
            .and_then(|response| versioning(&response))
        {
            Ok(item) => snapshot.panels[3].items.push(item),
            Err(error) => panel_error(&mut snapshot, 3, error),
        }
    }

    sort_and_limit(&mut snapshot.panels[1], &mut snapshot.logs, "S3 objects");
    sort_and_limit(&mut snapshot.panels[2], &mut snapshot.logs, "S3 prefixes");
    sort_and_limit(
        &mut snapshot.panels[3],
        &mut snapshot.logs,
        "S3 bucket settings",
    );
    snapshot
}

fn buckets(response: &Value) -> Result<Vec<Resource>, String> {
    array(response, "Buckets")?
        .iter()
        .map(|bucket| {
            let name = string(bucket, "Name")?;
            let region = optional_string(bucket, "BucketRegion")?;
            let created = optional_string(bucket, "CreationDate")?.unwrap_or("");
            Ok(resource(
                name,
                name,
                region
                    .filter(|region| !region.is_empty())
                    .unwrap_or(created),
                bucket.clone(),
            ))
        })
        .collect()
}

fn bucket_region(bucket: &Value) -> Result<Option<String>, String> {
    Ok(optional_string(bucket, "BucketRegion")?
        .filter(|region| !region.is_empty())
        .map(str::to_string))
}

fn get_bucket_location(
    cli: &FetchContext<'_>,
    snapshot: &mut Snapshot,
    bucket: &str,
) -> Result<String, String> {
    let command = vec![
        "s3api".into(),
        "get-bucket-location".into(),
        "--bucket".into(),
        bucket.into(),
    ];
    let response = cli.invoke(&mut snapshot.logs, &command, false)?;
    let location = response
        .get("LocationConstraint")
        .ok_or("AWS response is missing `LocationConstraint`")?;
    let region = match location {
        Value::Null => "us-east-1",
        Value::String(region) if region == "EU" => "eu-west-1",
        Value::String(region) if !region.is_empty() => region,
        Value::String(_) => {
            return Err("AWS response `LocationConstraint` must not be empty".into());
        }
        _ => return Err("AWS response `LocationConstraint` must be a string or null".into()),
    };
    Ok(region.into())
}

fn objects(response: &Value) -> Result<Vec<Resource>, String> {
    optional_array(response, "Contents")?
        .iter()
        .map(|object| {
            let key = string(object, "Key")?;
            let size = object
                .get("Size")
                .and_then(Value::as_u64)
                .map(|size| format!("{size} bytes"))
                .unwrap_or_else(|| "size unknown".into());
            Ok(resource(key, key, size, object.clone()))
        })
        .collect()
}

fn prefixes(response: &Value) -> Result<Vec<Resource>, String> {
    optional_array(response, "CommonPrefixes")?
        .iter()
        .map(|prefix| {
            let value = string(prefix, "Prefix")?;
            Ok(resource(value, value, "", prefix.clone()))
        })
        .collect()
}

fn versioning(response: &Value) -> Result<Resource, String> {
    let status = optional_string(response, "Status")?.unwrap_or("Disabled");
    Ok(resource(
        "versioning",
        "Versioning",
        status,
        response.clone(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_bucket_regions_and_scoped_contents() {
        let response: Value =
            serde_json::from_str(include_str!("../../tests/fixtures/aws_s3.json")).unwrap();
        let bucket = &array(&response, "Buckets").unwrap()[0];
        assert_eq!(bucket_region(bucket).unwrap().as_deref(), Some("eu-west-1"));
        assert_eq!(
            objects(&response["objectResponse"]).unwrap()[0].id,
            "logs/a.txt"
        );
        assert_eq!(
            prefixes(&response["prefixResponse"]).unwrap()[0].id,
            "logs/"
        );
    }

    #[test]
    fn missing_optional_contents_are_empty_but_malformed_contents_fail() {
        assert!(objects(&json!({})).unwrap().is_empty());
        assert!(objects(&json!({"Contents": "invalid"})).is_err());
    }
}
