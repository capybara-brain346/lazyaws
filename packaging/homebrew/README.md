# Homebrew tap

The generated formula targets `lazyaws` for macOS 15 or newer and Linux on
ARM64 or x86_64. The [public tap](https://github.com/capybara-brain346/homebrew-tap)
contains the verified `v0.1.0` formula.
AWS CLI v2 is needed for AWS operations. Demo mode works without AWS CLI or
credentials.

After the release archives and their `SHA256SUMS` are public, run this from the
repository root:

```sh
node scripts/generate-packages.mjs /path/to/SHA256SUMS
```

Copy this README to the root and `packaging/homebrew/Formula/lazyaws.rb` to
`Formula/lazyaws.rb` in the `capybara-brain346/homebrew-tap` repository. The
formula uses the version in `Cargo.toml`, URLs from `distribution/config.json`,
and only checksums from the release manifest.

Users install it with:

```sh
brew install capybara-brain346/tap/lazyaws
```
