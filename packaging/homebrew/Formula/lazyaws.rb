class Lazyaws < Formula
  desc "Lazygit-style terminal interface for AWS"
  homepage "https://lazyaws.xyz"
  version "0.1.0"
  license "MIT"

  on_macos do
    depends_on macos: :sequoia
    on_arm do
      url "https://github.com/capybara-brain346/lazyaws/releases/download/v0.1.0/lazyaws-v0.1.0-aarch64-apple-darwin.tar.gz"
      sha256 "146e9cee7a9b43d776ea5f24bd4a87a447cbee5a98da1b26041c46401c9f67e6"
    end
    on_intel do
      url "https://github.com/capybara-brain346/lazyaws/releases/download/v0.1.0/lazyaws-v0.1.0-x86_64-apple-darwin.tar.gz"
      sha256 "b1ee5033f0b0d2dc94441bcbf9d0bcf6022e1b9bc74319284a32ed732708ece1"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/capybara-brain346/lazyaws/releases/download/v0.1.0/lazyaws-v0.1.0-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "df316d13f2c24c88a01f95bf11b0cfceaf7d2d5aa2b9a8fb38a46f50b078795c"
    end
    on_intel do
      url "https://github.com/capybara-brain346/lazyaws/releases/download/v0.1.0/lazyaws-v0.1.0-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "5e9405338264b12f8c38b06b59c4985d39426c377dbb5705eced6ef63444e675"
    end
  end

  def install
    bin.install "lazyaws"
  end

  def caveats
    <<~EOS
      Normal AWS service use requires AWS CLI v2.
      Demo mode works without AWS CLI or credentials.

      Linux binaries target Ubuntu 24.04 and systems with a compatible libc.
    EOS
  end

  test do
    assert_match "lazyaws #{version}", shell_output("#{bin}/lazyaws --version")
  end
end
