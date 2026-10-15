<div align="center">

<img width="1774" height="887" alt="lazyaws" src="https://github.com/user-attachments/assets/87cd581d-2078-4a0b-a615-58de25cf0ad9" />

# Lazy AWS

**Lazygit-style terminal UI for AWS.** Browse EC2, ECS, ECR, S3, and Lambda, filter lists, inspect JSON, and run confirmed actions — all from one keyboard-driven screen.

Lazy AWS shells out to your AWS CLI and shows the exact command before it runs, so there are no hidden API calls. Try it with local sample data, then point it at your account.

[Service guide](docs/services.md) · [AWS CLI action inventory](docs/aws-cli-actions.md) · [Captured demo](docs/demo.txt)

</div>

## Features

- **Browse anywhere**: five service tabs (EC2, ECS, ECR, S3, Lambda) with resource lists, related resources, filters, and JSON details.
- **Act with confirmation**: every action — including reads — shows account, target, warning, and the exact AWS CLI command before it runs.
- **Lazygit layout**: five left panes, a detail pane, a command log, and a key bar. The focused left pane grows; the log compacts on short terminals.
- **Keyboard and mouse**: tabs, panes, rows, filters, scrolling, action dialogs, and drag-to-copy text selection all work with keys or mouse.
- **Live but safe refresh**: only the active service is polled every five seconds, without overlapping requests. Errors keep the last good data and mark it stale.
- **Demo mode**: `cargo run -- --demo` runs on local sample data with no AWS CLI or credentials.

## Requirements

- Rust toolchain
- AWS CLI v2
- Configured AWS credentials with the permissions in [the service guide](docs/services.md)

## Getting started

1. Install Rust and AWS CLI v2, and configure credentials through the AWS CLI.
2. Preview with sample data (no AWS needed):
   ```sh
   cargo run -- --demo
   ```
3. Run against your account:
   ```sh
   cargo run -- --profile dev --region eu-west-1
   ```
4. Click into any list, press `a` on a row, pick an action, review the command, and press `y` to run it.
5. Build the release binary when you are ready:
   ```sh
   cargo build --release
   ./target/release/lazyaws --help
   ```

Sign in with the AWS CLI before you start if your profile uses SSO.

## Use your own AWS setup

Without flags, Lazy AWS uses the AWS CLI's normal configuration and environment variables.

| Option | Result |
| --- | --- |
| `--profile NAME` | Pass a profile to each AWS command |
| `--region REGION` | Pass a region to each AWS command |
| `--aws-bin PATH` / `LAZYAWS_AWS_BIN` | Select an AWS executable |
| `AWS_PROFILE` / `AWS_REGION` | Passed through when no flag overrides them |

File transfers ask for a path. A bucket upload uses the local file name as the object key; an object upload uses the selected key. A download fails if the local destination already exists. See the service guide for file limits and the full action list.

## Safety

- Lazy AWS only talks to AWS through the AWS CLI: credentials in, JSON out. It disables the pager and prompts.
- The header shows the account ID and short principal. Press `1` to see the full ARN, user ID, profile, and region. It never shows credentials or tokens.
- The action dialog asks for confirmation before it runs anything. An interrupted or timed-out action can still finish in AWS — refresh before you try it again.
- Timeouts and limits live in [src/config.rs](src/config.rs): 20 seconds per command, 30 seconds per snapshot, 50 seconds per refresh including identity lookup. Large lists can be incomplete; the command log shows limit notices.

## Troubleshooting

- **Identity error in the header**: your credentials or SSO session are invalid. Sign in with the AWS CLI and press `r` to refresh.
- **Empty or stale panes**: the last successful data is kept and marked stale. Check the command log for the failed command and limit notices.
- **Action did not seem to run**: it may have timed out after AWS accepted it. Refresh and check the resource before retrying.
- **Copy does not work**: copy uses OSC 52 clipboard writes. Enable clipboard writes in your terminal (and in tmux or your multiplexer, if you use one). See the [clipboard support notes](https://docs.rs/crossterm/0.29.0/crossterm/clipboard/struct.CopyToClipboard.html#terminal-support).

## Screen and keys

```text
+---------------- EC2 | ECS | ECR | S3 | Lambda ----------+
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
| `y` / `Y` or click Y | Run the confirmed action |
| `n` / `N` or click N at confirmation | Cancel the action |
| Esc or click Cancel in the menu or input dialog | Close the action dialog |
| Mouse click on a tab, pane, or row | Change service, focus, or selection |
| Mouse wheel over a list or details | Move the selection or scroll details |
| Left mouse drag, then release | Select a rectangle of text and copy it |
| `?` | Show keys |
| `q`, Ctrl+C | Exit |

Status shows `pf` (profile) and `rg` (region). `AWS chain` means the AWS CLI selects credentials from its configuration and environment. `AWS config` means the AWS CLI selects the region. `DEMO` marks local sample data.

```text
AWS credentials --> AWS CLI --> JSON --> snapshot --> resource panes
Resource row --> action catalog --> confirmed operation --> AWS CLI --> result --> details
```

See [the service guide](docs/services.md) for commands and permissions, and [the implementation guide](docs/development.md) for module boundaries and the full event flow.

## Checks

```sh
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release
```

Tests use local data and a fake AWS executable. They do not need an AWS account.

## Contributing

Module boundaries and workflows are in [the implementation guide](docs/development.md). Run `cargo fmt`, `cargo clippy`, and `cargo test` before you send a change.

## License

[MIT](LICENSE).
