#!/usr/bin/env bash
# Update the Scoop bucket manifest after a release.
# Usage: scripts/bump_scoop.sh v0.1.0 /path/to/scoop-stratum-bucket
set -euo pipefail

TAG="${1:?missing release tag, e.g. v0.1.0}"
BUCKET_DIR="${2:?missing bucket repo path}"
VERSION="${TAG#v}"

URL="https://github.com/PetukhovArt/stratum/releases/download/${TAG}/stratum-lint-x86_64-pc-windows-msvc.zip"
tmp=$(mktemp)
curl -fsSL "$URL" -o "$tmp"
HASH=$(shasum -a 256 "$tmp" | awk '{print $1}')
rm -f "$tmp"

cd "$BUCKET_DIR"
cat > stratum-lint.json <<EOF
{
  "version": "${VERSION}",
  "description": "Stratum architectural linter",
  "homepage": "https://github.com/PetukhovArt/stratum",
  "license": "MIT|Apache-2.0",
  "architecture": {
    "64bit": {
      "url": "${URL}",
      "hash": "${HASH}"
    }
  },
  "bin": ["stratum-lint.exe", "stratum-lsp.exe"],
  "checkver": "github",
  "autoupdate": {
    "architecture": {
      "64bit": {
        "url": "https://github.com/PetukhovArt/stratum/releases/download/v\$version/stratum-lint-x86_64-pc-windows-msvc.zip"
      }
    }
  }
}
EOF

git add stratum-lint.json
git commit -m "stratum-lint ${VERSION}"
echo "ready to push bucket: cd ${BUCKET_DIR} && git push"
