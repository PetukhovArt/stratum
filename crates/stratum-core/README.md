# stratum-core

Domain types and incremental database for [Stratum tooling](https://github.com/PetukhovArt/stratum).

This crate is the foundation: every other crate in the workspace depends on it.

## Public API

The entire downstream-visible API of `stratum-core` is:

- The data types — `Module`, `Container`, `Layer`, `Stage`, `Severity`,
  `Violation`, `Edge`, `EdgeKind`, `VisibilityScope`, and the newtype IDs.
- The Salsa trait `ArchitectureDatabase` with three queries:
  - `compound_graph(project) -> CompoundGraphSnapshot`
  - `violations(project) -> Vec<Violation>`
  - `violations_for_file(project, file) -> Vec<Violation>`
- The concrete database struct `StratumDb`.

Intermediate Salsa queries are `pub(crate)`. Do not add new public queries
without a PRD-level decision (see decisions doc D6).

## Stratum primitives are first-class

The Stratum methodology (layers, stages, visibility, containment) is built
into the types — not layered on top via configuration. Custom rules
(in v1: Rhai per-module rules) extend behaviour but cannot redefine the
methodology.
