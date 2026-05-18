#!/usr/bin/env bash
# Update the Homebrew tap formula after a release.
# Usage: scripts/bump_homebrew.sh v0.1.0 /path/to/homebrew-stratum-tap
set -euo pipefail

TAG="${1:?missing release tag, e.g. v0.1.0}"
TAP_DIR="${2:?missing tap repo path}"
VERSION="${TAG#v}"

cd "$TAP_DIR"

declare -A urls=(
  [macos-arm]="https://github.com/PetukhovArt/stratum/releases/download/${TAG}/stratum-lint-aarch64-apple-darwin.tar.gz"
  [macos-x64]="https://github.com/PetukhovArt/stratum/releases/download/${TAG}/stratum-lint-x86_64-apple-darwin.tar.gz"
  [linux-arm]="https://github.com/PetukhovArt/stratum/releases/download/${TAG}/stratum-lint-aarch64-unknown-linux-gnu.tar.gz"
  [linux-x64]="https://github.com/PetukhovArt/stratum/releases/download/${TAG}/stratum-lint-x86_64-unknown-linux-gnu.tar.gz"
)

declare -A hashes=()
for k in "${!urls[@]}"; do
  echo "fetching ${urls[$k]}"
  tmp=$(mktemp)
  curl -fsSL "${urls[$k]}" -o "$tmp"
  hashes[$k]=$(shasum -a 256 "$tmp" | awk '{print $1}')
  rm -f "$tmp"
done

cat > Formula/stratum-lint.rb <<EOF
class StratumLint < Formula
  desc "Stratum architectural linter"
  homepage "https://github.com/PetukhovArt/stratum"
  version "${VERSION}"
  license any_of: ["MIT", "Apache-2.0"]

  on_macos do
    if Hardware::CPU.arm?
      url "${urls[macos-arm]}"
      sha256 "${hashes[macos-arm]}"
    else
      url "${urls[macos-x64]}"
      sha256 "${hashes[macos-x64]}"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "${urls[linux-arm]}"
      sha256 "${hashes[linux-arm]}"
    else
      url "${urls[linux-x64]}"
      sha256 "${hashes[linux-x64]}"
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
EOF

git add Formula/stratum-lint.rb
git commit -m "stratum-lint ${VERSION}"
echo "ready to push tap: cd ${TAP_DIR} && git push"
