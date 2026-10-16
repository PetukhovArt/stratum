# `@petukhovart/stratum-lint`

npm package for the [Stratum](https://github.com/PetukhovArt/stratum) architecture linter: the `stratum-lint` CLI and the `stratum-lsp` stdio language server.

```
npm install -g @petukhovart/stratum-lint
```

The binaries come in one optional dependency per platform (`@petukhovart/stratum-lint-<platform>-<arch>`, listed under `optionalDependencies`), so install runs no scripts, works with `--ignore-scripts`, and needs only the npm registry or its proxy. When npm skipped the platform package (`--omit=optional`, a lockfile written on another platform), `stratum-lint` and `stratum-lsp` exit with code 2 and name the missing package.
