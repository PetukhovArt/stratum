# Stratum in CI

A **CI Template** runs Stratum on every MR/PR and on the default branch: findings in the CI's own format, the baseline ratchet against the MR's base, and an advisory drift report. Job layout, snippets and inputs: [ci/README.md](../ci/README.md). Exit codes and output shapes: [harness.md](harness.md).

`stratum/co-change` runs only on a diff: the templates show it in the drift report, which is advisory. To gate on it, add a step `stratum-lint --changed="$BASE" --format agent` against the merge base.

## Before the first pipeline

1. Commit `stratum.config.jsonc` (`stratum-lint init`) and, if the project has existing violations, `stratum.baseline.json` (`stratum-lint baseline`). A baseline added in the same MR counts as introduced and passes.
2. Make `stratum-lint` resolvable without the internet (next section).
3. Wire the platform: GitLab or GitHub below.

## Getting the binary without outbound internet

Neither the templates nor the binary reach a public host: the templates talk only to the CI instance itself and to the npm registry the job is already configured with, and `stratum-lint` makes no network calls.

- The binary comes from npm: the launcher `@petukhovart/stratum-lint` plus one platform package it selects through `optionalDependencies` (`@petukhovart/stratum-lint-linux-x64` on a Linux runner). Behind a company registry proxy (Nexus, Artifactory, Verdaccio), the proxy must serve both from npmjs.
- Pin it in the project to choose the version yourself and skip the template's install step: `npm install -D @petukhovart/stratum-lint@0.2.0`. The template uses `node_modules/.bin/stratum-lint` when it exists.
- Without a pin, the template installs its own tagged version from the job's registry config: env, user `.npmrc`, or the project's `.npmrc`.

## GitLab

1. Import or fork the stratum repository into your GitLab, tags included: consumers reference it as `<group>/stratum`.
2. In the consuming project, `include: project: <group>/stratum` with `ref: v0.5.0` and `file: ci/gitlab/templates/stratum.yml` ([snippet](../ci/README.md#gitlab)). Jobs `stratum` (gate, Code Quality widget) and `stratum-drift` (advisory) appear in MR pipelines; `stratum` also runs on the default branch.
3. Set `image:` / `tags:` on both jobs if the default runner lacks node >= 18 and git.
4. MR note with the drift report: add the masked CI/CD variable `STRATUM_GITLAB_TOKEN`, a project access token with the `api` scope. Without it the report is only the "stratum drift" artifact on the MR.
5. Internal or self-signed CA on the GitLab instance: set `NODE_EXTRA_CA_CERTS` to the CA bundle in the image, or the note request fails TLS.

The template picks the base commit itself (merged-results pipelines included, [details](../ci/README.md#gitlab)) and fetches it into the shallow clone.

## GitHub

`uses: <org>/stratum/ci/github@v0.5.0` after `actions/checkout`, with `pull-requests: write` for the drift comment ([workflow](../ci/README.md#github-githubcom-and-github-enterprise-server)). Findings become annotations on the PR diff; the drift report goes to the job summary and to one PR comment. On GitHub Enterprise Server, `<org>/stratum` is the repository mirrored onto the instance.

## Accepting debt

The ratchet fails when a baseline entry's count grows, and a moved file counts as growth because the path is part of the fingerprint. For an MR that moves files or takes on debt on purpose:

1. Regenerate the baseline in the MR (`stratum-lint baseline`) and commit it.
2. Add the label `stratum:accept-debt`: the template skips `baseline --check` ([when the label takes effect](../ci/README.md#stratum-ci-templates)).

## Reproducing a pipeline locally

```
stratum-lint --format agent
stratum-lint baseline --check=origin/main
stratum-lint diff --base origin/main
```
