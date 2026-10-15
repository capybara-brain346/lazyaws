use std::{
    io::{self, Read},
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

use serde_json::Value;

use crate::config::{
    CLI_CONNECT_TIMEOUT_SECS, CLI_MAX_ATTEMPTS, CLI_READ_TIMEOUT_SECS, COMMAND_TIMEOUT, MAX_ITEMS,
    MAX_OUTPUT_BYTES, OUTPUT_BUFFER_BYTES, PROCESS_CHECK_INTERVAL, PROCESS_KILL_BIN,
};

use super::FetchContext;

#[cfg(unix)]
use std::os::unix::process::CommandExt;

impl FetchContext<'_> {
    pub(super) fn invoke(
        &self,
        logs: &mut Vec<String>,
        command: &[String],
        paginated: bool,
    ) -> Result<Value, String> {
        self.invoke_region(logs, command, paginated, None)
    }

    pub(super) fn invoke_region(
        &self,
        logs: &mut Vec<String>,
        command: &[String],
        paginated: bool,
        region: Option<&str>,
    ) -> Result<Value, String> {
        let remaining = match self.remaining() {
            Ok(remaining) => remaining,
            Err(error) => {
                self.log_timeout_once(logs);
                return Err(error);
            }
        };
        let mut args = Vec::new();
        if let Some(profile) = self.client.config.profile.as_deref() {
            args.extend(["--profile".into(), profile.into()]);
        }
        if let Some(region) = region.or(self.client.config.region.as_deref()) {
            args.extend(["--region".into(), region.into()]);
        }
        args.extend([
            "--cli-connect-timeout".into(),
            CLI_CONNECT_TIMEOUT_SECS.to_string(),
            "--cli-read-timeout".into(),
            CLI_READ_TIMEOUT_SECS.to_string(),
            "--no-cli-pager".into(),
            "--no-cli-auto-prompt".into(),
            "--output".into(),
            "json".into(),
        ]);
        args.extend(command.iter().cloned());
        if paginated {
            args.extend(["--max-items".into(), MAX_ITEMS.to_string()]);
        }

        let printable = display_command(&self.client.config.aws_bin, &args);
        match run_process(
            &self.client.config.aws_bin,
            &args,
            &self.client.cancel,
            COMMAND_TIMEOUT.min(remaining),
            MAX_OUTPUT_BYTES,
        )
        .and_then(|stdout| {
            let output = std::str::from_utf8(&stdout)
                .map_err(|error| format!("AWS CLI returned invalid UTF-8: {error}"))?;
            let value = if output.trim().is_empty() {
                Value::Object(Default::default())
            } else {
                serde_json::from_str(output)
                    .map_err(|error| format!("AWS CLI returned invalid JSON: {error}"))?
            };
            if !value.is_object() {
                return Err("AWS CLI JSON output must be an object".into());
            }
            validate_response(command, &value)?;
            Ok(value)
        }) {
            Ok(value) => {
                logs.push(format!("{printable} [ok]"));
                if has_more(&value) {
                    logs.push(format!("notice: {printable} reached the {MAX_ITEMS} item limit; results are incomplete"));
                }
                Ok(value)
            }
            Err(error) => {
                logs.push(format!("{printable} [failed: {error}]"));
                Err(error)
            }
        }
    }
}

fn validate_response(command: &[String], response: &Value) -> Result<(), String> {
    if command.first().map(String::as_str) != Some("ecr")
        || command.get(1).map(String::as_str) != Some("batch-delete-image")
    {
        return Ok(());
    }

    let failures = match response.get("failures") {
        None => return Ok(()),
        Some(Value::Array(failures)) => failures,
        Some(_) => return Err("AWS response `failures` must be an array".into()),
    };
    let failures = failures
        .iter()
        .map(|failure| {
            let image = failure
                .get("imageId")
                .ok_or("AWS response image failure is missing `imageId`")?;
            let id = image
                .get("imageTag")
                .or_else(|| image.get("imageDigest"))
                .and_then(Value::as_str)
                .unwrap_or("unknown image");
            let reason = failure
                .get("failureReason")
                .and_then(Value::as_str)
                .unwrap_or("unknown reason");
            let code = failure
                .get("failureCode")
                .and_then(Value::as_str)
                .unwrap_or("unknown code");
            Ok(format!("{id}: {code}: {reason}"))
        })
        .collect::<Result<Vec<_>, &str>>()?;
    if failures.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "ECR could not delete image(s): {}",
            failures.join("; ")
        ))
    }
}

fn run_process(
    binary: &str,
    args: &[String],
    cancel: &Arc<AtomicBool>,
    timeout: Duration,
    max_output: u64,
) -> Result<Vec<u8>, String> {
    if cancel.load(Ordering::Relaxed) {
        return Err("cancelled".into());
    }

    let start = Instant::now();
    let mut command = Command::new(binary);
    command
        .args(args)
        .env("AWS_PAGER", "")
        .env("AWS_MAX_ATTEMPTS", CLI_MAX_ATTEMPTS.to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(unix)]
    command.process_group(0);
    let mut child = command
        .spawn()
        .map_err(|error| format!("cannot start AWS CLI: {error}"))?;
    let pid = child.id();

    let used = Arc::new(AtomicU64::new(0));
    let output_exceeded = Arc::new(AtomicBool::new(false));
    let stdout = read_thread(
        child.stdout.take().expect("piped stdout"),
        used.clone(),
        output_exceeded.clone(),
        max_output,
    )
    .map_err(|error| {
        stop_child(&mut child);
        format!("cannot start AWS CLI output reader: {error}")
    })?;
    let stderr = match read_thread(
        child.stderr.take().expect("piped stderr"),
        used,
        output_exceeded.clone(),
        max_output,
    ) {
        Ok(reader) => reader,
        Err(error) => {
            stop_child(&mut child);
            let _ = join_reader(stdout);
            return Err(format!("cannot start AWS CLI output reader: {error}"));
        }
    };
    let status = loop {
        if output_exceeded.load(Ordering::Relaxed) {
            stop_child(&mut child);
            let _ = join_reader(stdout);
            let _ = join_reader(stderr);
            return Err(format!("AWS CLI output exceeded {max_output} bytes"));
        }
        if cancel.load(Ordering::Relaxed) {
            stop_child(&mut child);
            let _ = join_reader(stdout);
            let _ = join_reader(stderr);
            return Err("cancelled".into());
        }
        if start.elapsed() >= timeout {
            stop_child(&mut child);
            let _ = join_reader(stdout);
            let _ = join_reader(stderr);
            return Err(format!(
                "AWS CLI timed out after {} ms",
                timeout.as_millis()
            ));
        }
        match child.try_wait() {
            Ok(Some(status)) => {
                if !stdout.is_finished() || !stderr.is_finished() {
                    kill_process_group(pid);
                }
                break status;
            }
            Ok(None) => thread::sleep(PROCESS_CHECK_INTERVAL),
            Err(error) => {
                stop_child(&mut child);
                let _ = join_reader(stdout);
                let _ = join_reader(stderr);
                return Err(format!("cannot wait for AWS CLI: {error}"));
            }
        }
    };

    let stdout = join_reader(stdout)?;
    let stderr = join_reader(stderr)?;
    if stdout.failed || stderr.failed {
        return Err("cannot read AWS CLI output".into());
    }
    if stdout.exceeded || stderr.exceeded {
        return Err(format!("AWS CLI output exceeded {max_output} bytes"));
    }
    if !status.success() {
        let detail = String::from_utf8_lossy(&stderr.bytes);
        let detail = detail.trim();
        return Err(if detail.is_empty() {
            format!("AWS CLI exited with {status}")
        } else {
            format!("AWS CLI exited with {status}: {detail}")
        });
    }

    Ok(stdout.bytes)
}

fn stop_child(child: &mut std::process::Child) {
    kill_process_group(child.id());
    let _ = child.kill();
    let _ = child.wait();
}

#[cfg(unix)]
fn kill_process_group(pid: u32) {
    let group = format!("-{pid}");
    let _ = Command::new(PROCESS_KILL_BIN)
        .args(["-KILL", "--", group.as_str()])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

#[cfg(not(unix))]
// shortcut: Windows stops only the direct child; add Job Objects if wrappers need descendant cleanup.
fn kill_process_group(_pid: u32) {}

struct Captured {
    bytes: Vec<u8>,
    exceeded: bool,
    failed: bool,
}

fn read_thread<R: Read + Send + 'static>(
    mut reader: R,
    used: Arc<AtomicU64>,
    output_exceeded: Arc<AtomicBool>,
    max_output: u64,
) -> io::Result<thread::JoinHandle<Captured>> {
    thread::Builder::new().spawn(move || {
        let mut captured = Captured {
            bytes: Vec::new(),
            exceeded: false,
            failed: false,
        };
        let mut buffer = [0; OUTPUT_BUFFER_BYTES];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(count) => {
                    let before = used.fetch_add(count as u64, Ordering::Relaxed);
                    let keep = max_output.saturating_sub(before).min(count as u64) as usize;
                    captured.bytes.extend_from_slice(&buffer[..keep]);
                    captured.exceeded |= keep < count;
                    if keep < count {
                        output_exceeded.store(true, Ordering::Relaxed);
                    }
                }
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(_) => {
                    captured.failed = true;
                    break;
                }
            }
        }
        captured
    })
}

fn join_reader(reader: thread::JoinHandle<Captured>) -> Result<Captured, String> {
    reader
        .join()
        .map_err(|_| "AWS CLI output reader stopped unexpectedly".into())
}

fn display_command(binary: &str, args: &[String]) -> String {
    std::iter::once(binary)
        .chain(args.iter().map(String::as_str))
        .map(|part| format!("{part:?}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn has_more(value: &Value) -> bool {
    ["NextToken", "nextToken", "ContinuationToken"]
        .iter()
        .any(|field| {
            value
                .get(field)
                .and_then(Value::as_str)
                .is_some_and(|token| !token.is_empty())
        })
        || value.get("IsTruncated").and_then(Value::as_bool) == Some(true)
}

#[cfg(all(test, unix))]
#[path = "process/tests.rs"]
mod tests;
