# Lazy AWS

Lazy AWS is a Rust terminal interface for EC2, ECS, ECR, and S3. It uses a
[Lazygit layout](https://github.com/jesseduffield/lazygit/blob/master/docs/Config.md)
with five panes, a detail pane, a command log, and a key bar. The bottom left
pane grows when it has focus. The command log is compact on short terminals.
See [a captured demo screen](docs/demo.txt).

Lazy AWS shows resource lists, filters, and JSON details. It also runs the
actions in [the service guide](docs/services.md). The action dialog shows the
account, target, warning, and command before it runs.

## Run

Install Rust and AWS CLI v2. Configure AWS credentials through the AWS CLI.
Use an IAM identity with the permissions in
[the service guide](docs/services.md).

```sh
cargo run -- --demo
cargo run -- --profile dev --region eu-west-1
cargo build --release
./target/release/lazyaws --help
```

Demo mode uses local sample data. It needs no AWS CLI or credentials.
Use `--aws-bin PATH` or `LAZYAWS_AWS_BIN` to select an AWS executable.
Without `--profile` or `--region`, the AWS CLI uses its normal configuration
and environment variables. Sign in with the AWS CLI before you start Lazy AWS
if your profile uses SSO.

## Screen and keys

```text
+---------------- EC2 | ECS | ECR | S3 ----------------+
| Account: 123456789012 · assumed-role/ops/alice       |
| Filter                                               |
|-------------------|---------------------------------|
| Status            |                                 |
|-------------------| JSON details                    |
| Resources         |                                 |
|-------------------|                                 |
| Related resources |---------------------------------|
| Related resources | Command log                     |
| Related resources |                                 |
+-------------------+---------------------------------+
| a actions  keys                                     |
+-----------------------------------------------------+
```

| Key or action | Result |
| --- | --- |
| `[` / `]`, Shift+Left / Shift+Right | Change service |
| Tab / Shift+Tab, `h` / `l` | Change pane |
| `1` to `5` | Focus a left pane |
| `0`, Enter | Focus the detail pane |
| `6` | Focus the command log |
| `j` / `k`, Up / Down | Select a row |
| `/` | Filter the current list |
| Enter / Esc in the filter | Accept / clear the filter |
| PageUp / PageDown | Scroll details |
| `r` | Refresh data |
| `a` or right-click a selected row | Open its actions |
| Up / Down in the action menu | Select an action |
| Enter or click an action | Choose it |
| Enter or click Accept while editing | Check and accept the value |
| `y` or click Run | Run the confirmed action |
| Esc or click Cancel | Close the action dialog |
| Mouse click on a tab, pane, or row | Change service, focus, or selection |
| Mouse wheel over a list or details | Move the selection or scroll details |
| `?` | Show keys |
| `q`, Ctrl+C | Exit |

The `a actions` footer opens actions for the focused row. The header shows the
account ID and short principal. Press `1` to focus Status. The detail pane then
shows the full account ARN, user ID, profile, and region. An identity error
replaces the account details. Lazy AWS does not show credentials or tokens.

The action list depends on the selected resource and its state. The dialog asks
for confirmation before it runs an action, including read actions. A file
transfer asks for a path. A bucket upload uses the local file name as the
object key. An object upload uses the selected key. A download fails if the
local destination already exists. See the service guide for file limits and
the full action list.

## Data and refresh

```text
AWS credentials --> AWS CLI --> JSON --> snapshot --> resource panes
Keyboard or mouse ---------------------> selection --> refresh request
Resource row --> action catalog --> confirmed operation --> AWS CLI --> result --> details
```

```text
User      Runtime/UI      App       Worker handle    Worker thread    AWS CLI
 |             |           |              |                |              |
 |-- click --->|           |              |                |              |
 |             |-- Mouse ->| select row   |                |              |
 |-- press a ->|-- Key --->| open dialog  |                |              |
 |-- choose -->|-- Key --->| select action|                |              |
 |-- confirm ->|-- Key --->| validate     |                |              |
 |             |<-- Action::Execute ------|                |              |
 |             |-- Worker::execute ------>|                |              |
 |             |           |              |-- request ---->|              |
 |             |           |              |                |-- execute --->|
 |             |           |              |                |<-- result ----|
 |             |           |              |<-- response ---|              |
 |             |<-- poll response --------|                |              |
 |             |-- update ->|              |                |              |
 |             |-- draw --->|              |                |              |
```

The runtime calls `App` for each event. `App` returns `Action::Execute` to the
runtime. The runtime calls `Worker::execute`, which sends a request to the
worker thread. The worker thread runs the AWS CLI and sends the result back.
For a file path or task count, press Enter to accept the value, then press `y`
to run the action.

Only the active service is polled. A refresh starts every five seconds when
the worker is free. Requests do not overlap. The worker skips queued requests
for old selections and cancels a running command when selection changes.
The UI discards results for old selections.
Command timeouts, output limits, and list limits are defined in
[src/config.rs](src/config.rs). Large lists can be incomplete; the command log
shows limit notices. Each AWS command can run for up to 20 seconds. A service
snapshot has a 30-second limit. Identity lookup runs first with its own
20-second command limit. One refresh can take up to 50 seconds. An error keeps
the last successful data and marks it stale.
An interrupted or timed-out action can still finish in AWS. Refresh before you
try it again.

## Checks

```sh
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release
```

Tests use local data and a fake AWS executable. They do not need an AWS account.
See [the service guide](docs/services.md) for commands and permissions.
See [the implementation guide](docs/development.md) for module boundaries.
