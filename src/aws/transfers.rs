use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use serde_json::Value;

use crate::config::DOWNLOAD_STAGE_ATTEMPTS;

static NEXT_DOWNLOAD: AtomicU64 = AtomicU64::new(0);
static NEXT_INVOKE: AtomicU64 = AtomicU64::new(0);

pub(crate) const INVOKE_OUTPUT_PLACEHOLDER: &str = "<invoke-output-file>";
const INVOKE_PAYLOAD_LIMIT_BYTES: u64 = 8 * 1024 * 1024;

pub(super) struct DownloadStage {
    directory: PathBuf,
    output: PathBuf,
    destination: PathBuf,
}

impl DownloadStage {
    pub(super) fn new(destination: &str) -> Result<Self, String> {
        let destination = Path::new(destination);
        if destination.file_name().is_none() {
            return Err("Download path must name a file".into());
        }
        match fs::symlink_metadata(destination) {
            Ok(_) => return Err("Download path already exists".into()),
            Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
                return Err(format!("cannot check download path: {error}"));
            }
            Err(_) => {}
        }
        let parent = destination
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        if !parent.is_dir() {
            return Err("Download folder does not exist".into());
        }

        let name = destination
            .file_name()
            .ok_or("Download path must name a file")?;
        let parent = fs::canonicalize(parent)
            .map_err(|error| format!("cannot open download folder: {error}"))?;
        let destination = parent.join(name);
        for _ in 0..DOWNLOAD_STAGE_ATTEMPTS {
            let directory = parent.join(format!(
                ".lazyaws-download-{}-{}",
                std::process::id(),
                NEXT_DOWNLOAD.fetch_add(1, Ordering::Relaxed)
            ));
            match create_private_dir(&directory) {
                Ok(()) => {
                    let output = directory.join(name);
                    return Ok(Self {
                        directory,
                        output,
                        destination,
                    });
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => {
                    return Err(format!("cannot create download staging folder: {error}"));
                }
            }
        }
        Err("cannot create a unique download staging folder".into())
    }

    pub(super) fn output_path(&self) -> Result<String, String> {
        self.output
            .to_str()
            .map(str::to_string)
            .ok_or("Download path must be valid UTF-8".into())
    }

    pub(super) fn commit(self) -> Result<(), String> {
        if !self.output.is_file() {
            return Err("AWS CLI did not create the downloaded file".into());
        }
        fs::hard_link(&self.output, &self.destination)
            .map_err(|error| format!("cannot save download without overwriting: {error}"))
    }
}

impl Drop for DownloadStage {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.directory);
    }
}

pub(super) struct InvokeOutput {
    directory: PathBuf,
    output: PathBuf,
}

impl InvokeOutput {
    pub(super) fn new() -> Result<Self, String> {
        let parent = std::env::temp_dir();
        for _ in 0..DOWNLOAD_STAGE_ATTEMPTS {
            let directory = parent.join(format!(
                ".lazyaws-invoke-{}-{}",
                std::process::id(),
                NEXT_INVOKE.fetch_add(1, Ordering::Relaxed)
            ));
            match create_private_dir(&directory) {
                Ok(()) => {
                    let output = directory.join("response-payload.json");
                    return Ok(Self { directory, output });
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => {
                    return Err(format!("cannot create invoke staging folder: {error}"));
                }
            }
        }
        Err("cannot create a unique invoke staging folder".into())
    }

    pub(super) fn output_path(&self) -> Result<String, String> {
        self.output
            .to_str()
            .map(str::to_string)
            .ok_or("Invoke output path must be valid UTF-8".into())
    }

    pub(super) fn into_response(self, mut response: Value) -> Result<Value, String> {
        let payload = self.read_payload()?;
        if let Some(object) = response.as_object_mut() {
            object.insert("Payload".into(), payload.clone());
            if let Some(encoded) = object
                .get("LogResult")
                .and_then(Value::as_str)
                .map(str::to_string)
                && let Some(decoded) = decode_base64(&encoded)
            {
                object.insert("LogResultDecoded".into(), Value::String(decoded));
            }
            if let Some(error) = object
                .get("FunctionError")
                .and_then(Value::as_str)
                .filter(|error| !error.is_empty())
            {
                let excerpt = payload_excerpt(&payload);
                return Err(format!("Lambda function error ({error}): {excerpt}"));
            }
        }
        Ok(response)
    }

    fn read_payload(&self) -> Result<Value, String> {
        let bytes = match fs::read(&self.output) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Value::Null),
            Err(error) => return Err(format!("cannot read invoke output: {error}")),
        };
        if bytes.len() as u64 > INVOKE_PAYLOAD_LIMIT_BYTES {
            return Err(format!(
                "Lambda payload exceeded {} bytes",
                INVOKE_PAYLOAD_LIMIT_BYTES
            ));
        }
        if bytes.is_empty() {
            return Ok(Value::Null);
        }
        let text = String::from_utf8_lossy(&bytes);
        match serde_json::from_str::<Value>(&text) {
            Ok(value) => Ok(value),
            Err(_) => Ok(Value::String(text.into_owned())),
        }
    }
}

impl Drop for InvokeOutput {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.directory);
    }
}

fn payload_excerpt(payload: &Value) -> String {
    let text = match payload {
        Value::Null => "empty payload".to_string(),
        Value::String(text) if text.is_empty() => "empty payload".to_string(),
        Value::String(text) => text.clone(),
        _ => serde_json::to_string(payload).unwrap_or_else(|_| "unreadable payload".into()),
    };
    let mut excerpt: String = text.chars().take(1000).collect();
    if text.chars().count() > 1000 {
        excerpt.push_str("…[truncated]");
    }
    excerpt
}

fn decode_base64(encoded: &str) -> Option<String> {
    let mut bits = 0u32;
    let mut width = 0u8;
    let mut bytes = Vec::new();
    for character in encoded.chars().filter(|c| !c.is_whitespace()) {
        if character == '=' {
            break;
        }
        let value = match character {
            'A'..='Z' => character as u32 - 'A' as u32,
            'a'..='z' => character as u32 - 'a' as u32 + 26,
            '0'..='9' => character as u32 - '0' as u32 + 52,
            '+' => 62,
            '/' => 63,
            _ => return None,
        };
        bits = (bits << 6) | value;
        width += 6;
        if width >= 8 {
            width -= 8;
            bytes.push((bits >> width) as u8);
            bits &= (1u32 << width).saturating_sub(1);
        }
    }
    String::from_utf8(bytes).ok()
}

#[cfg(unix)]
fn create_private_dir(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::DirBuilderExt;

    let mut builder = fs::DirBuilder::new();
    builder.mode(0o700);
    builder.create(path)
}

#[cfg(not(unix))]
fn create_private_dir(path: &Path) -> std::io::Result<()> {
    fs::create_dir(path)
}

#[cfg(all(test, unix))]
mod tests {
    use std::{fs, os::unix::fs::symlink};

    use super::*;

    #[test]
    fn staging_directory_does_not_follow_or_remove_a_symlink_collision() {
        let root =
            std::env::temp_dir().join(format!("lazyaws-download-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir(&root).unwrap();
        let target = root.join("target");
        fs::create_dir(&target).unwrap();
        let collision = root.join("staging");
        symlink(&target, &collision).unwrap();

        let error = create_private_dir(&collision).unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::AlreadyExists);
        assert!(collision.is_symlink());
        assert!(fs::read_dir(&target).unwrap().next().is_none());

        fs::remove_file(collision).unwrap();
        fs::remove_dir_all(root).unwrap();
    }
}
