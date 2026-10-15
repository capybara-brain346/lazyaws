use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use crate::config::DOWNLOAD_STAGE_ATTEMPTS;

static NEXT_DOWNLOAD: AtomicU64 = AtomicU64::new(0);

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

    use super::create_private_dir;

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
