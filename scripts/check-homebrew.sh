#!/usr/bin/env bash
# Install the published Homebrew formula on a temporary runner and check it.
# Usage: scripts/check-homebrew.sh FORMULA NAME TAG
# Example: scripts/check-homebrew.sh capybara-brain346/tap/lazyaws lazyaws v0.2.2
set -euo pipefail

formula=$1
name=$2
tag=$3

# Homebrew/install commit used when the runner has no Homebrew.
brew_install_commit=7a3f48c7e498d7df3237fd41c30f4d492857edf4

if [[ "$RUNNER_OS" == Linux ]]; then
  brew_bin=/home/linuxbrew/.linuxbrew/bin
  brew_sbin=/home/linuxbrew/.linuxbrew/sbin
elif [[ "$(uname -m)" == arm64 ]]; then
  brew_bin=/opt/homebrew/bin
  brew_sbin=/opt/homebrew/sbin
else
  brew_bin=/usr/local/bin
  brew_sbin=/usr/local/sbin
fi

if ! command -v brew >/dev/null 2>&1 && [[ ! -x "$brew_bin/brew" ]]; then
  curl -fsSL \
    "https://raw.githubusercontent.com/Homebrew/install/$brew_install_commit/install.sh" \
    -o "$RUNNER_TEMP/homebrew-install.sh"
  NONINTERACTIVE=1 /bin/bash "$RUNNER_TEMP/homebrew-install.sh"
fi

brew_path="$(command -v brew || true)"
if [[ -z "$brew_path" ]]; then
  brew_path="$brew_bin/brew"
fi
[[ -x "$brew_path" ]] || {
  echo "Homebrew is not available at $brew_path" >&2
  exit 1
}
brew_dir="$(dirname "$brew_path")"
export PATH="$brew_bin:$brew_sbin:$brew_dir:$PATH"

brew --version
brew install --formula "$formula"
test "$(brew list --versions "$formula")" = "$name ${tag#v}"
brew test "$formula"
brew audit --strict "$formula"
