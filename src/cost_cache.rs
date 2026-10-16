//! The last Cost tab result, kept on disk with no expiry. Each Cost Explorer load is
//! charged, so a saved result stays until the user confirms a refresh or deletes the file.

use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use serde_json::{Value, json};

use crate::{
    app::App,
    config::{
        COST_CACHE_DIR, COST_CACHE_FILE_PREFIX, COST_CACHE_FORMAT, COST_CACHE_HOME_LINUX,
        COST_CACHE_HOME_MACOS,
    },
    model::{PANEL_COUNT, Resource, Service, Snapshot},
};

#[cfg(unix)]
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};

#[derive(Clone, Debug)]
pub struct CostCache {
    /// When each panel was fetched. A panel that never loaded has no time.
    pub fetched_at: [Option<SystemTime>; PANEL_COUNT],
    pub snapshot: Snapshot,
}

impl CostCache {
    /// When AWS last denied the forecast, in seconds since the Unix epoch. The Summary
    /// forecast row keeps it, so it is saved and loaded with the rest of the result.
    pub fn forecast_denied_at(&self) -> Option<u64> {
        let detail = &self.snapshot.panels[0]
            .items
            .iter()
            .find(|item| item.id == "forecast")?
            .detail;
        detail.get("permissionDenied")?.as_bool()?.then_some(())?;
        detail.get("deniedAt")?.as_u64()
    }

    /// The age of the oldest panel, so the shown age is never too young.
    pub fn age(&self, now: SystemTime) -> Duration {
        let oldest = self.fetched_at.iter().flatten().min();
        oldest.map_or(Duration::ZERO, |time| {
            now.duration_since(*time).unwrap_or_default()
        })
    }
}

pub fn age_text(age: Duration) -> String {
    let minutes = age.as_secs() / 60;
    match minutes {
        0 => "just now".into(),
        1..60 => format!("{minutes} min ago"),
        60..1440 => format!("{} h ago", minutes / 60),
        _ => format!("{} d ago", minutes / 1440),
    }
}

/// `XDG_CACHE_HOME` when set, otherwise the platform cache directory in `HOME`.
pub fn directory(env: impl Fn(&str) -> Option<String>) -> Option<PathBuf> {
    let base = env("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| {
            let home = PathBuf::from(env("HOME").filter(|home| !home.is_empty())?);
            Some(home.join(if cfg!(target_os = "macos") {
                COST_CACHE_HOME_MACOS
            } else {
                COST_CACHE_HOME_LINUX
            }))
        })?;
    Some(base.join(COST_CACHE_DIR))
}

/// Loads the saved result once the account is known. A bad file is skipped, not fatal.
pub fn restore(app: &mut App, dir: Option<&Path>) -> bool {
    let (Some(dir), Some(Ok(account))) = (dir, &app.account) else {
        return false;
    };
    match load(dir, &account.account) {
        Ok(cache) => {
            if app.cost_cache.is_none() {
                app.cost_cache = cache;
            }
        }
        Err(error) => app.append_logs([format!("Cost cache ignored: {error}")]),
    }
    true
}

/// Keeps a complete Cost result in memory, and on disk when a cache directory is set.
pub fn store(app: &mut App, dir: Option<&Path>, now: SystemTime) {
    let Some(cache) = app.cache_cost_snapshot(now).cloned() else {
        return;
    };
    let (Some(dir), Some(Ok(account))) = (dir, &app.account) else {
        return;
    };
    let result = save(dir, &account.account, &cache);
    if let Err(error) = result {
        app.append_logs([format!("Cost cache not saved: {error}")]);
    }
}

pub fn load(dir: &Path, account: &str) -> Result<Option<CostCache>, String> {
    let path = file_path(dir, account)?;
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("{}: {error}", path.display())),
    };
    let value = serde_json::from_str(&text)
        .map_err(|error| format!("{}: invalid JSON: {error}", path.display()))?;
    decode(&value, account)
        .map(Some)
        .map_err(|error| format!("{}: {error}", path.display()))
}

pub fn save(dir: &Path, account: &str, cache: &CostCache) -> Result<(), String> {
    let path = file_path(dir, account)?;
    let staged = path.with_extension("json.tmp");
    let fail = |error: io::Error| format!("{}: {error}", path.display());

    let mut builder = fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    builder.mode(0o700);
    builder.create(dir).map_err(fail)?;

    let mut options = OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut file = options.open(&staged).map_err(fail)?;
    file.write_all(encode(account, cache).to_string().as_bytes())
        .and_then(|()| file.sync_all())
        .map_err(fail)?;
    // Rename replaces the old file in one step, so a crash never leaves a partial cache.
    fs::rename(&staged, &path).map_err(fail)
}

fn file_path(dir: &Path, account: &str) -> Result<PathBuf, String> {
    if account.is_empty() || !account.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(format!(
            "Cost cache needs a numeric account ID, not `{account}`"
        ));
    }
    Ok(dir.join(format!("{COST_CACHE_FILE_PREFIX}{account}.json")))
}

fn encode(account: &str, cache: &CostCache) -> Value {
    let fetched_at = cache
        .fetched_at
        .iter()
        .map(|time| {
            time.map(|time| {
                time.duration_since(UNIX_EPOCH)
                    .map_or(0, |elapsed| elapsed.as_secs())
            })
        })
        .collect::<Vec<_>>();
    let panels = cache
        .snapshot
        .panels
        .iter()
        .map(|panel| {
            panel
                .items
                .iter()
                .map(|item| {
                    json!({
                        "id": item.id,
                        "label": item.label,
                        "summary": item.summary,
                        "detail": item.detail,
                    })
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    json!({
        "format": COST_CACHE_FORMAT,
        "account": account,
        "fetchedAt": fetched_at,
        "panels": panels,
    })
}

fn decode(value: &Value, account: &str) -> Result<CostCache, String> {
    if value.get("format").and_then(Value::as_u64) != Some(COST_CACHE_FORMAT) {
        return Err("unsupported cache format".into());
    }
    if value.get("account").and_then(Value::as_str) != Some(account) {
        return Err("cache belongs to another account".into());
    }
    let times = value
        .get("fetchedAt")
        .and_then(Value::as_array)
        .filter(|times| times.len() == PANEL_COUNT)
        .ok_or("cache must have one fetch time for each panel")?;
    let mut fetched_at = [None; PANEL_COUNT];
    for (slot, time) in fetched_at.iter_mut().zip(times) {
        *slot = match time {
            Value::Null => None,
            _ => Some(UNIX_EPOCH + Duration::from_secs(time.as_u64().ok_or("bad fetch time")?)),
        };
    }
    let panels = value
        .get("panels")
        .and_then(Value::as_array)
        .filter(|panels| panels.len() == PANEL_COUNT)
        .ok_or("cache must have one list for each panel")?;

    if fetched_at.iter().all(Option::is_none) {
        return Err("cache has no fetch time".into());
    }

    let mut snapshot = Snapshot::empty(Service::Cost);
    for (panel, items) in snapshot.panels.iter_mut().zip(panels) {
        panel.items = items
            .as_array()
            .ok_or("cache panel must be a list")?
            .iter()
            .map(decode_item)
            .collect::<Result<_, _>>()?;
    }
    Ok(CostCache {
        fetched_at,
        snapshot,
    })
}

fn decode_item(item: &Value) -> Result<Resource, String> {
    let text = |field: &str| {
        item.get(field)
            .and_then(Value::as_str)
            .map(str::to_owned)
            .ok_or_else(|| format!("cache row is missing `{field}`"))
    };
    Ok(Resource {
        id: text("id")?,
        label: text("label")?,
        summary: text("summary")?,
        detail: item.get("detail").cloned().unwrap_or(Value::Null),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::demo;

    #[test]
    fn saved_cost_survives_reload_and_is_scoped_to_its_account() {
        let dir = std::env::temp_dir().join(format!("lazyaws-cost-cache-{}", std::process::id()));
        let cache = CostCache {
            fetched_at: [
                Some(UNIX_EPOCH + Duration::from_secs(1_760_000_000)),
                None,
                None,
                None,
            ],
            snapshot: demo::snapshot(Service::Cost, None),
        };

        assert!(load(&dir, "111111111111").unwrap().is_none());
        save(&dir, "111111111111", &cache).unwrap();
        let loaded = load(&dir, "111111111111").unwrap().unwrap();
        assert_eq!(loaded.fetched_at, cache.fetched_at);
        assert_eq!(
            loaded.snapshot.panels[0].items[0].summary,
            cache.snapshot.panels[0].items[0].summary
        );
        assert_eq!(
            loaded.snapshot.panels[2].items.len(),
            cache.snapshot.panels[2].items.len()
        );

        assert!(load(&dir, "222222222222").unwrap().is_none());
        fs::copy(
            dir.join("cost-111111111111.json"),
            dir.join("cost-222222222222.json"),
        )
        .unwrap();
        assert!(load(&dir, "222222222222").is_err());
        assert!(save(&dir, "../escape", &cache).is_err());
        fs::remove_dir_all(dir).unwrap();
    }
}
