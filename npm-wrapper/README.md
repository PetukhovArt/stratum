# `@stratum/lint`

npm wrapper for the [Stratum](https://github.com/PetukhovArt/stratum) architectural linter. On install, `install.js` downloads the platform-matching prebuilt binary from the corresponding GitHub release and places it under `binary/`. The `stratum-lint` shim in `bin/` invokes that binary with the user's arguments.

## Install

```
npm install -g @stratum/lint
```

Supported platforms:

| Platform | Architectures |
|----------|---------------|
| Linux | x64, arm64 |
| macOS | x64, arm64 |
| Windows | x64 |

## Run

```
stratum-lint <project-root>
stratum-lint snapshot <root> --out graph.json
stratum-lint visualize <root>
```

See `stratum-lint --help` for the full set of subcommands.

## How the version is chosen

The downloaded binary version is locked to this package's `version` field. Bump the package version to match the GitHub release tag (`v0.1.0` ↔ `0.1.0`); the install script appends `v$version` to the release URL.
