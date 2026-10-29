use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::{
    edge::EdgeKind,
    ids::{ContainerId, LayerId, ModuleId},
    purity::Purity,
    stage::Stage,
    violation::SourceLocation,
    visibility::VisibilityScope,
};

/// One source unit (file or SFC) appearing as a node in the Compound DAG.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Module {
    pub id: ModuleId,
    pub path: PathBuf,
    pub container: ContainerId,
    pub layer: LayerId,
    pub purity: Purity,
    pub visibility: VisibilityScope,
    #[serde(default)]
    pub exports: Exports,
    #[serde(default)]
    pub calls: Vec<Call>,
    /// Source order, package and unresolved imports included.
    #[serde(default)]
    pub imports: Vec<Import>,
    #[serde(default)]
    pub suppressions: Vec<Suppression>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Call {
    /// The tracked-call entry it matched: `inject`, `obj.get`, `*.get`, or a factory name.
    pub name: String,
    pub location: SourceLocation,
    /// In call order; for [`CallForm::Client`] the arguments of the factory call.
    #[serde(default)]
    pub args: Vec<CallArg>,
    /// The name of the top-level declaration holding the call: `default` for an anonymous
    /// `export default`, `None` for a top-level statement that declares nothing.
    #[serde(default)]
    pub owner: Option<String>,
    #[serde(default)]
    pub form: CallForm,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CallArg {
    /// A string literal, or a template literal without substitutions.
    Str(String),
    Ident(String),
    Other,
}

/// How a call reached its tracked entry.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CallForm {
    /// The entry itself was called.
    #[default]
    Direct,
    /// `c.method()` where `c` is a variable of the same module set to a factory call.
    Client { method: String },
    /// A factory call whose result no member call of the same module uses through a variable.
    Unfollowed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Import {
    pub specifier: String,
    /// Start of the import statement, or of the `import()` / `require()` / `import.meta.glob()` call.
    pub location: SourceLocation,
    pub kind: EdgeKind,
    pub type_only: bool,
    pub bindings: Bindings,
    /// `None` for packages and anything else outside the graph.
    pub target: Option<ModuleId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Bindings {
    /// By the name the target exports; `default` for a default import.
    Names(Vec<ImportedName>),
    /// Namespace import, `export * as ns from`, `require()`, `import()`, `import.meta.glob()`.
    All,
    None,
    Forward,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportedName {
    pub imported: String,
    pub type_only: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Exports {
    /// One per distinct exported name.
    pub decls: Vec<ExportDecl>,
    /// Indices into [`Module::imports`] of every `export * from`.
    pub stars: Vec<usize>,
    /// Names counted toward the surface that no import can take (Vue `defineProps` members).
    pub unimportable: u32,
}

impl Exports {
    /// Distinct names, plus 1 for every `export * from`, plus the unimportable names.
    #[must_use]
    pub fn count(&self) -> u32 {
        u32::try_from(self.decls.len() + self.stars.len())
            .unwrap_or(u32::MAX)
            .saturating_add(self.unimportable)
    }

    #[must_use]
    pub fn star_reexports(&self) -> u32 {
        u32::try_from(self.stars.len()).unwrap_or(u32::MAX)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportDecl {
    pub name: String,
    pub type_only: bool,
    pub location: SourceLocation,
    pub origin: ExportOrigin,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportOrigin {
    Local,
    /// `import` indexes [`Module::imports`]; `imported` is `*` for `export * as name from`.
    Reexport {
        import: usize,
        imported: String,
    },
}

/// A `stratum-ignore <slug>[, <slug>] -- <reason>` comment as written, not validated.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Suppression {
    pub location: SourceLocation,
    /// The line after the comment ends.
    pub applies_to: u32,
    pub slugs: Vec<String>,
    /// `None` when there is no `--` or nothing after it.
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContainerKind {
    Layer,
    Slice,
    SubUnit,
    Nested,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Container {
    pub id: ContainerId,
    pub name: String,
    pub path: PathBuf,
    pub kind: ContainerKind,
    pub layer: LayerId,
    pub parent: Option<ContainerId>,
    pub stage: Stage,
    pub entry: Option<ModuleId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub segment_dirs: Vec<String>,
}

/// A named group of Modules with its own visibility and dependency direction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Layer {
    pub id: LayerId,
    pub name: String,
    pub path: PathBuf,
    pub depends_on: Vec<LayerId>,
    pub slices: bool,
}
