# stratum-core

Domain types for [Stratum tooling](https://github.com/PetukhovArt/stratum).

This crate is the foundation: every other crate in the workspace depends on it.

## Public API

The entire downstream-visible API of `stratum-core` is:

- The data types — `Module`, `Container`, `Layer`, `Purity`, `Severity`,
  `Violation`, `Edge`, `EdgeKind`, `VisibilityScope`, and the newtype IDs.

## Stratum primitives are first-class

The Stratum methodology (layers, purity ranks, visibility, containment) is built
into the types — not layered on top via configuration. Custom rules
(in v1: Rhai per-module rules) extend behaviour but cannot redefine the
methodology.
