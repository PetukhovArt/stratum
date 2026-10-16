# Release procedure

This document walks through cutting a `stratum-lint` / `stratum-lsp` release. Treat it as the source of truth — when something changes, update this file in the same PR.

## Pre-flight

1. Workspace is clean: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`.
2. Frontend builds: `cd frontend/stratum-visualizer-frontend && npm install && npm run build`. `dist/` exists and is under 1.5 MB (CI gate).
3. Local `cargo dist plan` succeeds (`cargo install cargo-dist --locked` if missing).
4. Pick the new semantic version `X.Y.Z`. Pre-releases use `X.Y.Z-rc.N`.

## Cut the release

1. Bump versions:
   - Every `crates/*/Cargo.toml`'s `version = ...`. The npm packages take theirs from cargo-dist's `dist-manifest.json`.
   - The `@petukhovart/stratum-lint@X.Y.Z` pin in `ci/github/action.yml` and `ci/gitlab/templates/stratum.yml`.
   - `claude-plugin/.claude-plugin/plugin.json` and `claude-plugin-lsp/.claude-plugin/plugin.json` `version`.
2. Update `CHANGELOG.md` — move "Unreleased" content under the new version + date.
3. Commit: `chore(release): vX.Y.Z`.
4. Tag the commit: `git tag vX.Y.Z && git push origin main vX.Y.Z`.
5. `.github/workflows/release.yml` fires. Watch the run:
   - `plan` job runs `cargo dist plan`.
   - `build` job runs `cargo dist build` per target on the right runner.
   - Each artifact must stay under 25 MB (CI ceiling; PRD target is 20 MB).
   - `publish` job uploads to the GitHub Release for the tag.
   - `publish-npm` job runs `scripts/npm-pack.mjs` on the archives and publishes each `@petukhovart/stratum-lint-<platform>-<arch>`, then the launcher `@petukhovart/stratum-lint` (secret `NPM_TOKEN`, `--provenance`; a version with `-` goes to the `next` dist-tag).
6. Verify the GitHub Release lists the archive of every target in `dist-workspace.toml`, and npm lists the new version of the launcher and of each platform package.

`release.yml` is written by hand, not by `dist generate` (`allow-dirty = ["ci"]` lets cargo-dist accept it). Never run `dist generate` or `dist init` with CI enabled: it overwrites the file and drops `publish-npm`. A target added to `dist-workspace.toml` also needs its row in the `build` matrix; `npm-pack.mjs` fails before publishing when an archive in the manifest is missing, and names a triple it has no npm `os`/`cpu` for.

## Downstream package updates

After the GitHub Release exists:

1. **Homebrew tap** — `scripts/bump_homebrew.sh vX.Y.Z /path/to/homebrew-stratum-tap` recomputes the `sha256` per artefact and commits to the tap. Then `cd /path/to/homebrew-stratum-tap && git push`.
2. **Scoop bucket** — `scripts/bump_scoop.sh vX.Y.Z /path/to/scoop-stratum-bucket`, then push.

## Smoke test

### npm packages, before tagging

Install must work with `--ignore-scripts` and with no host but the registry. With the archives and `dist-manifest.json` in `target/distrib` (from `cargo dist build`, or a release run's artifacts):

```bash
node scripts/npm-pack.mjs target/distrib target/npm
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

# Homebrew (macOS)
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

The **Graph Snapshot** version (currently `2`) is the public contract between the linter (`snapshot`, `visualize`), the Visualizer and external tooling; bumping it requires a coordinated frontend release.
The other versioned shapes are `--format json` (currently `2`), the baseline file (`1`), `rules` / `explain` JSON (`1`) and the `context` brief JSON (`2`). A breaking change to one bumps its `version` and is listed under **Breaking** in `CHANGELOG.md`.
