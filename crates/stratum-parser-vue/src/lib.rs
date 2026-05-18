//! Vue SFC extractor for Stratum tooling.
//!
//! MVP scope (PRD decision D8): `<script>` and `<script setup>` blocks only.
//! No template parsing, no macro expansion, no `defineProps`/`Emits`/`Expose`
//! interpretation.

#![forbid(unsafe_code)]

pub mod script_extractor;
pub mod sfc_splitter;
pub mod span_remap;

pub use script_extractor::VueExtractor;
pub use sfc_splitter::{ScriptBlock, SplitError, split};
pub use span_remap::shift;
