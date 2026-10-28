# Release procedure

This document walks through cutting a `stratum-lint` / `stratum-lsp` release. Treat it as the source of truth — when something changes, update this file in the same PR.

## Pre-flight

1. Workspace is clean: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`.
2. Frontend builds: `cd frontend/stratum-visualizer-frontend && npm install && npm run build`. `dist/` exists and is under 1.5 MB (CI gate).
3. Performance budget: run `cargo run --release -p stratum-budget -- check` on Windows and on macOS; it must pass on both (budgets: [docs/harness.md](docs/harness.md#budgets), method: [docs/calibration.md](docs/calibration.md#budget-project)). The maintainer runs the macOS one.
4. Optional dry run: start the `release` workflow by hand (`workflow_dispatch`). It builds every target and packs the npm tarballs, and publishes nothing.

## Cut the release

A version bump landing on `main` is the release. Versions follow SemVer; while 0.x, a minor bump carries new features or breaking changes and a patch only fixes. There are no pre-release versions: the dry run above replaces them.

1. Bump the version: every `crates/*/Cargo.toml`, both `claude-plugin*/.claude-plugin/plugin.json` and the `@petukhovart/stratum-lint@X.Y.Z` pin in `ci/github/action.yml` and `ci/gitlab/templates/stratum.yml`. `node scripts/check-version.mjs` fails until all of them agree.
2. In `CHANGELOG.md`, rename `## Unreleased` to `## X.Y.Z (YYYY-MM-DD)` and open a new empty `## Unreleased` above it. `node scripts/release-notes.mjs X.Y.Z` prints what becomes the release notes.
3. Commit `chore(release): X.Y.Z` and push to `main`.
4. `.github/workflows/release.yml` runs:
   - `gate` checks the versions agree and that `vX.Y.Z` is not tagged yet, and that the changelog section exists;
   - `build` runs `cargo build --release` per target and packs `stratum-lint-<target>.tar.gz` / `.zip` with both binaries; each binary must stay under 25 MB;
   - `npm-pack` turns the archives into the platform packages and the launcher (`scripts/npm-pack.mjs`);
   - `release` tags the commit `vX.Y.Z` and writes the GitHub Release with the changelog section as notes and the archives attached;
   - `npm` publishes each `@petukhovart/stratum-lint-<platform>-<arch>`, then the launcher (secret `NPM_TOKEN`, `--provenance`).
5. Verify the GitHub Release lists the archive of every target, and npm lists the new version of the launcher and of each platform package.

A target added to the `build` matrix also needs its npm `os`/`cpu` in `NPM_PLATFORMS` (`scripts/npm-pack.mjs`), which fails before publishing on a triple it does not know.

## Downstream package updates

After the GitHub Release exists:

No release job updates the tap or the bucket: both are bumped by hand with the scripts below. Releases build three targets (Linux x64, Windows x64, macOS arm64), so Homebrew serves macOS arm64 and Linux x64 and Scoop serves Windows x64.

1. **Homebrew tap** — `scripts/bump_homebrew.sh vX.Y.Z /path/to/homebrew-stratum-tap` recomputes the `sha256` per artefact and commits to the tap. Then `cd /path/to/homebrew-stratum-tap && git push`.
2. **Scoop bucket** — `scripts/bump_scoop.sh vX.Y.Z /path/to/scoop-stratum-bucket`, then push.

## Smoke test

### npm packages, before tagging

Install must work with `--ignore-scripts` and with no host but the registry. With the `stratum-lint-<target>` archives in a folder (a dry run's `archive-*` artifacts):

```bash
node scripts/npm-pack.mjs <archives-dir> X.Y.Z target/npm
dir=$(mktemp -d) && cd "$dir" && npm init -y > /dev/null
# launcher + this platform's package; --offline proves no registry or GitHub fetch happens
npm install --offline --ignore-scripts --no-audit --no-fund \
  /path/to/stratum/target/npm/petukhovart-stratum-lint-X.Y.Z.tgz \
  /path/to/stratum/target/npm/petukhovart-stratum-lint-<platform>-<arch>-X.Y.Z.tgz
npx stratum-lint --help
npx stratum-lsp < /dev/null    # starts, exits when stdin closes
# missing platform package: exit 2 naming it
rm -rf node_modules/@petukhovart/stratum-lint-<platform>-<arch>
npx stratum-lint --help; echo "exit $?"
```

To test resolution through a registry proxy, publish the tarballs to a local Verdaccio (`npx verdaccio`, then `npm publish <tarball> --registry http://localhost:4873`) and `npm install --ignore-scripts --registry http://localhost:4873 @petukhovart/stratum-lint@X.Y.Z`.

### Released artefacts

On a clean machine (or VM):

```bash
# Linux x64
curl -L https://github.com/PetukhovArt/stratum/releases/download/vX.Y.Z/stratum-lint-x86_64-unknown-linux-gnu.tar.gz | tar -xz
./stratum-lint --help
test -x ./stratum-lsp    # the archive bundles stratum-lsp too

# npm install path
npm i -g --ignore-scripts @petukhovart/stratum-lint
stratum-lint --help
command -v stratum-lsp

# Homebrew (macOS arm64 or Linux x64)
brew tap petukhovart/stratum
brew install stratum-lint
stratum-lint --help

# Scoop (Windows PowerShell)
scoop bucket add stratum https://github.com/PetukhovArt/scoop-stratum-bucket
scoop install stratum-lint
stratum-lint --help
```

## Rollback

If a release is broken:

1. `gh release delete vX.Y.Z --yes`
2. `git push --delete origin vX.Y.Z` (rare — usually keep the tag for a postmortem).
3. The bump scripts only need to be re-run after a fixed release is cut; the tap / bucket / npm tags are immutable once published.
4. **Never** force-push over a released tag.

## Release cadence

- Patch releases (`0.X.Z+1`): bug fixes, doc updates.
- Minor releases (`0.X+1.0`): new rules, new CLI flags, snapshot version unchanged.
- Major releases (`0.X+1` post-1.0): snapshot version bump, breaking API changes.

The **Graph Snapshot** version (currently `3`) is the public contract between the linter (`snapshot`, `visualize`), the Visualizer and external tooling; bumping it requires a coordinated frontend release.
The other versioned shapes are `--format json` (currently `2`), the baseline file (`2`; version `1` is still read), `rules` / `explain` JSON (`1`) and the `context` brief JSON (`2`). A breaking change to one bumps its `version` and is listed under **Breaking** in `CHANGELOG.md`.
