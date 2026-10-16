use std::{
    io::{self, IsTerminal},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, Sender},
    },
    thread::{self, JoinHandle},
    time::{Instant, SystemTime},
};

use crossterm::{
    clipboard::CopyToClipboard,
    event::{self, DisableMouseCapture, EnableMouseCapture, Event},
    execute,
};
use serde_json::{Value, json};

use crate::{
    app::{Action, App},
    aws::{self, AwsCli},
    config::{Config, EVENT_WAIT, POLL_INTERVAL},
    cost_cache, demo,
    model::{Service, Snapshot},
    operations::{AccountContext, Operation},
    ui,
};

mod selection;

use selection::TextSelection;

struct Request {
    generation: u64,
    cancel: Arc<AtomicBool>,
    kind: RequestKind,
}

enum RequestKind {
    Refresh {
        service: Service,
        selected: Option<String>,
        forecast_denied_at: Option<u64>,
    },
    Execute {
        operation: Operation,
        value: String,
    },
}

enum Response {
    Refresh {
        generation: u64,
        service: Service,
        result: Result<Box<Snapshot>, String>,
        identity: Result<AccountContext, String>,
        logs: Vec<String>,
    },
    Execute {
        operation: Operation,
        result: Result<Value, String>,
        logs: Vec<String>,
        demo: bool,
    },
}

struct Worker {
    requests: Option<Sender<Request>>,
    responses: Receiver<Response>,
    cancel: Arc<AtomicBool>,
    operation_cancel: Option<Arc<AtomicBool>>,
    thread: Option<JoinHandle<()>>,
}

impl Worker {
    fn start(config: Config) -> io::Result<Self> {
        let (requests, incoming) = mpsc::channel::<Request>();
        let (outgoing, responses) = mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));

        let thread = thread::Builder::new()
            .name("aws-cli".into())
            .spawn(move || {
                let mut pending = None;
                let mut identity = None;
                while let Some(mut request) = pending.take().or_else(|| incoming.recv().ok()) {
                    if matches!(request.kind, RequestKind::Refresh { .. }) {
                        while let Ok(newer) = incoming.try_recv() {
                            match &newer.kind {
                                RequestKind::Refresh { .. } => {
                                    request = newer;
                                }
                                RequestKind::Execute { .. } => {
                                    pending = Some(newer);
                                    break;
                                }
                            }
                        }
                    }

                    if request.cancel.load(Ordering::Relaxed) {
                        continue;
                    }

                    let response = match request.kind {
                        RequestKind::Refresh {
                            service,
                            selected,
                            forecast_denied_at,
                        } => {
                            let mut logs = Vec::new();
                            let client = AwsCli::new(config.clone(), Arc::clone(&request.cancel));
                            let account = if config.demo {
                                Ok(demo_account())
                            } else if let Some(account) = &identity {
                                Ok(AccountContext::clone(account))
                            } else {
                                client.identity(&mut logs)
                            };
                            if let Ok(account) = &account {
                                identity = Some(account.clone());
                            }
                            let result = if config.demo {
                                Ok(demo::snapshot(service, selected.as_deref()))
                            } else {
                                client.fetch_with(service, selected.as_deref(), forecast_denied_at)
                            };
                            Response::Refresh {
                                generation: request.generation,
                                service,
                                result: result.map(Box::new),
                                identity: account,
                                logs,
                            }
                        }
                        RequestKind::Execute { operation, value } => {
                            let mut logs = Vec::new();
                            let result = if config.demo {
                                operation.args(&value).map(|_| {
                                    json!({
                                        "simulated": true,
                                        "service": operation.service,
                                        "operation": operation.name,
                                        "target": operation.target,
                                    })
                                })
                            } else {
                                AwsCli::new(config.clone(), Arc::clone(&request.cancel))
                                    .execute(&operation, &value, &mut logs)
                            };
                            Response::Execute {
                                operation,
                                result,
                                logs,
                                demo: config.demo,
                            }
                        }
                    };

                    if outgoing.send(response).is_err() {
                        break;
                    }
                }
            })?;

        Ok(Self {
            requests: Some(requests),
            responses,
            cancel,
            operation_cancel: None,
            thread: Some(thread),
        })
    }

    fn request(&mut self, app: &mut App, generation: &mut u64) -> io::Result<()> {
        if self.operation_cancel.is_none() {
            self.cancel.store(true, Ordering::Relaxed);
        }
        self.cancel = Arc::new(AtomicBool::new(false));
        *generation = generation.wrapping_add(1);

        let request = Request {
            generation: *generation,
            cancel: Arc::clone(&self.cancel),
            kind: RequestKind::Refresh {
                service: app.service,
                selected: app.selected_primary().map(String::from),
                forecast_denied_at: app
                    .cost_cache
                    .as_ref()
                    .and_then(|cache| cache.forecast_denied_at()),
            },
        };

        self.requests
            .as_ref()
            .ok_or_else(|| io::Error::other("CLI worker stopped"))?
            .send(request)
            .map_err(|_| io::Error::other("CLI worker stopped"))?;
        app.loading = true;
        Ok(())
    }

    fn execute(
        &mut self,
        operation: Operation,
        value: String,
        generation: &mut u64,
    ) -> io::Result<()> {
        if self.operation_cancel.is_some() {
            return Err(io::Error::other("an AWS operation is already running"));
        }
        self.cancel.store(true, Ordering::Relaxed);
        *generation = generation.wrapping_add(1);
        let operation_cancel = Arc::new(AtomicBool::new(false));
        self.operation_cancel = Some(Arc::clone(&operation_cancel));
        let request = Request {
            generation: *generation,
            cancel: operation_cancel,
            kind: RequestKind::Execute { operation, value },
        };
        self.requests
            .as_ref()
            .ok_or_else(|| io::Error::other("CLI worker stopped"))?
            .send(request)
            .map_err(|_| io::Error::other("CLI worker stopped"))
    }

    fn operation_finished(&mut self) {
        self.operation_cancel = None;
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
        if let Some(cancel) = &self.operation_cancel {
            cancel.store(true, Ordering::Relaxed);
        }
        self.requests.take();

        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

struct TerminalGuard {
    mouse_capture: bool,
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        if self.mouse_capture {
            let _ = execute!(io::stdout(), DisableMouseCapture);
        }
        ratatui::restore();
    }
}

pub fn run(config: Config) -> io::Result<()> {
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err(io::Error::other(
            "A terminal is required. Use --help for options.",
        ));
    }

    let mut worker = Worker::start(config.clone())?;
    let mut guard = TerminalGuard {
        mouse_capture: false,
    };
    let mut terminal = ratatui::try_init()?;
    guard.mouse_capture = true;
    execute!(io::stdout(), EnableMouseCapture)?;
    let mut app = App::new();
    let mut generation = 0;
    worker.request(&mut app, &mut generation)?;
    let mut last_refresh = Instant::now();
    let mut selection = TextSelection::default();
    let mut hit_map = ui::HitMap::default();
    // Demo data never touches the saved Cost result.
    let cost_dir = cost_cache::directory(|name| std::env::var(name).ok()).filter(|_| !config.demo);
    let mut cost_restored = false;

    loop {
        while let Ok(response) = worker.responses.try_recv() {
            match response {
                Response::Refresh {
                    generation: response_generation,
                    service,
                    result,
                    identity,
                    logs,
                } => {
                    app.append_logs(logs);
                    if response_generation != generation {
                        continue;
                    }
                    app.account = Some(identity);
                    last_refresh = Instant::now();
                    if !cost_restored {
                        cost_restored = cost_cache::restore(&mut app, cost_dir.as_deref());
                    }
                    // The Cost tab can open from the saved result while another service loads.
                    if service != app.service {
                        continue;
                    }

                    match result {
                        Ok(snapshot) => {
                            let cost = snapshot.service == Service::Cost;
                            app.apply(*snapshot);
                            if cost {
                                cost_cache::store(&mut app, cost_dir.as_deref(), SystemTime::now());
                            }
                        }
                        Err(error) => {
                            app.loading = false;
                            app.error = Some(error.clone());
                            app.append_logs([error]);
                        }
                    }
                }
                Response::Execute {
                    operation,
                    result,
                    logs,
                    demo,
                } => {
                    let output = operation_output(&operation, &result, demo);
                    worker.operation_finished();
                    app.executing = false;
                    app.actions = None;
                    app.operation_output = Some(output);
                    app.append_logs(logs);
                    app.append_logs([operation_log(&operation, &result, demo)]);
                    worker.request(&mut app, &mut generation)?;
                }
            }
        }

        if app.resume_cost() == Action::Refresh {
            worker.request(&mut app, &mut generation)?;
        }

        if app.service.auto_refresh()
            && !app.loading
            && app.actions.is_none()
            && !app.executing
            && last_refresh.elapsed() >= POLL_INTERVAL
        {
            worker.request(&mut app, &mut generation)?;
        }

        let screen = terminal.draw(|frame| {
            if !selection.draw(frame, app.theme()) {
                hit_map = ui::draw_interactive(frame, &app, &config);
            }
        })?;

        if event::poll(EVENT_WAIT)? {
            let action = match event::read()? {
                Event::Key(key) => {
                    selection.clear();
                    app.handle_key(key)
                }
                Event::Mouse(mouse) => {
                    selection.handle_mouse(mouse, screen.buffer, &mut app, &hit_map)
                }
                Event::Resize(_, _) | Event::FocusLost => {
                    selection.clear();
                    Action::None
                }
                _ => Action::None,
            };
            match action {
                Action::Quit => break,
                Action::Refresh => worker.request(&mut app, &mut generation)?,
                Action::Execute(operation, value) => {
                    worker.execute(operation, value, &mut generation)?;
                }
                Action::Copy(text) => {
                    if let Err(error) =
                        execute!(io::stdout(), CopyToClipboard::to_clipboard_from(text))
                    {
                        app.append_logs([format!("Clipboard copy failed: {error}")]);
                    }
                }
                Action::None => {}
            }
        }
    }

    Ok(())
}

fn demo_account() -> AccountContext {
    AccountContext {
        account: "123456789012".into(),
        arn: "arn:aws:iam::123456789012:user/lazyaws-demo".into(),
        user_id: "AIDAEXAMPLEDEMO".into(),
    }
}

fn operation_output(operation: &Operation, result: &Result<Value, String>, demo: bool) -> String {
    let mut lines = Vec::new();
    if demo {
        lines.push("Demo mode: no AWS command was run.".to_owned());
    }
    match result {
        Ok(value) => {
            lines.push(format!(
                "{} completed for {}.",
                operation.label, operation.target
            ));
            if let Ok(value) = serde_json::to_string_pretty(value) {
                lines.extend(value.lines().map(str::to_owned));
            }
        }
        Err(error) => {
            lines.push(format!("{} failed: {error}", operation.label));
            if aws::outcome_unknown(error) {
                lines.push("The outcome may be unknown. Refresh before retry.".into());
            }
        }
    }
    lines.join("\n")
}

fn operation_log(operation: &Operation, result: &Result<Value, String>, demo: bool) -> String {
    let mode = if demo { "[demo; no AWS command] " } else { "" };
    match result {
        Ok(_) => format!(
            "{mode}{} completed for {}",
            operation.label, operation.target
        ),
        Err(error) if aws::outcome_unknown(error) => format!(
            "{mode}{} timed out for {}; outcome may be unknown, refresh before retry",
            operation.label, operation.target
        ),
        Err(error) => format!("{mode}{} failed: {error}", operation.label),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demo_worker_returns_requested_context_and_shuts_down() {
        let mut worker = Worker::start(Config {
            demo: true,
            ..Config::default()
        })
        .unwrap();
        let mut app = App::new();
        let mut generation = 0;
        worker.request(&mut app, &mut generation).unwrap();
        let Response::Refresh {
            generation: response_generation,
            result,
            identity,
            ..
        } = worker.responses.recv_timeout(POLL_INTERVAL).unwrap()
        else {
            panic!("expected refresh response");
        };
        assert_eq!(response_generation, generation);
        assert!(identity.is_ok());
        let snapshot = result.unwrap();
        assert_eq!(snapshot.service, Service::Ec2);
        assert!(!snapshot.panels[0].items.is_empty());
        drop(worker);
    }

    #[test]
    fn worker_survives_superseded_requests_and_returns_the_latest_service() {
        let mut worker = Worker::start(Config {
            demo: true,
            ..Config::default()
        })
        .unwrap();
        let mut app = App::new();
        let mut generation = 0;

        for index in 0..100 {
            app.service = Service::ALL[index % Service::ALL.len()];
            worker.request(&mut app, &mut generation).unwrap();
        }

        loop {
            match worker.responses.recv_timeout(POLL_INTERVAL).unwrap() {
                Response::Refresh {
                    generation: response_generation,
                    result,
                    ..
                } if response_generation == generation => {
                    assert_eq!(
                        result.unwrap().service,
                        Service::ALL[99 % Service::ALL.len()]
                    );
                    break;
                }
                Response::Execute { .. } => panic!("unexpected operation response"),
                Response::Refresh { .. } => {}
            }
        }
    }

    #[test]
    fn demo_operation_survives_a_queued_refresh() {
        let mut worker = Worker::start(Config {
            demo: true,
            ..Config::default()
        })
        .unwrap();
        let mut app = App::new();
        let mut generation = 0;
        worker.request(&mut app, &mut generation).unwrap();
        let operation = Operation {
            label: "Stop instance",
            service: "ec2",
            name: "stop-instances",
            input: json!({"InstanceIds":["i-example"]}),
            region: None,
            target: "i-example".into(),
            warning: "Stops the instance.",
            parameter: None,
            paginated: false,
        };
        worker
            .execute(operation.clone(), String::new(), &mut generation)
            .unwrap();
        let operation_cancel = Arc::clone(worker.operation_cancel.as_ref().unwrap());

        worker.request(&mut app, &mut generation).unwrap();
        assert!(!operation_cancel.load(Ordering::Relaxed));

        loop {
            match worker.responses.recv_timeout(POLL_INTERVAL).unwrap() {
                Response::Execute {
                    operation: completed,
                    result,
                    demo,
                    ..
                } => {
                    assert_eq!(completed, operation);
                    assert!(demo);
                    assert_eq!(result.unwrap()["simulated"], true);
                    break;
                }
                Response::Refresh { .. } => {}
            }
        }
    }

    #[test]
    fn cli_read_timeouts_warn_that_the_outcome_is_unknown() {
        let operation = Operation {
            label: "Stop instance",
            service: "ec2",
            name: "stop-instances",
            input: json!({"InstanceIds":["i-example"]}),
            region: None,
            target: "i-example".into(),
            warning: "Stops the instance.",
            parameter: None,
            paginated: false,
        };
        let read_timeout = Err(
            "AWS CLI exited with exit status: 255: Read timeout on endpoint URL: \"https://ec2\""
                .to_string(),
        );
        assert!(operation_log(&operation, &read_timeout, false).contains("outcome may be unknown"));
        assert!(operation_output(&operation, &read_timeout, false).contains("may be unknown"));

        let rejected = Err("AWS CLI exited with exit status: 254: AccessDenied".to_string());
        assert!(!operation_log(&operation, &rejected, false).contains("unknown"));
    }
}
