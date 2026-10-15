<div align="center">

<img width="1774" height="887" alt="lazyaws" src="https://github.com/user-attachments/assets/87cd581d-2078-4a0b-a615-58de25cf0ad9" />

# Lazy AWS

**Lazygit-style terminal UI for AWS.** Browse EC2, ECS, ECR, S3, and Lambda, filter lists, inspect JSON, and run confirmed actions — all from one keyboard-driven screen.

Lazy AWS shells out to your AWS CLI and shows the exact command before it runs, so there are no hidden API calls. Try it with local sample data, then point it at your account.

[Website](https://lazyaws.xyz) · [Service guide](docs/services.md) · [AWS CLI action inventory](docs/aws-cli-actions.md) · [Captured demo](docs/demo.txt)

</div>

## Features

- **Browse anywhere**: five service tabs (EC2, ECS, ECR, S3, Lambda) with resource lists, related resources, filters, and JSON details.
- **Act with confirmation**: every action — including reads — shows account, target, warning, and the exact AWS CLI command before it runs.
- **Lazygit layout**: five left panes, a detail pane, a command log, and a key bar. The focused left pane grows; the log compacts on short terminals.
- **Keyboard and mouse**: tabs, panes, rows, filters, scrolling, action dialogs, and drag-to-copy text selection all work with keys or mouse.
- **Live but safe refresh**: only the active service is polled, five seconds after the previous refresh finishes, without overlapping requests. Errors keep the last good data and mark it stale.
- **Themes**: press `t` to preview and apply one of 20 bundled color themes, or keep the default terminal colors.
- **Demo mode**: `cargo run -- --demo` runs on local sample data with no AWS CLI or credentials.

## Install

We support macOS, Ubuntu, Arch Linux, and Omarchy. Windows is not supported.
The first binary targets are macOS 15 and Ubuntu 24.04 on ARM64 and x86_64,
plus Arch Linux and Omarchy on x86_64. Debian users can use curl or Homebrew,
but Debian is not supported.

Use curl on macOS or Linux:

```sh
curl -fsSL https://lazyaws.xyz/install.sh | sh
```

The script checks the archive's SHA-256 and installs to `~/.local/bin` without
sudo. If needed, add that directory to your PATH. To inspect the script first:

```sh
curl -fsSLo install.sh https://lazyaws.xyz/install.sh
less install.sh
sh install.sh
```

Or use Homebrew on macOS or Linux:

```sh
brew install capybara-brain346/tap/lazyaws
```

Run the installed demo without AWS credentials:

```sh
lazyaws --demo
```

AWS mode needs AWS CLI v2 and credentials with the permissions in
[the service guide](docs/services.md). Rust is needed only for source builds.
See [release operations](docs/releases.md) for channel status, AUR publication,
upgrades, and rollback.

## Getting started

These steps build from source. For a binary install, use the commands above.

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
- Timeouts and limits live in [src/config.rs](src/config.rs): 20 seconds per command, 30 seconds per snapshot, and 60 minutes per S3 file upload or download. The identity lookup runs on the first refresh and again only after it fails. Large lists can be incomplete; the command log shows limit notices.

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
| `[` or `]`, Shift+Left or Shift+Right | Change service |
| `0` to `6` | Focus a pane |
| Tab / Shift+Tab, `h` / `l` | Cycle through panes |
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
| `t` | Open the theme picker |
| `j` / `k`, Up / Down in the theme picker | Preview a theme |
| Enter or click a theme | Apply it |
| Esc or `t` in the theme picker | Close it and keep the current theme |
| `?` | Show keys |
| `q`, Ctrl+C | Exit |

Status shows `pf` (profile) and `rg` (region). `AWS chain` means the AWS CLI selects credentials from its configuration and environment. `AWS config` means the AWS CLI selects the region. `DEMO` marks local sample data.

```text
AWS credentials --> AWS CLI --> JSON --> snapshot --> resource panes
Resource row --> action catalog --> confirmed operation --> AWS CLI --> result --> details
```

See [the service guide](docs/services.md) for commands and permissions, and [the implementation guide](docs/development.md) for module boundaries and the full event flow.

## Themes

`assets/themes.json` holds the theme presets. The build embeds the file, so the
binary needs no extra files. The picker shows `Terminal` first. `Terminal` uses
the colors of your terminal. The bundled presets are:

Catppuccin Mocha, Catppuccin Latte, Dracula, Nord, Gruvbox Dark, Gruvbox Light,
Tokyo Night, One Dark, Solarized Dark, Solarized Light, GitHub Dark,
GitHub Light, Ayu Dark, Everforest Dark, Rosé Pine, Kanagawa, Monokai,
Night Owl, Material Palenight, and Cobalt2.

Each preset sets all 18 color roles: `background`, `text`, `border`, `focus`,
`title`, `muted`, `key`, `warning`, `error`, `selection_text`,
`selection_background`, `inactive_selection_background`, `text_selection_text`,
`text_selection_background`, `json_key`, `json_string`, `json_literal`, and
`json_number`. A color is `#RRGGBB`, an ANSI color name, an ANSI index, or
`reset`. To add a preset, add an object to the `themes` list and rebuild. A
missing or bad color stops the load. Then only `Terminal` is available, and the
command log shows the error.

The selected theme applies to the current session only.

## Checks

```sh
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build --release
```

Tests use local data and a fake AWS executable. They do not need an AWS account.

## Website

The Next.js landing page is in [apps/landing](apps/landing/README.md). Run `npm ci` and `npm run dev` from that directory.

## Contributing

Module boundaries and workflows are in [the implementation guide](docs/development.md). Run `cargo fmt`, `cargo clippy`, and `cargo test` before you send a change.

## License

[MIT](LICENSE).
