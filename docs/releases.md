# Release operations

## Publication status

| Channel | Status |
| --- | --- |
| Source repository | Public: https://github.com/capybara-brain346/lazyaws |
| Website | Existing Worker at https://lazyaws.xyz; `www.lazyaws.xyz` is retained |
| GitHub Releases | First release in preparation |
| curl | Installer prepared; publish the latest pointer only after download checks |
| Homebrew | macOS/Linux formula generator prepared; tap publication follows the release |
| crates.io | Local credentials supplied; package checks precede first publication |
| AUR | Recipe prepared; owner declined AUR publication |
| R2 | Account not enabled; owner chose GitHub downloads now and R2 later |
| Homebrew core / MacPorts / Ubuntu / Snap | Later review; no listing claimed |

## Configuration and file flow

`Cargo.toml` is the version source. `distribution/config.json` defines hosts,
targets, install defaults, and OS limits. Do not change a published archive.
The installer is generated from `distribution/install.sh.in`; do not edit the
generated public script directly.

```text
Cargo.toml + distribution/config.json
         |
         +--> generate-installer.mjs --> public/install.sh --> Cloudflare
         |
         +--> package-release.mjs --> four archives --> SHA256SUMS
                                                        |
                                                        v
                                                 GitHub Release
                                                        |
                   +------------------------------------+
                   v                                    v
            generate-packages.mjs                curl download
                   |
                   +--> Homebrew formula --> public tap
                   +--> PKGBUILD + .SRCINFO --> local Arch/Omarchy package
```

The website call path is unchanged. Its command data changes:

```text
Before: Home -> InstallGuide -> CopyButton(git clone / cargo run)
After:  Home -> InstallGuide -> CopyButton(curl / brew / lazyaws --demo)
Added:  release workflow -> package-release.mjs -> tar -> release assets
Added:  generate-packages.mjs -> formula / PKGBUILD / .SRCINFO
```

## Prepare a version

1. Change the package version in `Cargo.toml`; update `Cargo.lock`.
2. Generate the installer and run checks from the repository root:

   ```sh
   node scripts/generate-installer.mjs
   node --test scripts/*.test.mjs tests/install.test.mjs
   cargo fmt --all -- --check
   cargo clippy --all-targets --locked -- -D warnings
   cargo test --locked
   cargo build --release --locked
   cargo package --list
   cargo publish --dry-run --locked
   ```

3. Run the landing app's lint, build, type check, and browser tests. Run
   `npx wrangler deploy --dry-run` from `apps/landing`.
4. Commit and push the checked source. The release must use that exact commit.
   To test the release workflow without publication, dispatch `release.yml`
   on the branch with input `mode=dry-run`.
5. Create and push the matching `vX.Y.Z` tag. `release.yml` runs native tests
   and builds on all four configured targets. It tests the extracted binary,
   collects all four archives, and verifies their digests before it publishes
   a draft release. Immutable releases are enabled in the repository.

```text
Maintainer          GitHub Actions             GitHub Releases
    |--push tag---------->|                          |
    |                     |--native build/tests      |
    |                     |--create draft----------->|
    |                     |--upload + verify-------->|
    |                     |--publish---------------->|
    |<--release URL-------|                          |
```

## Publish the install channels

Download `SHA256SUMS` and all four archives from the published release into a
temporary directory. Verify all digests before generating packages:

```sh
node scripts/generate-packages.mjs /path/to/SHA256SUMS
```

Copy `packaging/homebrew/Formula/lazyaws.rb` to `Formula/lazyaws.rb` in
`capybara-brain346/homebrew-tap`. Check the formula, install it, run `brew test`,
and publish the tap update. Do not publish a formula with a temporary checksum.

The AUR recipe stays in this repository. It can be built locally with
`makepkg -si` from `packaging/aur/lazyaws-bin` after the release files exist.
It is not in the AUR or a configured pacman repository. Do not advertise
`yay -S lazyaws-bin` or `pacman -S lazyaws` as available install commands.

For the first crates.io release, use the local Cargo credential and run
`cargo publish --locked` from the clean release commit. The publish command
sends only the source package selected by `Cargo.toml`; it does not upload
the website or credentials. Later, configure Trusted Publishing for
`.github/workflows/crates.yml` and its `crates.io` environment. That workflow
can run a dry run or publish from a matching stable tag.

After the public downloads and tap pass, write the exact tag plus a newline
to `apps/landing/public/latest.txt`. Build and deploy the landing app with
`npm run deploy`. Verify `/install.sh`, `/latest.txt`, and a fresh install from
`https://lazyaws.xyz`. Commit the updated pointer and generated recipes.

Cloudflare deploys use the existing `lazyaws` Worker in account
`32f2a5e2a676ee905a3828b2b38b0150`. Custom domains are declared in
`apps/landing/wrangler.jsonc` so a deploy retains dashboard changes.

## Upgrade, rollback, and removal

- curl: rerun the installer to upgrade. To select a prior version, download
  the script and run `sh install.sh vX.Y.Z`. Use `LAZYAWS_INSTALL_DIR` to select
  an explicit destination. A failed install keeps the previous executable.
- Homebrew: use `brew update` and `brew upgrade lazyaws`. Use Homebrew to
  remove or replace Homebrew-managed files.
- Local Arch package: rebuild the recipe for the chosen version and install
  it with pacman. Use pacman to remove package-managed files.
- Source registry: use `cargo install lazyaws --locked` to install the current
  registry release, or select a version with Cargo's `--version` option.
- Remove a curl install by deleting only the installed `lazyaws` executable.
  No AWS files or shell configuration are changed by the installer.

If a release is bad, restore the previous `latest.txt` value and deploy the
site, then publish a corrected package version. Do not overwrite immutable
assets or move a published tag. Restore the tap separately. A latest pointer
change does not downgrade users who already installed a version.

## Deferred checks

R2 needs account activation and a download domain before it can be used.
External package listings require their maintainers' review. Test the current
Arch and Omarchy environment for each release, including terminal exit.
Native macOS builds do not prove that Gatekeeper will accept a download on a
clean Mac. Resolve signing or notarization if that check blocks execution.

## Native install checks

After the site and tap are live, run `.github/workflows/install.yml` with its
required `tag` input, for example `v0.1.0`. The workflow checks the deployed
installer against its generated source, installs the exact release, checks
help/version and demo exit, then installs and tests the Homebrew formula on
all four native targets. Homebrew setup runs only on temporary GitHub runners.

The initial macOS run found a test path assumption. macOS resolves its
temporary directory from `/var` to `/private/var`. The test now keeps the raw
input path but compares the result with the canonical parent. The application
path is unchanged:

```text
Before:
download test -> Operation::args -> args::download_file -> download_path
download test -> assert_eq(raw destination)

After:
download test -> Operation::args -> args::download_file -> download_path [same]
+ download test -> Path::parent -> Path::canonicalize -> PathBuf::join
- assert_eq(raw destination)
+ assert_eq(canonical destination)
```
