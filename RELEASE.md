# Release procedure

This document walks through cutting a `stratum-lint` / `stratum-lsp` release. Treat it as the source of truth — when something changes, update this file in the same PR.

## Pre-flight

1. Workspace is clean: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`.
2. Frontend builds: `cd frontend/stratum-visualizer-frontend && npm install && npm run build`. `dist/` exists and is under 1.5 MB (CI gate).
3. Local `cargo dist plan` succeeds (`cargo install cargo-dist --locked` if missing).
4. Pick the new semantic version `X.Y.Z`. Pre-releases use `X.Y.Z-rc.N`.

## Cut the release

1. Bump versions:
   - Every `crates/*/Cargo.toml`'s `version = ...`.
   - `npm-wrapper/package.json` `version`.
2. Update `CHANGELOG.md` — move "Unreleased" content under the new version + date.
3. Commit: `chore(release): vX.Y.Z`.
4. Tag the commit: `git tag vX.Y.Z && git push origin main vX.Y.Z`.
5. `.github/workflows/release.yml` fires. Watch the run:
   - `plan` job runs `cargo dist plan`.
   - `build` job runs `cargo dist build` per target on the right runner.
   - Each artifact must stay under 25 MB (CI ceiling; PRD target is 20 MB).
   - `publish` job uploads to the GitHub Release for the tag.
6. Verify the GitHub Release page lists all five targets:
   - `stratum-lint-x86_64-pc-windows-msvc.zip`
   - `stratum-lint-x86_64-apple-darwin.tar.gz`
   - `stratum-lint-aarch64-apple-darwin.tar.gz`
   - `stratum-lint-x86_64-unknown-linux-gnu.tar.gz`
   - `stratum-lint-aarch64-unknown-linux-gnu.tar.gz`

## Downstream package updates

After the GitHub Release exists:

1. **Homebrew tap** — `scripts/bump_homebrew.sh vX.Y.Z /path/to/homebrew-stratum-tap` recomputes the `sha256` per artefact and commits to the tap. Then `cd /path/to/homebrew-stratum-tap && git push`.
2. **Scoop bucket** — `scripts/bump_scoop.sh vX.Y.Z /path/to/scoop-stratum-bucket`, then push.
3. **npm wrapper** — `cd npm-wrapper && npm publish --access public`. The `postinstall` script fetches the binary that matches `package.json`'s version against the GitHub release URL.

## Smoke test

On a clean machine (or VM):

```bash
# Linux x64
curl -L https://github.com/PetukhovArt/stratum/releases/download/vX.Y.Z/stratum-lint-x86_64-unknown-linux-gnu.tar.gz | tar -xz
./stratum-lint --help

# npm install path
npm i -g @stratum/lint
stratum-lint --help

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

The **Graph Snapshot** version (currently `1`) is the public contract between the linter and the visualizer / LSP / external tooling; bumping it requires a coordinated frontend release.
