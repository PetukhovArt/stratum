#![allow(clippy::unwrap_used)]

use proptest::prelude::*;
use stratum_core::Purity;

fn arb_purity() -> impl Strategy<Value = Purity> {
    (1u8..=4).prop_map(|r| Purity::new(r).unwrap())
}

proptest! {
    #[test]
    fn purity_dep_monotone(a in arb_purity(), b in arb_purity()) {
        let dep = a.may_depend_on(b);
        if a.rank() >= b.rank() {
            prop_assert!(dep);
        } else {
            prop_assert!(!dep);
        }
    }
}
