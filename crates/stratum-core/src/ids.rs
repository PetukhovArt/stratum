use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

macro_rules! id_newtype {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(
            Debug,
            Clone,
            Copy,
            PartialEq,
            Eq,
            Hash,
            PartialOrd,
            Ord,
            Serialize,
            Deserialize,
            JsonSchema,
        )]
        #[repr(transparent)]
        #[serde(transparent)]
        pub struct $name(pub u32);

        impl $name {
            pub const fn new(raw: u32) -> Self {
                Self(raw)
            }

            #[must_use]
            pub const fn raw(self) -> u32 {
                self.0
            }
        }
    };
}

id_newtype!(
    ModuleId,
    "Stable identifier for a Module inside a Compound DAG."
);
id_newtype!(ContainerId, "Stable identifier for a Container.");
id_newtype!(LayerId, "Stable identifier for a Layer.");
id_newtype!(
    ProjectId,
    "Stable identifier for a Project (workspace root)."
);
id_newtype!(RuleId, "Stable identifier for a Rule.");

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_copy_and_hashable() {
        let a = ModuleId::new(7);
        let b = a;
        assert_eq!(a, b);
        assert_eq!(a.raw(), 7);

        let mut set = std::collections::HashSet::new();
        set.insert(a);
        assert!(set.contains(&b));
    }

    #[test]
    fn ids_roundtrip_through_serde() {
        let a = ContainerId::new(42);
        let s = serde_json::to_string(&a).unwrap();
        assert_eq!(s, "42");
        let b: ContainerId = serde_json::from_str(&s).unwrap();
        assert_eq!(a, b);
    }
}
