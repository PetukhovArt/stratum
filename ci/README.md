# Stratum CI templates

Two adapters around the same sequence: lint with findings in the CI's own format, `baseline --check` against the merge base, and on merge requests an advisory drift report (`stratum-lint diff --base`). The GitHub action needs bash (the default on all GitHub runners); the GitLab template needs a POSIX shell executor. They need no outbound internet: the only hosts they talk to are the CI instance itself and the npm registry the job already uses.

`stratum-lint` comes from the project's `node_modules/.bin` when the project pins `@petukhovart/stratum-lint`. Otherwise the template installs the version it was tagged with into `.stratum/` from the job's npm registry config (env, user `.npmrc`, project `.npmrc`).

Both templates treat `stratum-lint` exit 3 (no error, but baseline entries are fixed and `baseline --prune` is due) as a pass. Prune in a separate change.

The label `stratum:accept-debt` on an MR/PR skips `baseline --check`, for example after a file move. A label added later applies to the next pipeline only: on GitLab run a new pipeline, on GitHub `labeled`/`unlabeled` in `on.pull_request.types` reruns the workflow.

## GitLab

Import or fork the stratum repository into your GitLab, then in the consuming project:

```yaml
include:
  - project: <group>/stratum
    ref: v0.2.0
    file: ci/gitlab/templates/stratum.yml
    inputs:
      stage: test   # optional, default test
```

A copy works too: `include: local: ci/stratum.yml`. So does a component in an instance-local CI/CD catalog.

| Job | Runs on | Result |
|---|---|---|
| `stratum` | MR pipelines, default branch | Code Quality report (`gl-code-quality-report.json`) in the MR widget; fails on findings or on baseline growth |
| `stratum-drift` | MR pipelines | `stratum-drift.md` exposed in the MR as "stratum drift"; drift is a warning (exit 1, allowed), a tool failure fails |

- The jobs need node >= 18 and git in the image. Set `image:`, `tags:` and `needs:` on the jobs yourself:
  ```yaml
  stratum:
    tags: [docker]
  stratum-drift:
    tags: [docker]
  ```
- They keep `default: before_script`, so `npm config set registry ...` there still applies.
- MR note: set the CI/CD variable `STRATUM_GITLAB_TOKEN` (masked) to a project access token with the `api` scope, role Reporter or higher. The job then keeps one note per project root up to date. Without the variable, the drift report is only the artifact.
- Self-signed or internal CA: set `NODE_EXTRA_CA_CERTS` to the CA bundle path in the image (for example a file variable), otherwise the note request fails TLS.
- The base is `CI_MERGE_REQUEST_TARGET_BRANCH_SHA` in merged-results pipelines, otherwise the merge base `CI_MERGE_REQUEST_DIFF_BASE_SHA`.
- `CI_MERGE_REQUEST_DIFF_BASE_SHA` exists only in merge request pipelines. Without `workflow: rules` or job rules that create them, only the default-branch run happens.

## GitHub (github.com and GitHub Enterprise Server)

```yaml
on:
  pull_request:
    types: [opened, synchronize, reopened, labeled, unlabeled]
  push:
    branches: [main]

permissions:
  contents: read
  pull-requests: write   # drift comment; without it the report is in the job summary only

jobs:
  stratum:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v5
      - uses: <org>/stratum/ci/github@v0.2.0
        with:
          root: .   # optional, project root inside the repository
```

The gate prints findings as annotations on the PR diff. On pull requests the drift report goes to the job summary and to one PR comment per project root, created when drift appears and updated afterwards. Fork PRs get the summary only and never fail because of the comment. Pass `token: ''` to turn the comment off.

On GHES, use the stratum repository mirrored onto your instance. `actions/checkout` ships with GHES.
