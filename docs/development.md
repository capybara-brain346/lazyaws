# Implementation guide

The application has one terminal loop and one AWS CLI worker. The worker reads
resource data and runs confirmed actions. Rust code forbids `unsafe`.

| File | Purpose |
| --- | --- |
| `src/config.rs` | Defaults, the default theme, limits, and command-line options |
| `src/theme.rs` | Theme color roles and the presets file parser |
| `assets/themes.json` | Bundled theme presets, embedded at build time |
| `src/model.rs` | Service, resource, panel, and snapshot types |
| `src/app.rs`, `src/app/` | Selection, focus, filters, action state, theme state, and input |
| `src/operations.rs`, `src/operations/` | Account identity and the allowed action catalog |
| `src/aws/` | AWS CLI process control, queries, actions, and file transfer |
| `src/demo.rs` | Local sample data |
| `src/ui.rs` | Layout and the `draw` wrapper |
| `src/ui/mouse.rs` | Mouse targets and reverse-order hit testing |
| `src/ui/actions.rs` | Action dialog, warnings, and command preview |
| `src/ui/details.rs` | JSON details, account details, and command output |
| `src/ui/chrome.rs` | Borders, help, and key bar |
| `src/ui/themes.rs` | Theme picker popup and its mouse targets |
| `src/runtime.rs` | Worker, polling, input events, and terminal cleanup |
| `src/runtime/selection.rs` | Mouse text selection, fixed screen, and plain text copy |
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

`App::open_actions_for` permits actions for loaded rows while a refresh runs.
It still checks the account, selected row, filter, and panel errors. A new
primary selection clears related rows before it requests a refresh. The dialog
keeps the target and arguments captured when it opens.

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

## Mouse text selection

The runtime holds a screen buffer while the left mouse button is down. It
defers a left click until release. A drag selects a rectangle and returns
`Action::Copy` instead of passing the click to `App`. This prevents a text
selection from changing a row, opening a dialog, or running an action. Selection
also works while an action runs and while a dialog or help screen is open.
Keys, resize, focus loss, and other mouse actions clear the selection.

```text
Before
runtime::run -> App::handle_mouse -> HitMap::target_at

After
+ runtime::run -> TextSelection::handle_mouse
+   -> App::handle_mouse -> HitMap::target_at (click on release)
+   -> Drag::update -> Drag::text -> Action::Copy (drag on release)
+ runtime::run -> TextSelection::draw -> Frame::buffer_mut
+ runtime::run -> CopyToClipboard::to_clipboard_from -> execute!
```

```text
Displayed Buffer -> fixed Buffer -> rectangle -> plain text -> OSC 52 -> clipboard
```

```text
User             Runtime             TextSelection          Terminal
 |-- press -------->|-- screen ---------->|
 |-- drag --------->|-- coordinates ----->|
 |                  |<-- highlight ------|
 |                  |-- frame -------------------------------->|
 |-- release ------>|-- coordinates ----->|
 |                  |<-- Action::Copy ---|
 |                  |-- OSC 52 text --------------------------->|
```

The copy path uses Crossterm's `osc52` feature. It does not read the clipboard.
The terminal and any multiplexer must allow OSC 52 writes. There is no reply
that confirms a clipboard write. Output errors enter the command log without
the selected text. Tests check click/drag separation, fixed text, Unicode,
blank selection, cancellation, resize, and rows in each service.

## Expanded action catalog

The action catalog now supplies read and change commands for all resource tabs.
The existing worker and confirmation path run them. No worker or query API changed. Demo data includes service ARNs, task definition
ARNs, and repository ARNs for the added actions.

```text
Before
App::open_actions_for -> operations::available -> service::available -> operation
App::confirm_action -> Operation::args -> existing count/file checks
AwsCli::execute -> FetchContext::invoke_region -> validate_response (ECR deletion)
ui::draw_interactive -> draw_action_dialog -> draw_action_menu (first rows)

After
= App::open_actions_for -> operations::available -> service::available -> operation
+ service::available supplies added action descriptors
+ App::confirm_action -> Operation::args -> JSON/tag/revision/deployment checks
+ AwsCli::execute -> Operation::args -> captured GetObject bucket/key flags
= AwsCli::execute -> FetchContext::invoke_region
+                                      -> validate_response (ECR read/delete, ECS run-task)
~ FetchContext::invoke_region -> display_command (JSON input hidden in logs)
~ ui::draw_interactive -> draw_action_dialog -> draw_action_menu (selected row window)
```

`=` marks an existing call, `+` marks added behavior, and `~` marks changed behavior.
`service::available` refers to the EC2, ECS, ECR, S3, or Lambda catalog function chosen
by `operations::available`. JSON fields add to a captured input object; they
cannot replace fixed top-level fields or set `registryId`. Text tag input keeps
the key. ECS revision and deployment ARN input keeps the account and Region.
The allowed CLI paginators use `MAX_ITEMS` from config. ECS deployment lists
use API pagination; the command log reports a returned token.

```text
Selected resource --> catalog --> Operation + input --> Operation::args
Operation::args --> checked CLI arguments --> worker --> AWS CLI
AWS CLI --> JSON --> validate_response --> result or error --> App Details
```

S3 GetObject uses captured `--bucket=...` and `--key=...` arguments.
The AWS CLI streaming command does not support `--cli-input-json`. The equals
form keeps a key that starts with `--` inside the value. The destination remains
the final argument, so the existing download stage can replace it.

The action menu computes its first row from the selected action and available
height. Mouse targets keep the original action index. The fake CLI tests check
API failures returned with a successful process exit and hidden JSON request
values in command logs. UI checks cover a long
menu at the minimum terminal size. The [command inventory](aws-cli-actions.md)
records supported actions and CLI-only commands.

## Themes

`App::new` loads the presets once. `App::theme` gives the theme to draw. While
the picker is open, it gives the previewed theme. Each draw function gets the
theme from the `App` and does not use fixed color constants. `pane_block` and
the first full-screen fill paint `background` and `text`. The default
`Terminal` theme uses `reset` for both, so the screen keeps the terminal colors.

```text
Before
ui::draw_interactive -> draw_services / draw_side_panels / draw_details_and_log
                        -> pane_block(title, focused) -> config::*_COLOR
runtime::run -> TextSelection::draw(frame) -> config::TEXT_SELECTION_*_COLOR

After
+App::new -> theme::load(THEMES_JSON) -> parse_theme -> Color::from_str
+ui::draw_interactive -> App::theme -> Block(theme.base())
~                     -> draw_services / draw_side_panels / draw_details_and_log
~                        -> pane_block(title, focused, theme) -> theme fields
+                     -> themes::draw_theme_picker, when open -> MouseTarget::ChooseTheme
~runtime::run -> TextSelection::draw(frame, App::theme)
+App::handle_key('t') -> open_themes
+App::handle_key, picker open -> handle_theme_key -> apply_theme
+App::handle_mouse, picker open -> ChooseTheme(index) -> apply_theme
```

```text
assets/themes.json --include_str!--> THEMES_JSON --theme::load--> Vec<Theme>
DEFAULT_THEME + Vec<Theme> --> App.themes
Key t --> App.theme_picker = Some(App.theme)
j/k --> theme_picker index --> App::theme --> draw functions --> screen colors
Enter / click --> App.theme = index      Esc / t --> picker closes, theme unchanged
```

A focused selected row uses `selection_text` on `selection_background`. An
unfocused selected row uses `text` on `inactive_selection_background`. Thus a
preset can use a bright accent for the focused row and keep the unfocused row
readable.
