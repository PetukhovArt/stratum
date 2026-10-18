//! Vue SFC extractor for Stratum tooling.
//!
//! Scope: `<script>` and `<script setup>` blocks only.
//! No template parsing, no macro expansion. `defineProps`/`Emits`/`Expose`/`Model`
//! are read only to count the component's exports.

#![forbid(unsafe_code)]

pub mod script_extractor;
mod setup_macros;
pub mod sfc_splitter;
pub mod span_remap;

pub use script_extractor::VueExtractor;
pub use sfc_splitter::{ScriptBlock, SplitError, split};
