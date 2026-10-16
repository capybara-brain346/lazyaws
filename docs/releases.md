# Release operations

## Publication status

| Channel | Status |
| --- | --- |
| Source repository | Public: https://github.com/capybara-brain346/lazyaws |
| Website | Existing Worker at https://lazyaws.xyz; `www.lazyaws.xyz` is retained |
| GitHub Releases | [Latest release](https://github.com/capybara-brain346/lazyaws/releases/latest), immutable assets, published by `release.yml` |
| curl | Live at https://lazyaws.xyz/install.sh; installs the latest GitHub release automatically |
| Homebrew | Published [tap](https://github.com/capybara-brain346/homebrew-tap); its `update-lazyaws` workflow follows the latest release |
| crates.io | Published [0.1.0](https://crates.io/crates/lazyaws/0.1.0); locked registry install and demo checks pass |
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

## Install channels

curl and Homebrew follow the latest GitHub release without a manual step or a
secret. GitHub marks a release as latest only when `release.yml` publishes it,
after all archives and `SHA256SUMS` pass verification.

```text
push tag -> release.yml -> verified GitHub Release (latest)
                               |
        +----------------------+------------------------------+
        v                                                     v
install.sh (no version)                       homebrew-tap update-lazyaws.yml
 HEAD github.com/.../releases/latest           every 30 min or a manual run
 -> 302 /releases/tag/vX.Y.Z                    -> newer release? check SHA256SUMS
 -> download + SHA-256 check                    -> generate formula from that tag
                                                -> commit with the tap's own token
                                                -> brew test + audit on 4 targets
```

- curl: `install.sh` follows the redirect from `latestUrl` in
  `distribution/config.json`. It accepts only a redirect to this repository's
  `/releases/tag/vX.Y.Z` page. The script holds no version, so the site needs a
  deploy only when the installer logic changes. Deploy with `npm run deploy`
  from `apps/landing`, then check that `https://lazyaws.xyz/install.sh` matches
  `apps/landing/public/install.sh`.
- Homebrew: the tap workflow runs every 30 minutes. It never moves the formula
  to an older version. To update at once, run
  `gh workflow run update-lazyaws.yml --repo capybara-brain346/homebrew-tap`.
  GitHub disables scheduled workflows after 60 days with no activity in the
  repository. If the tap does not follow a release, enable the workflow in the
  tap's Actions tab, then run it manually.
- After a release, check both channels: run `install.yml` with the tag, or run
  the installer with no version and `brew upgrade lazyaws`.

The generated recipes in `packaging/` are reference copies. The tap workflow
generates its own formula from the release tag, so they do not need a commit
for each release.

The AUR recipe stays in this repository. It can be built locally with
`makepkg -si` from `packaging/aur/lazyaws-bin` after the release files exist.
It is not in the AUR or a configured pacman repository. Do not advertise
`yay -S lazyaws-bin` or `pacman -S lazyaws` as available install commands.

For the first crates.io release, use the local Cargo credential and run
`cargo publish --locked` from the clean release commit. The publish command
sends only the source package selected by `Cargo.toml`; it does not upload
the website or credentials. `.github/workflows/crates.yml` can run a dry run
or publish from a matching stable tag. Its publish job reads
`CARGO_REGISTRY_TOKEN` from the `crates.io` GitHub environment. The supplied
credential is configured there; it is not available to pull-request jobs.
Replace it through GitHub environment secrets when it expires or is revoked.

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

If a release is bad, publish a corrected version; curl and Homebrew then follow
it. To stop new installs before that, mark the previous release as latest with
`gh release edit vX.Y.Z --latest`, and revert the formula commit in the tap. The
tap workflow does not downgrade the formula by itself. Do not overwrite
immutable assets or move a published tag. Changing the latest release does not
downgrade users who already installed a version.

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
The same jobs run `brew audit --strict` against the installed formula.

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

## First release verification

- [Native release checks](https://github.com/capybara-brain346/lazyaws/actions/runs/38738345755): all four targets passed formatting, Clippy, 99 Rust tests, release builds, extracted binary checks, and demo exit. The Linux x86_64 binary also passed in a clean Arch container.
- [Public install checks](https://github.com/capybara-brain346/lazyaws/actions/runs/38739508477): the deployed curl script and Homebrew formula installed and passed version, help, and demo checks on all four targets.
- [Strict Homebrew checks](https://github.com/capybara-brain346/lazyaws/actions/runs/38740094248): curl install, Homebrew install, formula tests, and strict audit pass on all four targets. The ARM64 job needed retries after two Homebrew broken-pipe failures. A diagnostic run and the final plain-command retry passed; the earlier pipe failures have no confirmed cause.
- All four public archive digests match `SHA256SUMS` and GitHub's asset digests.
- All six browser tests pass against `https://lazyaws.xyz`. The deployed Worker version is `89f29d77-721e-432f-aef2-1b15eb0fd57b`. Both custom domains remain attached.
- A registry install of `lazyaws 0.1.0` passed help, version, and terminal demo checks on Omarchy.
- [Cargo workflow dry run](https://github.com/capybara-brain346/lazyaws/actions/runs/38740098209) passed. The manual publish job uses the configured `crates.io` environment secret.
- Omarchy checks used version `4.0.4-1`, `glibc 2.44+r24+g16be1518495f-1`, and `gcc-libs 16.2.1+r23+gd564253eb6c8-1`.
- The local Arch recipe builds one package from the public archive. `!strip` and `!debug` keep the already stripped binary unchanged and prevent an empty debug package. No package was installed into the user's system for this check.

Homebrew core is deferred until the project meets its current
[acceptance policy](https://docs.brew.sh/Package-Acceptance-Policy). The tap is
independent of core acceptance. AUR publication and R2 remain deferred by the
owner. Other reviewed listings remain later phases.
