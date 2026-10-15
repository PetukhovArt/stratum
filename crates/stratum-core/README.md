# stratum-core

Domain types for [Stratum tooling](https://github.com/PetukhovArt/stratum).

This crate is the foundation: every other crate in the workspace depends on it.

## Public API

The re-exports in [src/lib.rs](src/lib.rs).

## Stratum primitives are first-class

Layers, purity ranks, visibility scopes and containers are dedicated types, not configuration over generic primitives. What configuration decides is where they come from: `layers` and `structure` in `stratum.config.jsonc` say which folders are layers, slices, entries, segments and private zones. Custom rules (Rhai scripts) add checks but cannot redefine these primitives.
