use serde_json::{Value, json};

use crate::model::{Resource, Snapshot};

use super::{
    FetchContext,
    common::{
        array, optional_array, optional_string, panel_error, resource, selected_id, set_panel,
        sort_and_limit, string, tag_resources,
    },
};

pub(super) fn fetch(cli: &FetchContext<'_>, selected: Option<&str>) -> Snapshot {
    let mut snapshot = Snapshot::empty(crate::model::Service::Ecr);
    let response = match cli.invoke(
        &mut snapshot.logs,
        &["ecr".into(), "describe-repositories".into()],
        true,
    ) {
        Ok(response) => response,
        Err(error) => {
            panel_error(&mut snapshot, 0, error);
            return snapshot;
        }
    };

    let result = repositories(&response);
    set_panel(&mut snapshot, 0, result, "ECR repositories");
    let Some(repository_name) = selected_id(&snapshot.panels[0].items, selected) else {
        return snapshot;
    };
    let repository = snapshot.panels[0]
        .items
        .iter()
        .find(|item| item.id == repository_name)
        .map(|item| item.detail.clone())
        .unwrap_or(Value::Null);

    let images_command = vec![
        "ecr".into(),
        "describe-images".into(),
        "--repository-name".into(),
        repository_name.clone(),
    ];
    match cli
        .invoke(&mut snapshot.logs, &images_command, true)
        .and_then(|response| images(&response))
    {
        Ok((items, tags)) => {
            snapshot.panels[1].items = items;
            snapshot.panels[2].items = tags;
        }
        Err(error) => {
            panel_error(&mut snapshot, 1, &error);
            panel_error(&mut snapshot, 2, error);
        }
    }
    sort_and_limit(&mut snapshot.panels[1], &mut snapshot.logs, "ECR images");
    sort_and_limit(
        &mut snapshot.panels[2],
        &mut snapshot.logs,
        "ECR image tags",
    );

    let result = string(&repository, "repositoryArn")
        .map(str::to_string)
        .and_then(|arn| {
            let command = vec![
                "ecr".into(),
                "list-tags-for-resource".into(),
                "--resource-arn".into(),
                arn,
            ];
            cli.invoke(&mut snapshot.logs, &command, false)
        })
        .and_then(|response| {
            optional_array(&response, "tags")
                .and_then(|tags| tag_resources(tags, "Key", "Value", ""))
        });
    set_panel(&mut snapshot, 3, result, "ECR repository tags");
    snapshot
}

fn repositories(response: &Value) -> Result<Vec<Resource>, String> {
    array(response, "repositories")?
        .iter()
        .map(|repository| {
            let name = string(repository, "repositoryName")?;
            let uri = optional_string(repository, "repositoryUri")?.unwrap_or("");
            Ok(resource(name, name, uri, repository.clone()))
        })
        .collect()
}

fn images(response: &Value) -> Result<(Vec<Resource>, Vec<Resource>), String> {
    let details = array(response, "imageDetails")?;
    let mut images = Vec::new();
    let mut tags = Vec::new();
    for image in details {
        let digest = string(image, "imageDigest")?;
        let image_tags = optional_array(image, "imageTags")?;
        let labels = image_tags
            .iter()
            .map(|tag| {
                tag.as_str()
                    .ok_or("AWS response image tag must be a string")
            })
            .collect::<Result<Vec<_>, _>>()?;
        let pushed = image_timestamp(image)?;
        images.push(resource(
            digest,
            digest,
            if labels.is_empty() {
                pushed
            } else {
                labels.join(", ")
            },
            image.clone(),
        ));
        for tag in labels {
            tags.push(resource(
                format!("{digest}:{tag}"),
                tag,
                digest,
                json!({"digest": digest, "tag": tag, "image": image}),
            ));
        }
    }
    Ok((images, tags))
}

fn image_timestamp(image: &Value) -> Result<String, String> {
    match image.get("imagePushedAt") {
        None | Some(Value::Null) => Ok(String::new()),
        Some(Value::String(timestamp)) => Ok(timestamp.clone()),
        Some(Value::Number(timestamp)) => Ok(timestamp.to_string()),
        Some(_) => Err("AWS response `imagePushedAt` must be a timestamp".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_image_digests_and_all_tags() {
        let response: Value =
            serde_json::from_str(include_str!("../../tests/fixtures/aws_ecr.json")).unwrap();
        let (images, tags) = images(&response).unwrap();
        assert_eq!(images.len(), 2);
        assert_eq!(images[0].summary, "1767323045");
        assert_eq!(tags.len(), 2);
        assert_eq!(tags[0].id, "sha256:aaa:v1");
    }

    #[test]
    fn rejects_invalid_image_shapes() {
        assert!(images(&json!({"imageDetails": [{"imageTags": "bad"}]})).is_err());
    }
}
