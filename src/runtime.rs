use std::{
    io::{self, IsTerminal},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, Sender},
    },
    thread::{self, JoinHandle},
    time::Instant,
};

use crossterm::event::{self, Event};

use crate::{
    app::{Action, App},
    aws::AwsCli,
    config::{Config, EVENT_WAIT, LOG_CAPACITY, POLL_INTERVAL},
    demo,
    model::{Service, Snapshot},
    ui,
};

struct Request {
    generation: u64,
    service: Service,
    selected: Option<String>,
    cancel: Arc<AtomicBool>,
}

struct Response {
    generation: u64,
    result: Result<Snapshot, String>,
}

struct Worker {
    requests: Option<Sender<Request>>,
    responses: Receiver<Response>,
    cancel: Arc<AtomicBool>,
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
                while let Ok(mut request) = incoming.recv() {
                    // Only the newest selection needs an AWS request.
                    for newer in incoming.try_iter() {
                        request = newer;
                    }

                    if request.cancel.load(Ordering::Relaxed) {
                        continue;
                    }

                    let result = if config.demo {
                        Ok(demo::snapshot(request.service, request.selected.as_deref()))
                    } else {
                        AwsCli::new(config.clone(), Arc::clone(&request.cancel))
                            .fetch(request.service, request.selected.as_deref())
                    };

                    if outgoing
                        .send(Response {
                            generation: request.generation,
                            result,
                        })
                        .is_err()
                    {
                        break;
                    }
                }
            })?;

        Ok(Self {
            requests: Some(requests),
            responses,
            cancel,
            thread: Some(thread),
        })
    }

    fn request(&mut self, app: &mut App, generation: &mut u64) -> io::Result<()> {
        self.cancel.store(true, Ordering::Relaxed);
        self.cancel = Arc::new(AtomicBool::new(false));
        *generation = generation.wrapping_add(1);

        let request = Request {
            generation: *generation,
            service: app.service,
            selected: app.selected_primary().map(String::from),
            cancel: Arc::clone(&self.cancel),
        };

        self.requests
            .as_ref()
            .ok_or_else(|| io::Error::other("CLI worker stopped"))?
            .send(request)
            .map_err(|_| io::Error::other("CLI worker stopped"))?;
        app.loading = true;
        Ok(())
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
        self.requests.take();

        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
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
    let _guard = TerminalGuard;
    let mut terminal = ratatui::try_init()?;
    let mut app = App::new();
    let mut generation = 0;
    worker.request(&mut app, &mut generation)?;
    let mut last_request = Instant::now();

    loop {
        while let Ok(response) = worker.responses.try_recv() {
            if response.generation != generation {
                continue;
            }

            match response.result {
                Ok(snapshot) => app.apply(snapshot),
                Err(error) => {
                    app.loading = false;
                    app.error = Some(error.clone());
                    app.logs.push_back(error);

                    while app.logs.len() > LOG_CAPACITY {
                        app.logs.pop_front();
                    }
                }
            }
        }

        if !app.loading && last_request.elapsed() >= POLL_INTERVAL {
            worker.request(&mut app, &mut generation)?;
            last_request = Instant::now();
        }

        terminal.draw(|frame| ui::draw(frame, &app, &config))?;

        if event::poll(EVENT_WAIT)?
            && let Event::Key(key) = event::read()?
        {
            match app.handle_key(key) {
                Action::Quit => break,
                Action::Refresh => {
                    worker.request(&mut app, &mut generation)?;
                    last_request = Instant::now();
                }
                Action::None => {}
            }
        }
    }

    Ok(())
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
        let response = worker.responses.recv_timeout(POLL_INTERVAL).unwrap();
        assert_eq!(response.generation, generation);
        let snapshot = response.result.unwrap();
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
            let response = worker.responses.recv_timeout(POLL_INTERVAL).unwrap();
            if response.generation == generation {
                assert_eq!(response.result.unwrap().service, Service::S3);
                break;
            }
        }
    }
}
