# Implementation guide

The application has one terminal loop and one AWS CLI worker. The worker reads
resource data and runs confirmed actions. Rust code forbids `unsafe`.

| File | Purpose |
| --- | --- |
| `src/config.rs` | Defaults, limits, and command-line options |
| `src/model.rs` | Service, resource, panel, and snapshot types |
| `src/app.rs`, `src/app/` | Selection, focus, filters, action state, and input |
| `src/operations.rs`, `src/operations/` | Account identity and the allowed action catalog |
| `src/aws/` | AWS CLI process control, queries, actions, and file transfer |
| `src/demo.rs` | Local sample data |
| `src/ui.rs` | Layout and the `draw` wrapper |
| `src/ui/mouse.rs` | Mouse targets and reverse-order hit testing |
| `src/ui/actions.rs` | Action dialog, warnings, and command preview |
| `src/ui/details.rs` | JSON details, account details, and command output |
| `src/ui/chrome.rs` | Borders, help, and key bar |
| `src/runtime.rs` | Worker, polling, input events, and terminal cleanup |
| `tests/` | UI checks and test data |

`ui::draw(frame, app, config)` remains a wrapper for callers that need no mouse
targets. Runtime calls `ui::draw_interactive` to draw the same screen and get a
`HitMap`. The map uses the rectangles that the UI draws. It lists panes before
rows. Reverse search makes a row win over its pane.

## Call paths

The draw wrapper stays for tests and non-interactive callers. Runtime uses the
new draw path to receive the hit map. Mouse events use that map. A confirmed
action goes through a channel to the worker thread.

```text
Before
runtime::run -> terminal.draw
-  -> ui::draw -> draw_services / draw_side_panels / draw_details_and_log
runtime::run -> Event::Key -> App::handle_key

After
+runtime::run -> terminal.draw
+  -> ui::draw_interactive -> draw_services / draw_side_panels / draw_details_and_log
+     -> draw_action_dialog, when open
+     -> HitMap
+runtime::run -> Event::Mouse -> App::handle_mouse -> HitMap::target_at
+                                                    -> select_row / open_actions_for
+runtime::run -> Action::Execute -> Worker::execute -> requests.send
+worker closure -> AwsCli::execute -> FetchContext::invoke_region
+                  -> run_process -> Command::spawn
```

The `-` marks the old draw call. The `+` marks the new draw, mouse, and action
paths. The old `ui::draw` path remains as a wrapper:

```text
ui::draw -> ui::draw_interactive -> pane drawing + HitMap (discarded)
```

## Data flow

```text
Resource query:
Config + selection --> Worker --> AWS CLI --> JSON --> Snapshot --> App --> panes

Action:
Selected row --> operations catalog --> dialog input --> validated args
Validated args --> Worker --> AWS CLI --> output + log --> App --> details
Local file --> S3 PutObject -------------------------------> S3 object
S3 object --> GetObject --> private temp file --> new local destination
```

The mouse uses the last hit region that contains the pointer. A modal control
is added after pane and row regions. The App ignores other clicks while an
action dialog or help screen is open. The dialog shows the selected account,
target, warning, and computed command. Input and CLI output pass through the UI
text sanitizer before display.

## Event sequence

```text
User     Runtime        UI         App        Worker handle    Worker thread   AWS CLI
 |          |           |           |               |                |           |
 |          |-- draw -->|           |               |                |           |
 |          |<- HitMap -|           |               |                |           |
 |-- click->|           |           |               |                |           |
 |          |-- Mouse ---------------->|             |                |           |
 |          |           |           | select row    |                |           |
 |-- a ---->|-- Key ------------------>|             |                |           |
 |          |           |           | open dialog   |                |           |
 |-- choose>|-- Key ------------------>| choose action|               |           |
 |-- y ---->|-- Key ------------------>| validate args|               |           |
 |          |<-- Action::Execute -------|             |                |           |
 |          |-- Worker::execute -------------------->|                |           |
 |          |           |           |               |-- Request ----->|           |
 |          |           |           |               |                |-- execute->|
 |          |           |           |               |                |<- result --|
 |          |           |           |               |<- Response -----|           |
 |          |<-- poll response ----------------------|                |           |
 |          |-- update App -------------------------->|                |           |
 |          |-- draw -->|           |               |                |           |
```

The runtime calls `App` for each event. `App` returns `Action::Execute` to the
runtime. The runtime then calls `Worker::execute`. The worker sends a request
to its thread. The thread calls `AwsCli::execute`. The runtime reads the result
and updates `App` before it draws the next frame.
For an action with an input field, press Enter to accept the value, then press
`y` to run the action.

## Worker refresh sequence

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
JSON runner. Pass command arguments directly to the process. Do not use a shell.
Keep a selected resource by ID across refreshes. Clear related data when its
parent selection changes. Keep old data on a query error only when its parent
context is still valid.

AWS actions have a 20-second command time limit. A timed-out action can still
finish in AWS. Refresh the resource before you try it again.

No data migration or code generation is required.
