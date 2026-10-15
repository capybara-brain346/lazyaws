# Package publication plan

Plan date: 2026-10-15. The first release is published on GitHub, Homebrew,
and crates.io. The installer is live at `https://lazyaws.xyz/install.sh`.
Current checks and deferred channels are recorded in `docs/releases.md`.

## Parallel phases and agent ownership

Use GPT-6 Luna with extra-high reasoning for sub-agents. Use up to three
sub-agents at a time. Each agent owns separate files. The main agent owns
shared configuration, integration, website changes, and publication order.
All agents may use the available GitHub and Cloudflare access. Report a missing
permission or account without printing credentials. Continue independent work
while the owner supplies access.

The owner approved making `capybara-brain346/lazyaws` public, including its
Git history. Preserve any custom domain already attached to the landing Worker.
Discover that domain before deployment and record it in release configuration.

| Phase | Owner | Work | Completion check |
| --- | --- | --- | --- |
| 0: access and contract | Main agent | Check GitHub, Worker domains, R2, package names, and publication access. Define `distribution/config.json` and the versioned archive contract. | Other agents have the same targets, URLs, version source, and archive names. |
| 1A: installer | Installer agent | Add the POSIX installer, generator, and focused failure/upgrade tests. | Installer tests pass; generated script agrees with config. |
| 1B: build and release | Release agent | Add native target CI, archive packaging, metadata, and a draft-first release workflow. | Local checks pass; workflow can build all four archives from the release commit. |
| 1C: package recipes | Package agent | Add Homebrew and AUR recipe generation from archive checksums. Check remote ownership and access. | Recipe syntax and metadata checks pass; no fake checksum or pending URL is advertised. |
| 1D: hosting and guides | Main agent, with phases 1A-1C | Reconcile the Worker domain, prepare download hosting, update user guides and website, and track blockers. | Website tests pass; links match the selected release host. |
| 2: integration | Main agent | Review all changes, test the generated installer and packages against the same archives, and push the release commit. | One commit and version pass all required checks. |
| 3: binary publication | Main agent | Create tag and draft release, collect all four tested archives, publish GitHub files, then update latest. Add R2 later. | Public downloads have matching versions and SHA-256 values. |
| 4A: Homebrew | Package agent after phase 3 | Publish the tap formula using final URLs and checksums. | Public formula installs the released binary on tested macOS/Linux targets. |
| 4B: Arch recipe | Package agent after phase 3 | Generate `PKGBUILD` and `.SRCINFO` with final checksums. The owner declined AUR publication. | Local package metadata matches the tested recipe; no AUR listing is claimed. |
| 4C: crates.io | Release agent after phase 2 | Package, dry run, then publish using the owner's registry access. | Registry version is available and a clean install succeeds. |
| 5: reviewed listings | Available agent after phase 4 | Prepare Homebrew core, MacPorts, and any justified Ubuntu/Snap submissions. | Track submitted, accepted, and blocked states separately. |

```text
Main: config + access
          |
          +--> Agent A: installer ----------+
          +--> Agent B: release builds -----+--> Main: combined checks
          +--> Agent C: package recipes ----+             |
          +--> Main: hosting + website -----+             v
                                                tested release files
                                                         |
                         +-------------------------------+-----------+
                         v                               v           v
                    Homebrew tap                        AUR      crates.io
```

```text
Main agent        Sub-agents           GitHub/Cloudflare        Owner
    |--contract------>|                       |                  |
    |                 |--access/checks------->|                  |
    |<--files/results-|                       |                  |
    |--report missing access----------------------------------->|
    |<--access supplied-----------------------------------------|
    |--combined checks and publication------->|                  |
    |<--public URLs and status----------------|                  |
```

Do not let agents change repository visibility, release tags, latest pointers,
or shared files at the same time. Publication uses one coordinator. Agents can
publish their assigned package channels after the coordinator supplies final
URLs and checksums. A failed channel must not cause a rebuild of a released
version. No Windows work is included.

Access check: GitHub authentication works. The repository is now public and
immutable releases are enabled. The Worker has `lazyaws.xyz` and
`www.lazyaws.xyz`; both domains are retained in Wrangler config. R2 is not
enabled. The owner chose GitHub Releases now and R2 later. crates.io publication
and a locked registry install passed. A publish credential is
configured in the `crates.io` GitHub environment. The owner declined AUR
publication, so its recipe stays in this repository.

## Support statement

Use this text in the README, website, install script, and release notes:

> We support macOS, Ubuntu, Arch Linux, and Omarchy. macOS and supported Linux
> users can use curl or Homebrew. Arch Linux and Omarchy users can also build
> the supplied package recipe locally. Debian users can use curl or Homebrew,
> but Debian is not supported. Windows is not supported.

Start with Ubuntu 24.04 and macOS 15 on ARM64 and x86_64. Add
Arch Linux and Omarchy on x86_64, tested against their current releases.
Record the tested Arch package versions and Omarchy version for each release.
Add other OS or CPU versions only after tests on those versions pass.
For Debian, permit a curl install with an unsupported-system notice.
Do not add Windows files, jobs, or install commands.

| System | curl | Homebrew | AUR |
| --- | --- | --- | --- |
| macOS | Yes | Yes | No |
| Ubuntu | Yes | Yes | No |
| Arch Linux | Yes | Yes | Recipe only; not published |
| Omarchy | Yes | Yes | Same local recipe; not published |
| Debian | Unsupported | Unsupported | No |
| Other Linux | Outside the initial curl scope | Possible where Homebrew requirements are met; untested | No project support |

Homebrew runs on Linux as well as macOS. Use one tap for both systems; users
must install Homebrew separately. See [Homebrew on Linux](https://docs.brew.sh/Homebrew-on-Linux).
Omarchy uses Arch packages and the AUR; it needs no separate executable or
package format. See the [Omarchy package guide](https://omarchy.org/manual/other-packages/).

Homebrew gives Intel Macs Tier 3 status and plans to remove Intel support in
or after September 2027. Test the tap on Intel while Homebrew permits it.
Keep curl as the install method for Intel Macs after that limit.
See [Homebrew support tiers](https://docs.brew.sh/Support-Tiers).

## Starting state

- `Cargo.toml` defines `lazyaws` version `0.1.0` with an MIT license.
- `Cargo.lock`, `LICENSE`, and the embedded theme file already exist.
- `src/main.rs` already handles `--help` and `--version` without a terminal.
- `.github/workflows/rust.yml` builds and tests on `ubuntu-latest` only.
- This checkout has no version tags, release workflow, installer, or formula.
  Remote release and package ownership still need checks.
- The README and landing page describe source builds. The landing page uses
  a Next.js static export on Cloudflare Workers Static Assets.
- AWS CLI v2 is an external runtime requirement. Rust is needed only for
  source builds. Demo mode needs no AWS CLI or credentials.

## Publication channels

Use one release build for all binary download channels. Cloud hosting and a
package registry have different roles. Extra cloud mirrors do not create
package listings.

| Channel | Stage | Publication process |
| --- | --- | --- |
| GitHub Releases | First release | Create a version tag and draft release. Attach archives, checksums, source, and notes. Publish after all checks pass. Enable immutable releases. |
| Cloudflare R2 | Later, after account activation | Upload the same archives and checksums. Use a download subdomain in the existing Cloudflare account. GitHub serves the first release. |
| Homebrew tap | First release | Create `capybara-brain346/homebrew-tap`. Add one macOS/Linux formula with OS and CPU selection. Test and update it for each release. |
| Arch User Repository (AUR) | Deferred by the owner | Check the `lazyaws-bin` name and ownership. Publish a tested `PKGBUILD` and generated `.SRCINFO` through AUR Git over SSH. Use it for Arch and Omarchy. |
| Omarchy package repository | After the AUR release | Request inclusion through the Omarchy package project if useful. Its maintainers control acceptance. AUR publication does not add a package to this repository. |
| crates.io | First release, if the name is available | Check ownership, package contents, and metadata. Run a publish dry run, then publish the source package. |
| Homebrew core | After the first stable release | Check current acceptance rules. Submit a source-build formula by pull request. Maintainers control acceptance. |
| MacPorts | After the first release | Add a source-build Portfile, checksums, license, and dependencies. Test it, then submit a pull request to `macports-ports`. |
| Ubuntu PPA / apt | Later expansion | Create a Launchpad account and PPA. Prepare and sign Ubuntu source packages. Upload with `dput`; Launchpad builds and publishes them. |
| Ubuntu archive | Later expansion | Follow the new-package review and sponsorship process. An upstream release does not make `apt install lazyaws` available. |
| Snap Store | Assess before packaging | Register the name, build a snap, test AWS CLI and credential access, upload, and release to a channel. Classic confinement needs review if required. |
| GitLab Releases / SourceForge | Extra mirrors only | Upload the same tested files if there is a user need. Keep GitHub as the release record. |

GitHub supports draft releases before immutable publication; use that order.
See [GitHub release publication](https://docs.github.com/en/repositories/releasing-projects-on-github/managing-releases-in-a-repository).

Homebrew taps can publish independently. Homebrew core requires an acceptable
source build and review. See [tap publication](https://docs.brew.sh/How-to-Create-and-Maintain-a-Tap),
[formula requirements](https://docs.brew.sh/Acceptable-Formulae), and
[acceptance policy](https://docs.brew.sh/Package-Acceptance-Policy).
For MacPorts, use the [contribution process](https://guide.macports.org/#project.contributing).

For AUR publication, register an account and SSH key, check existing packages,
then prepare `lazyaws-bin` with the versioned Linux archive URL, SHA-256,
`arch=('x86_64')`, MIT license, and `provides`/`conflicts` for `lazyaws`.
Install the binary and license into package paths. Do not run the curl script
from `PKGBUILD`. Build as a normal user in a clean Arch environment. Generate
`.SRCINFO` with `makepkg --printsrcinfo`, commit both files, and push to
`ssh://aur@aur.archlinux.org/lazyaws-bin.git`. Repeat for each stable release.
The AUR hosts the recipe; R2 hosts its upstream archive. A source-build
`lazyaws` package can follow if requested; do not create duplicate variants
for Omarchy. See the [AUR submission guide](https://wiki.archlinux.org/title/AUR_submission_guidelines)
and [AUR metadata guide](https://wiki.archlinux.org/title/.SRCINFO).
For optional Omarchy repository inclusion, use the
[Omarchy package project](https://github.com/omacom/omarchy-pkgs).

For Ubuntu, use [PPA uploads](https://ubuntu.com/docs/launchpad/user/how-to/packaging/ppa-package-upload/)
and [new-package review](https://ubuntu.com/project/docs/how-ubuntu-is-made/processes/new-packages/).
For Snap, use [publication](https://snapcraft.io/docs/releasing-your-app/) and
[classic confinement review](https://snapcraft.io/docs/reference/administration/reviewing-classic-confinement-snaps/).
The Snap access test is necessary because Lazy AWS runs the user's AWS CLI
and uses its configuration. Do not change that model just to obtain a listing.

Exclude Windows stores and Linux repositories outside the support scope. Do not add
npm or PyPI wrappers for this Rust executable. Do not add Docker Hub or GHCR
images to the first release: a container would need terminal, AWS CLI, and
credential setup beyond the requested local install.

## Cloudflare use

Keep the existing landing Worker. Serve `install.sh` and `latest.txt` as static
assets at `https://lazyaws.xyz`. Keep `www.lazyaws.xyz` attached to the Worker.
Use GitHub Releases for versioned downloads now. R2 is a later mirror after
the owner enables it. No database, queue, download API, or new Worker runtime
is required.

The release base URL is
`https://github.com/capybara-brain346/lazyaws/releases/download`.
When R2 is enabled, configure a download subdomain, copy and verify existing
release files, then change the release config and regenerate the installer
and package recipes. Keep existing GitHub release URLs available.

R2 custom domains support caching; `r2.dev` is for development. R2 has no
direct egress charge, but storage and operations can incur charges.
See [public bucket setup](https://developers.cloudflare.com/r2/buckets/public-buckets/)
and [R2 prices](https://developers.cloudflare.com/r2/pricing/).
Workers Static Assets have a 25 MiB file limit. R2 keeps binary storage separate
from site deployments. See [Workers limits](https://developers.cloudflare.com/workers/platform/limits/).

```text
Cargo.toml version + tag + source
               |
               v
       GitHub Actions build/tests
               |
               v
       archives + SHA256SUMS
               |
       +-------+------------------+
       |                          |
       v                          v
GitHub Release files       R2 mirror (later)
       |                          ^
       +---same files later-------+
       |
       +------------+-------------+-----------------+
                    |             |                 |
                    v             v                 v
             curl installer  Homebrew formula  AUR PKGBUILD
                    |         macOS + Linux     Arch + Omarchy
                    v             |                 |
             ~/.local/bin         v                 v
                            Homebrew prefix   pacman package

Source + package metadata --> crates.io --> cargo install --> local binary
```

## Ordered implementation

1. **Confirm publication access.** Check remote tags and releases. Check the
   crates.io name and owner, tap repository, AUR name and SSH access,
   Cloudflare domain, and R2 access.
   Use the repository's existing `0.1.0` only if it has not been published.
   Assign one maintainer to release updates and failed publications.
2. **Define release configuration.** Use `Cargo.toml` as the version source.
   Add one distribution config file for hosts, target mapping, OS limits,
   install defaults, and download timeouts. Generate public installer values
   from it; do not make the installer download and execute a second config file.
   Keep website commands and links in `apps/landing/src/config.ts`.
3. **Extend native CI.** Use pinned OS labels: `ubuntu-24.04`,
   `ubuntu-24.04-arm`, `macos-15`, and `macos-15-intel`. Run formatting,
   Clippy, and tests. Build with `cargo build --release --locked`.
   These labels are listed in the [GitHub runner reference](https://docs.github.com/en/actions/reference/runners/github-hosted-runners).
   Test the Linux x86_64 archive and AUR recipe in a clean Arch container.
   Test the installed program in an Omarchy terminal before each release.
   A container check does not replace that Omarchy check.
4. **Add release packaging.** Trigger on stable `vX.Y.Z` tags. Verify that the
   tag matches `Cargo.toml`. Package four target archives as shown below.
   Include `lazyaws`, `LICENSE`, and a short install note. Use `SHA256SUMS`
   for archive digests. Check the packaged binary, not only the build output.
5. **Add curl installation.** Implement the contract below. Test it against
   local fixture downloads before it can select a public release.
6. **Add the macOS/Linux tap and AUR package.** Use all four archives and their
   SHA-256 values in one custom formula. Add OS and architecture selection,
   MIT metadata, `bin.install`, and an installed-version test. A custom tap may
   install these binaries; a later core submission must build from source.
   Do not make Rust or AWS CLI install dependencies: the binary needs no Rust,
   and the demo works without AWS. State the AWS CLI v2 requirement in caveats.
   Follow the [Formula Cookbook](https://docs.brew.sh/Formula-Cookbook).
   Add the AUR recipe described above. Test Arch and Omarchy installs from
   that recipe. Do not claim that `pacman -S lazyaws` works until the package
   is in a configured binary repository.
7. **Publish the Rust source package.** Add repository, homepage, readme,
   keywords and categories to `Cargo.toml`. Set a minimum Rust version only
   after checking that compiler; the first release uses checked stable Rust.
   Use an explicit package file list that includes all source modules,
   `assets/themes.json`, the lockfile, license, and README. Exclude the landing
   app, captures, and generated files. Run `cargo package --list` and
   `cargo publish --dry-run --locked`. Publish the first version manually.
   Use the supplied Cargo credential as an environment secret for the manual
   publish workflow. Keep it out of PR jobs and source files. Follow
   [Cargo publication](https://doc.rust-lang.org/cargo/reference/publishing.html).
8. **Publish and test in order.** Upload all files to a draft GitHub release.
   Verify all four archive digests and publish the release. Verify its public
   downloads before updating `latest.txt` on the site. Update and test the tap
   and AUR recipe. Add an R2 mirror only after account activation. Record success per
   channel and retry a failed channel without rebuilding or replacing a
   published version.
9. **Update user instructions.** Put curl and Homebrew before source-build
   steps. Use `lazyaws --demo` for the installed demo. State OS support and AWS
   requirements. Add the local Arch/Omarchy recipe and Linux Homebrew command.
   Update website copy tests and both installation guides.
10. **Request reviewed listings.** Submit Homebrew core and MacPorts packages
    after the first stable release passes. Track PPA, Ubuntu archive, and Snap
    work separately. State which listings are live; do not advertise pending ones.

| Target | Archive suffix |
| --- | --- |
| `aarch64-apple-darwin` | `aarch64-apple-darwin.tar.gz` |
| `x86_64-apple-darwin` | `x86_64-apple-darwin.tar.gz` |
| `aarch64-unknown-linux-gnu` | `aarch64-unknown-linux-gnu.tar.gz` |
| `x86_64-unknown-linux-gnu` | `x86_64-unknown-linux-gnu.tar.gz` |

Use names such as `lazyaws-v0.1.0-aarch64-apple-darwin.tar.gz` under
`https://github.com/capybara-brain346/lazyaws/releases/download/v0.1.0/`.
Ubuntu 24.04 is the initial Linux
build baseline. Reuse its Linux archives for Arch and Omarchy after compatibility
tests pass. A Debian install can fail if its system libraries are too old.

The planned package commands, after publication, are:

```sh
# macOS and Linux with Homebrew installed
brew install capybara-brain346/tap/lazyaws

# Arch Linux or Omarchy: build the supplied recipe locally
cd packaging/aur/lazyaws-bin
makepkg -si
```

AUR publication is deferred. Do not advertise `yay -S lazyaws-bin` until a
public listing exists. The generated recipe uses the final release checksums.

## Curl contract

The planned public command is:

```sh
curl -fsSL https://lazyaws.xyz/install.sh | sh
```

Also document download, inspection, and `sh install.sh` as separate commands.
Support an optional version argument, for example `sh install.sh v0.1.0`.

- Use POSIX `sh`, curl, tar, and the OS SHA-256 tool. Do not require jq or Rust.
- Select macOS or Linux with `uname`; map `arm64`/`aarch64` and
  `x86_64`/`amd64`. On Linux, read OS identification. Accept Ubuntu and the
  tested Arch/Omarchy systems; Omarchy can identify as Arch. Warn for Debian
  and stop for other distributions. Do not treat every Arch derivative as
  supported. Reject other OS and untested OS/CPU combinations.
- With no version, fetch `latest.txt` once. Validate its version format.
  Then fetch the archive and checksums from that exact version directory.
- Use HTTPS, bounded downloads, HTTP error checks, a private temporary
  directory, and cleanup traps. Verify SHA-256 before extraction or execution.
  Checksums detect damaged files; checksums on the same host do not prove
  publisher identity if that host is compromised.
- Install to `~/.local/bin/lazyaws` without sudo. Allow an explicit destination.
  Validate the archive entries and install only the expected regular executable.
  Check its version, then replace the destination with a same-directory rename.
  On failure, keep the old binary. Do not replace a binary managed by Homebrew
  or pacman.
- Print the installed version and any PATH instruction. Do not edit shell files,
  install AWS CLI, or access credentials. Re-run the script to upgrade; pass a
  prior version to roll back. Remove the installed file to uninstall.

```text
User          Installer           Site/R2              Local disk
 |--run---------->|                   |                     |
 |               |--GET version----->|                     |
 |               |<--exact version---|                     |
 |               |--GET files------->|                     |
 |               |<--archive+hashes---|                     |
 |               |--verify, stage, check version----------->|
 |               |--rename only after success------------->|
 |<--version-----|                   |                     |
```

Keep versioned files immutable with long cache lifetimes. Give `latest.txt`
and `install.sh` short cache lifetimes. Configure cache rules explicitly.
Exclude download paths from browser challenges. Publish no prerelease as latest.
For a bad release, restore the previous latest pointer, purge that cache entry,
and publish a new version. Homebrew, AUR, and Cargo changes need separate recovery;
a latest-pointer change does not downgrade existing installs.

## Files and call paths

| Reach | Planned files |
| --- | --- |
| Release values and scripts | New `distribution/config.json`, focused scripts under `scripts/`, generated `apps/landing/public/install.sh` |
| CI | Existing `.github/workflows/rust.yml`; new `.github/workflows/release.yml` |
| Rust package metadata | `Cargo.toml`; update `Cargo.lock` when the package version changes |
| Website | `apps/landing/src/config.ts`, `components/install-guide.tsx`, `tests/landing.spec.ts` |
| Documentation | `README.md`, `apps/landing/README.md`, release operations guide |
| External tap | `homebrew-tap/Formula/lazyaws.rb`, tap README and checks |
| AUR recipe | New `packaging/aur/lazyaws-bin/PKGBUILD` and `.SRCINFO`; publish to the AUR Git repository |
| Test data | Installer fixtures only; existing AWS fixtures need no change |

The current callers and callees were checked in the source. The website call
path remains the same. Its command data and requirement text change.

```text
Before
Home -> InstallGuide -> installation.map -> CopyButton(command)
  command data: git clone / cargo run / cargo build

After
Home -> InstallGuide -> installation.map -> CopyButton(command) [same calls]
- command data: source build as the first install path
+ command data: curl / brew install / AUR install / lazyaws --demo

Unchanged executable path
main -> Config::parse -> Config::parse_with_env
main -> Cli::Version -> println!(CARGO_PKG_VERSION)
main -> Cli::Run -> runtime::run
```

No Rust function change is needed for publication. The existing version path
supplies the release and installer check.

## Release acceptance

- Formatting, Clippy, tests, and locked release builds pass on the target matrix.
- Extracted binaries pass `--version` and `--help`. Run demo mode in a terminal
  and exit normally on each target. No AWS credentials are needed.
- Installer checks cover Ubuntu, Arch/Omarchy detection, Debian notice, rejected systems,
  checksum failure, failed download, and an upgrade that keeps the old file on
  failure. Use small local fixtures; do not duplicate the AWS adapter tests.
- Test the tap install, version, upgrade, and uninstall on both Mac architectures
  and both Ubuntu architectures. Check the x86_64 tap on Arch and Omarchy.
  Run `brew audit --strict` and `brew test` for the tap formula.
- Build and check the AUR package in a clean Arch environment. Check `.SRCINFO`
  against `PKGBUILD`, run `namcap`, and test install, upgrade, and removal.
  Check that the installed file belongs to the package. Test the same package
  on Omarchy, including demo mode and normal terminal exit.
- Test copied website commands and support text. Run the landing app's lint,
  type check, build, and browser tests.
- Verify package contents, public URLs, response codes, SHA-256 values, and
  exact version agreement across GitHub, Homebrew, AUR, and crates.io. Check R2
  in the same way when it is enabled.
- Limit release credentials to the release jobs: GitHub release write access,
  bucket-scoped R2 writes, tap-repository writes, and the AUR maintainer SSH key.
  Keep the AUR key in the release environment. Do not expose these credentials
  to PR jobs. Pin external release actions to reviewed commits.
- Test downloads on a clean Mac. If macOS blocks execution, resolve code signing
  and notarization before claiming that install path works. Do not instruct
  users to disable OS protection.

Record completed checks and publication URLs in `docs/releases.md`.
AUR publication and R2 are deferred by the owner. Apple signing access and
external listing acceptance were not required for the completed curl/Homebrew
release and remain unverified. ArchWiki's full submission page was blocked
during research; recheck its current rules before publication.
