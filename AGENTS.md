# Stratum

- No comments in code, doc comments included. Allowed: tool directives (`prettier-ignore`, `@stratum-purity N`, `stratum-ignore`, `/// <reference`), `upstream: <url>` for behaviour forced by code outside this repo, and `///` text that clap or schemars turn into `--help` or `stratum.schema.json`. Say the rest with a name, a type, a test or a lint rule.
- `node scripts/check-comments.mjs` checks the lines added since `origin/main` (`--staged`, `--base <ref>`, `--all`); CI runs it on every pull request. `git config core.hooksPath .githooks` runs it on staged lines before each commit.
