#![allow(clippy::unwrap_used)]

use camino::Utf8Path;
use proptest::prelude::*;
use stratum_parser_ts::{LanguageExtractor, OxcTsExtractor};

fn arb_specifier() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("./a".to_string()),
        Just("./b".to_string()),
        Just("@/x".to_string()),
        Just("react".to_string()),
        Just("../up".to_string()),
    ]
}

proptest! {
    /// Extracting the same source twice yields identical results.
    #[test]
    fn extraction_is_deterministic(specs in proptest::collection::vec(arb_specifier(), 0..10)) {
        let src: String = specs
            .iter()
            .enumerate()
            .map(|(i, s)| format!(r#"import x{i} from "{s}";"#))
            .collect::<Vec<_>>()
            .join("\n");
        let e = OxcTsExtractor::new();
        let a = e.extract(Utf8Path::new("x.ts"), &src).unwrap();
        let b = e.extract(Utf8Path::new("x.ts"), &src).unwrap();
        prop_assert_eq!(a, b);
    }

    /// Order of imports in output matches order in source.
    #[test]
    fn imports_preserve_source_order(specs in proptest::collection::vec(arb_specifier(), 1..10)) {
        let src: String = specs
            .iter()
            .enumerate()
            .map(|(i, s)| format!(r#"import x{i} from "{s}";"#))
            .collect::<Vec<_>>()
            .join("\n");
        let e = OxcTsExtractor::new();
        let a = e.extract(Utf8Path::new("x.ts"), &src).unwrap();
        prop_assert_eq!(a.imports.len(), specs.len());
        for (i, s) in specs.iter().enumerate() {
            prop_assert_eq!(&a.imports[i].specifier, s);
        }
    }
}
