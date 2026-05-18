//! Stable hash for the *effective* config of one rule on one file.
//!
//! Phase 4's Salsa wiring keys per-rule queries by
//! `(rule_id, scope_value, config_hash(rule_options + severity))`.

use serde::Serialize;
use xxhash_rust::xxh3::Xxh3;

use stratum_core::severity::Severity;

/// Compute a stable u64 hash of `(severity, options)` for one rule on one file.
pub fn rule_config_hash<O: Serialize>(severity: Severity, options: &O) -> u64 {
    let mut h = Xxh3::new();
    h.update(&[severity as u8]);
    let bytes = serde_json::to_vec(options).unwrap_or_default();
    h.update(&bytes);
    h.digest()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[derive(Serialize)]
    struct Opts {
        x: u32,
    }

    #[test]
    fn same_input_same_hash() {
        let a = rule_config_hash(Severity::Error, &Opts { x: 1 });
        let b = rule_config_hash(Severity::Error, &Opts { x: 1 });
        assert_eq!(a, b);
    }

    #[test]
    fn different_severity_different_hash() {
        let a = rule_config_hash(Severity::Error, &Opts { x: 1 });
        let b = rule_config_hash(Severity::Warning, &Opts { x: 1 });
        assert_ne!(a, b);
    }

    #[test]
    fn different_options_different_hash() {
        let a = rule_config_hash(Severity::Error, &Opts { x: 1 });
        let b = rule_config_hash(Severity::Error, &Opts { x: 2 });
        assert_ne!(a, b);
    }
}
