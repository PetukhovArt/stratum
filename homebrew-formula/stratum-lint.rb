# Documentation copy of the Homebrew formula.
# The real file lives in the tap repo `homebrew-stratum-tap`.
# `scripts/bump_homebrew.sh` updates it after each release.

class StratumLint < Formula
  desc "Stratum architectural linter — compound-DAG layer checks for TS/Vue"
  homepage "https://github.com/PetukhovArt/stratum"
  version "0.1.0"
  license any_of: ["MIT", "Apache-2.0"]

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/PetukhovArt/stratum/releases/download/v0.1.0/stratum-lint-aarch64-apple-darwin.tar.gz"
      sha256 "FILL_AT_RELEASE"
    else
      url "https://github.com/PetukhovArt/stratum/releases/download/v0.1.0/stratum-lint-x86_64-apple-darwin.tar.gz"
      sha256 "FILL_AT_RELEASE"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/PetukhovArt/stratum/releases/download/v0.1.0/stratum-lint-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "FILL_AT_RELEASE"
    else
      url "https://github.com/PetukhovArt/stratum/releases/download/v0.1.0/stratum-lint-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "FILL_AT_RELEASE"
    end
  end

  def install
    bin.install "stratum-lint"
    bin.install "stratum-lsp" if File.exist?("stratum-lsp")
  end

  test do
    assert_match "stratum-lint", shell_output("#{bin}/stratum-lint --help")
  end
end
