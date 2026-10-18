#!/usr/bin/env bash
set -euo pipefail

TAG="${1:?missing release tag, e.g. v0.1.0}"
TAP_DIR="${2:?missing tap repo path}"
VERSION="${TAG#v}"

cd "$TAP_DIR"

declare -A urls=(
  [macos-arm]="https://github.com/PetukhovArt/stratum/releases/download/${TAG}/stratum-lint-aarch64-apple-darwin.tar.gz"
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
  license "MIT"

  on_macos do
    depends_on arch: :arm64
    url "${urls[macos-arm]}"
    sha256 "${hashes[macos-arm]}"
  end

  on_linux do
    depends_on arch: :x86_64
    url "${urls[linux-x64]}"
    sha256 "${hashes[linux-x64]}"
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
