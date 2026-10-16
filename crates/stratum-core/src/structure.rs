use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Names that shape containers. Each list matches whole path components, never substrings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct Structure {
    /// Exact file names. A folder holding one is a container, and that file is its only import target from outside.
    pub entries: Vec<String>,
    /// Folder names that group a container's files by purpose. A segment folder is never a container.
    pub segments: Vec<String>,
    /// Private-zone folder names: visible only inside their parent folder's subtree.
    pub private: Vec<String>,
}

impl Default for Structure {
    fn default() -> Self {
        Self {
            entries: [
                "index.ts",
                "index.tsx",
                "index.js",
                "index.jsx",
                "index.vue",
            ]
            .map(String::from)
            .to_vec(),
            segments: vec![],
            private: vec![],
        }
    }
}

impl Structure {
    #[must_use]
    pub fn entry_rank(&self, file_name: &str) -> Option<usize> {
        self.entries.iter().position(|e| e == file_name)
    }

    #[must_use]
    pub fn is_segment(&self, folder: &str) -> bool {
        self.segments.iter().any(|s| s == folder)
    }

    #[must_use]
    pub fn is_private(&self, folder: &str) -> bool {
        self.private.iter().any(|p| p == folder)
    }
}
