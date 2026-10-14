# Implementation guide

The application has one CLI worker and one terminal loop. All AWS commands are
read commands. Rust code forbids `unsafe`.

| File | Purpose |
| --- | --- |
| `src/config.rs` | Defaults, limits, command-line options |
| `src/model.rs` | Service, resource, panel, and snapshot types |
| `src/app.rs` | Selection, focus, filters, and key input |
| `src/aws/` | CLI process control and service queries |
| `src/demo.rs` | Local sample data |
| `src/ui.rs`, `src/ui/` | Terminal layout, shared borders, and key displays |
| `src/runtime.rs` | Worker, polling, and terminal cleanup |
| `src/main.rs` | Command-line entry point |
| `tests/` | UI checks and test data |

```text
main -> runtime::run -> App::handle_key
                    -> Worker::request -> Sender::send
                    -> App::apply
                    -> ui::draw -> terminal buffer

worker closure -> AwsCli::fetch -> service fetch -> FetchContext::invoke
               -> FetchContext::invoke_region -> run_process -> Command::spawn
```

```text
Key input --> App --> request channel --> CLI worker
                          CLI worker --> response channel --> App --> UI
```

```text
UI loop               CLI worker
   |-- generation N ----->|
   |-- cancel N --------->|
   |-- generation N+1 --->|
   |<-- result N ---------|
   | discard N            |
   |<-- result N+1 -------|
   | apply N+1            |
```

Keep configurable values in `src/config.rs`. Service adapters use the shared
JSON runner. Pass command arguments directly to the process; do not use a shell.
Keep a selected resource by ID across refreshes. Clear related data when its
parent selection changes. Keep old data on a query error only when its parent
context is still valid.

No data migration or code generation is required.
