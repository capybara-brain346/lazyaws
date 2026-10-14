# Lazy AWS

A Rust terminal interface for EC2, ECS, ECR, and S3. The screen uses the
[Lazygit layout](https://github.com/jesseduffield/lazygit/blob/master/docs/Config.md):
five panes on the left, a detail pane on the right, a command log, and a key bar.
Use the service tabs to change the AWS service.
The bottom left pane expands when it has focus. The command log is compact
on terminals with fewer than 40 rows.
See [a captured demo screen](docs/demo.txt).

This first release supports resource lists, filters, and JSON detail views.
It uses AWS CLI read commands. It does not change AWS resources.

## Run

Install Rust and AWS CLI v2. Configure AWS credentials through the AWS CLI.
Use an IAM identity with the read permissions in [the service guide](docs/services.md).

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
| Status            |                                 |
|-------------------|                                 |
| Resources         | JSON details                    |
|-------------------|                                 |
| Related resources |                                 |
|-------------------|                                 |
| Related resources |---------------------------------|
|-------------------| Command log                     |
| Related resources |                                 |
+-------------------+---------------------------------+
| Keys                                                |
+-----------------------------------------------------+
```

| Key | Action |
| --- | --- |
| `[` / `]`, Shift+Left / Shift+Right | Change service |
| Tab / Shift+Tab, `h` / `l` | Change pane |
| `1` to `5` | Select a left pane |
| `0`, Enter | Select the detail pane |
| `6` | Select the command log |
| `j` / `k`, Up / Down | Select a row |
| `/` | Filter the current list |
| Enter / Esc in the filter | Accept / clear the filter |
| PageUp / PageDown | Scroll details |
| `r` | Refresh data |
| `?` | Show keys |
| `q`, Ctrl+C | Exit |

Select an instance, cluster, repository, or bucket in the first resource pane.
The other panes show its resources. Select a row to show its JSON details.
The application keeps each service's selection when you change tabs.

## Data and refresh

```text
AWS account --> AWS CLI --> JSON parser --> service snapshot --> panes
Key input ----------------> selection --> CLI request
```

```text
Terminal          Worker             AWS CLI
   |-- request ---->|                   |
   |                |-- read command -->|
   |                |<-- JSON / error --|
   |<-- snapshot ---|                   |
   |-- draw panes   |                   |
```

Only the active service is polled. A refresh starts every five seconds when
the worker is free. Requests do not overlap. The worker skips queued requests
for old selections and cancels a running command when selection changes.
The UI discards results for old selections.
Command timeouts, output limits, and list limits are defined in
[src/config.rs](src/config.rs). Large lists can be incomplete; the command log
shows limit notices. A command can run for up to 20 seconds; a full refresh has
a 30-second limit. An error keeps the last successful data and marks it stale.

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
