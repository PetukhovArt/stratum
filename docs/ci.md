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

## A C4 diagram in an MR description

GitLab and GitHub render a ```` ```mermaid ```` block in markdown, C4 diagrams included. Print one and paste it into the MR or PR description:

```
stratum-lint c4                                       # containers: the top-level layers
stratum-lint c4 --level system                        # needs `systems` in the config
stratum-lint c4 --feature checkout                    # only what one feature reaches
```

Wrap the output in a fence:

````
```mermaid
C4Container
title Containers of Web UI
...
```
````

The container level shows one box per top-level layer; layers nested in another layer's folder count for it. To look inside one, use `--level component --container <layer id>`: one box per folder or file directly in that layer, at most 60 (the smallest fold into one "… N more"). For a narrower view, use `--feature`. The Mermaid output places the boxes and moves the labels itself (`UpdateRelStyle`, transparent filler boxes), so arrows stay out of boxes and labels apart; keep those lines when you paste it. At the container level, the other systems sit in an untitled dashed boundary below the system's.

Mermaid lays the boxes out in the reader's browser and starts a new row where a box would pass the screen width (`screen.availWidth`, the screen in CSS pixels, so a 1920-px screen at 150 % scaling counts as 1280). The output keeps every row at 2 or 3 boxes, which fit a screen 1280 px wide; on a narrower screen the rows wrap and arrows can cross boxes. A headless browser that renders the diagram (a screenshot job, for one) reports an 800-px screen unless told otherwise: start Chrome with `--screen-info={0,0 1920x1080}`.

## Reproducing a pipeline locally

```
stratum-lint --format agent
stratum-lint baseline --check=origin/main
stratum-lint diff --base origin/main
```
