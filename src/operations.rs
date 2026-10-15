mod catalog;
pub use catalog::available;

use std::{fs, path::Path};

use serde_json::{Value, json};

use crate::config::S3_MAX_UPLOAD_BYTES;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AccountContext {
    pub account: String,
    pub arn: String,
    pub user_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Operation {
    pub label: &'static str,
    pub service: &'static str,
    pub name: &'static str,
    pub input: Value,
    pub region: Option<String>,
    pub target: String,
    pub warning: &'static str,
    pub parameter: Option<&'static str>,
}

impl Operation {
    pub fn args(&self, value: &str) -> Result<Vec<String>, String> {
        let mut input = self.input.clone();
        let mut extra = Vec::new();

        match self.parameter {
            Some("Desired task count") => {
                if self.service != "ecs" || self.name != "update-service" {
                    return Err("Desired task count is only valid for ECS service updates".into());
                }
                let count = value
                    .trim()
                    .parse::<i32>()
                    .map_err(|_| "Desired task count must be a non-negative integer".to_string())?;
                if count < 0 {
                    return Err("Desired task count must be a non-negative integer".into());
                }
                input
                    .as_object_mut()
                    .ok_or("Operation input must be a JSON object")?
                    .insert("desiredCount".into(), json!(count));
            }
            Some("Upload file path") => {
                if self.service != "s3api" || self.name != "put-object" {
                    return Err("Upload file path is only valid for S3 object uploads".into());
                }
                let path = upload_path(value)?;
                if !input.get("Key").is_some_and(|key| key.as_str().is_some()) {
                    let name = path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .filter(|name| !name.is_empty())
                        .ok_or("Upload file name must be valid UTF-8")?;
                    input
                        .as_object_mut()
                        .ok_or("Operation input must be a JSON object")?
                        .insert("Key".into(), Value::String(name.into()));
                }
                extra.extend(["--body".into(), path_string(&path)?]);
            }
            Some("Download file path") => {
                if self.service != "s3api" || self.name != "get-object" {
                    return Err("Download file path is only valid for S3 object downloads".into());
                }
                let path = download_path(value)?;
                extra.push(path_string(&path)?);
            }
            Some(_) => return Err("Unknown operation parameter".into()),
            None => {}
        }

        let input = serde_json::to_string(&input)
            .map_err(|error| format!("cannot serialize AWS input: {error}"))?;
        let mut args = vec![
            self.service.into(),
            self.name.into(),
            "--cli-input-json".into(),
            input,
        ];
        args.extend(extra);
        Ok(args)
    }
}

fn upload_path(value: &str) -> Result<std::path::PathBuf, String> {
    if value.is_empty() {
        return Err("Upload file path must not be empty".into());
    }
    let path = fs::canonicalize(value)
        .map_err(|error| format!("cannot open upload file `{value}`: {error}"))?;
    if !path.is_file() {
        return Err("Upload path must be a regular file".into());
    }
    if fs::metadata(&path)
        .map_err(|error| format!("cannot read upload file metadata: {error}"))?
        .len()
        > S3_MAX_UPLOAD_BYTES
    {
        return Err("Upload file exceeds the 5 GB single-request limit".into());
    }
    Ok(path)
}

fn download_path(value: &str) -> Result<std::path::PathBuf, String> {
    if value.is_empty() {
        return Err("Download file path must not be empty".into());
    }
    let requested = Path::new(value);
    let name = requested
        .file_name()
        .filter(|name| !name.is_empty())
        .ok_or("Download path must name a file")?;
    let parent = requested
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let parent = fs::canonicalize(parent)
        .map_err(|error| format!("cannot open download folder: {error}"))?;
    let destination = parent.join(name);
    match fs::symlink_metadata(&destination) {
        Ok(_) => return Err("Download path already exists".into()),
        Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
            return Err(format!("cannot check download path: {error}"));
        }
        Err(_) => {}
    }
    Ok(destination)
}

fn path_string(path: &Path) -> Result<String, String> {
    path.to_str()
        .map(str::to_string)
        .ok_or("File path must be valid UTF-8".into())
}
