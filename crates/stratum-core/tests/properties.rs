#![allow(clippy::unwrap_used)]

use proptest::prelude::*;
use stratum_core::{ModuleId, Purity, Severity};

fn arb_stage() -> impl Strategy<Value = Purity> {
    (1u8..=4).prop_map(|r| Purity::new(r).unwrap())
}

fn arb_severity() -> impl Strategy<Value = Severity> {
    prop_oneof![
        Just(Severity::Off),
        Just(Severity::Info),
        Just(Severity::Warning),
        Just(Severity::Error),
    ]
}

proptest! {
    #[test]
    fn stage_dep_reflexive(s in arb_stage()) {
        prop_assert!(s.may_depend_on(s));
    }

    #[test]
    fn stage_dep_monotone(a in arb_stage(), b in arb_stage()) {
        let dep = a.may_depend_on(b);
        if a.rank() >= b.rank() {
            prop_assert!(dep);
        } else {
            prop_assert!(!dep);
        }
    }

    #[test]
    fn only_error_fails(s in arb_severity()) {
        prop_assert_eq!(s.fails_build(), matches!(s, Severity::Error));
    }

    #[test]
    fn module_id_roundtrip(raw in any::<u32>()) {
        let id = ModuleId::new(raw);
        let s = serde_json::to_string(&id).unwrap();
        let back: ModuleId = serde_json::from_str(&s).unwrap();
        prop_assert_eq!(id, back);
    }
}
