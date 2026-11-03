macro_rules! catalog {
    ($($key:ident { en: $en:literal $(,)? })*) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum Key {
            $($key,)*
        }

        impl Key {
            pub const ALL: &[Self] = &[$(Self::$key,)*];

            #[must_use]
            pub const fn template(self) -> &'static str {
                match self {
                    $(Self::$key => $en,)*
                }
            }

            #[must_use]
            pub const fn name(self) -> &'static str {
                match self {
                    $(Self::$key => stringify!($key),)*
                }
            }

            #[must_use]
            pub fn from_name(name: &str) -> Option<Self> {
                Self::ALL.iter().copied().find(|key| key.name() == name)
            }
        }
    };
}

catalog! {
    Verbatim { en: "{text}" }
    ListComma { en: ", " }
    ListAnd { en: " and " }
    ListOr { en: " or " }
    RunExplain {
        en: "Run \"stratum-lint explain {slug}\"",
    }

    CrossLayerImport {
        en: "Module imports from layer '{to}' which '{from}' does not depend on",
    }
    CrossLayerImportFix {
        en: "Move the imported code into a layer '{from}' may import, or add '{to}' to '{from}'.depends_on if the dependency is legitimate. Do not move business logic into shared to get around the rule: shared is common code without domain logic. Introduce an abstraction instead: an interface in the lower layer, the implementation in the upper one.",
    }

    Cycle {
        en: "Import cycle of {count} modules through {name}; about {closing} imports close it",
    }
    CycleFix {
        en: "Break it at one of: {cuts}; extract the shared piece into a lower module or invert the dependency",
    }
    CycleCut {
        en: "`{from}{at} -> {to}` (frees {freed} of {size})",
    }

    PublicApi {
        en: "Import of `{target}` reaches inside `{unit}`",
    }
    PublicApiFix {
        en: "Import from `{unit}` through its entry `{entry}`, exporting the name there if it is missing.",
    }
    PublicApiNoEntry {
        en: "Import of `{target}` reaches inside `{unit}`, which has no entry file",
    }
    PublicApiNoEntryFix {
        en: "Add an entry file to `{unit}` (a name from `structure.entries`, e.g. index.ts) that exports what outside code needs, and import through it.",
    }

    VisibilityScope {
        en: "`{target}` is in a private zone of `{root}`; only modules under `{root}` may import it",
    }
    VisibilityScopeFix {
        en: "Import it through the entry of the container that owns `{root}`, move the importer under `{root}`, or move the code out of the private zone if it is meant to be shared.",
    }

    ImportPolicyDefault {
        en: "Import denied by import-policy",
    }
    ImportPolicy { en: "{message} ({to})" }

    RestrictedCallsDefault {
        en: "Call restricted by restricted-calls",
    }
    RestrictedCall { en: "{message} ({call}())" }

    ReachabilityPolicyDefault {
        en: "Denied module reachable by reachability-policy",
    }
    ReachabilityPolicy {
        en: "{message}: `{from}` reaches `{to}` through {path}{tail}",
    }
    ReachabilityPolicyMore {
        en: " (and {more} more denied modules)",
    }

    CoChangeDefault {
        en: "Files that change together changed alone",
    }
    CoChange {
        en: "{message}: `{first}`{more} changed, but no file matching {require} did",
    }
    CoChangeMore { en: " and {more} more" }
    CoChangeFix {
        en: "Change a file matching {require} in the same change, or acknowledge it: `stratum-ignore stratum/co-change -- <reason>` in a comment of `{first}`",
    }

    SuppressionForbidden {
        en: "Directive `{written}` is forbidden: `stratum/suppression` `forbid` lists `{listed}`.",
    }
    SuppressionForbiddenFix {
        en: "Fix what the directive hides and delete it. To keep existing ones, accept them into the baseline (`stratum-lint baseline`); for a folder, turn the rule off in an `overrides` block.",
    }
    SuppressionInvalid {
        en: "Suppression `{names}` suppresses nothing: {problems}.",
    }
    SuppressionUnused {
        en: "Suppression `{names}` hides nothing on line {line}; delete it.",
    }
    ProblemNoRule { en: "it names no rule" }
    ProblemNotARule { en: "`{slug}` is not a rule" }
    ProblemNoReason { en: "it has no reason after `--`" }

    UnusedExport {
        en: "Export `{name}` is imported by no module reachable from the entry points",
    }

    UnreachableModule {
        en: "No entry point reaches this module through any import",
    }

    UnusedDependency {
        en: "Package `{package}` is declared in `{field}` but nothing imports it",
    }
    UnusedDependencyFix {
        en: "Remove `{package}` from package.json, or add it to the rule's `ignore` if it is used another way (a CLI not in `scripts`, a plugin loaded by name, global types)",
    }
    UnlistedDependency {
        en: "Package `{package}` is imported but not declared in package.json ({fields})",
    }
    UnlistedDependencyFix {
        en: "Add `{package}` to package.json, or add it to the rule's `ignore` if it is an alias or a virtual module",
    }

    BarrelInSegment {
        en: "Entry file '{file}' sits directly in segment folder '{segment}'",
    }
    BarrelInSegmentFix {
        en: "Delete '{file}' and import the files of '{segment}' directly; the unit that owns the segment exposes its public surface through its own entry file.",
    }
    BarrelAtLayerRoot {
        en: "Entry file '{file}' sits at the root of layer '{layer}'",
    }
    BarrelAtLayerRootFix {
        en: "Delete '{file}' and import from the files, or from a sub-unit's entry file, directly.",
    }

    AnEntryFile { en: "an entry file" }
    APrivateZoneFolder { en: "a private-zone folder" }

    ComposerScope {
        en: "Composer '{name}' has no file matching '{pattern}' in its private zone",
    }
    ComposerScopeFix {
        en: "Add a file matching '{pattern}' to {zone} inside '{name}' and move the wiring of its sub-units into it.",
    }

    LayerBreadth {
        en: "Layer '{name}' holds {items} slices and files directly inside it (limit {max})",
    }
    LayerBreadthFix {
        en: "Group related slices under a shared parent slice, or move the ones several layers use into a lower layer.",
    }
    KindsBreadth {
        en: "Container '{name}' (stage {stage}) holds {kinds} kinds directly inside it (limit {max}); {next}",
    }
    KindsBreadthToStage3 {
        en: "evolve it to stage 3 by grouping files into segment folders",
    }
    KindsBreadthToStage4 {
        en: "evolve it to stage 4 by splitting it into sub-units with their own entry file",
    }
    KindsBreadthAtStage4 {
        en: "it is at stage 4 already: group its sub-units into fewer, larger ones",
    }
    KindsBreadthFix {
        en: "Group the files into segment folders (stage 3), or split them into sub-units with their own entry file (stage 4).",
    }
    ComposerBreadth {
        en: "Composer '{name}' holds {files} files at its root besides its entry (limit {max}); a composer only wires its sub-units",
    }
    ComposerBreadthFix {
        en: "Move the logic in the root files of '{name}' into its sub-units or a new sub-unit, and keep only the wiring at the root.",
    }
    FolderBreadth {
        en: "Folder '{path}' of '{name}' holds {items} items directly inside it (limit {max})",
    }
    FolderBreadthFix {
        en: "Split '{path}' into sub-folders by theme, or into sub-units with their own entry file.",
    }
    SingleFile {
        en: "Folder '{name}' holds a single file, '{file}'",
    }
    SingleFileFix {
        en: "Move '{file}' up into the parent folder and delete '{name}', unless more files are about to join it.",
    }

    DeepModule {
        en: "Module exports {count} names{stars} (limit {limit}); split it or hide some exports",
    }
    DeepModuleStars {
        en: ", {stars} of them `export *` re-exports counted as 1 each that may hide more",
    }
    DeepModuleFix {
        en: "Group related exports behind a single facade or split the module along the natural fault lines.",
    }

    DepthRatio {
        en: "Depth ratio {ratio} ({size} modules in {container} / its {exports} exports) is below the configured minimum {min}; module exposes too much relative to its container size",
    }
    DepthRatioItsContainer { en: "its container" }
    DepthRatioFix {
        en: "Consolidate functionality so each public symbol stands for more internal work, or split the container.",
    }

    MillerLimit {
        en: "Imported by {fan_in} modules from other containers whose layers depend on this one (limit {limit}); this module is a change hotspot",
    }
    MillerLimitFix {
        en: "Split the module so each consumer reaches only the surface it needs, or move the implementation closer to its consumers.",
    }

    MissingEntry {
        en: "Folder '{name}' has internal structure ({parts}) but no entry file",
    }
    MissingEntryFix {
        en: "Add {entry} to '{name}' exporting its public surface, or flatten the folder if it is not a unit.",
    }

    NoNavDoc {
        en: "Folder '{name}' has no navigation doc '{nav}'",
    }
    NoNavDocFix {
        en: "Add '{nav}' to '{name}': what it holds, its public surface and links to its parts.",
    }
    StrayNavDoc {
        en: "Navigation doc '{nav}' sits in '{name}', a {kind}, which is no unit",
    }
    StrayNavDocFix {
        en: "Move its content into the navigation doc of the folder that owns '{name}' and delete '{nav}' here.",
    }
    DirLayerRoot { en: "layer root" }
    DirContainer { en: "container" }
    DirSegment { en: "segment folder" }
    DirPrivateZone { en: "private zone" }
    DirCollection { en: "collection folder" }
    NavRequires {
        en: "Navigation doc '{nav}' of '{a}' requires '{b}', which {reason}",
    }
    NavRequiresImports { en: "imports it" }
    NavRequiresSibling { en: "is its sibling" }
    NavRequiresFix {
        en: "Drop '{b}' from the requires of '{a}', or move what '{a}' needs from '{b}' into a folder both may depend on: documents follow imports.",
    }

    NotKebabCase {
        en: "Folder '{name}' is not kebab-case",
    }
    NotKebabCaseFix { en: "Rename '{name}' to '{kebab}'." }
    ReservedName {
        en: "Folder '{name}' has a reserved name",
    }
    ReservedNameFix {
        en: "Name '{name}' after what it holds, not its kind, or move its files next to the code that uses them.",
    }
    DuplicateSlice {
        en: "Slice '{slice}' in layer '{layer}' repeats the name of slice '{twin}' in layer '{twin_layer}'",
    }
    DuplicateSliceFix {
        en: "Merge '{slice}' and '{twin}' into one slice, or rename one so the two names mean different things.",
    }

    PromotionPressure {
        en: "Imported by {pressure} modules from layers that do not depend on '{layer}' (threshold {limit}); consider promoting it to a lower layer",
    }
    PromotionPressureFix {
        en: "Move this module into the lowest layer that every dependent already depends on.",
    }

    Purity {
        en: "Module at purity {from} imports from purity {to}: purer code cannot depend on impurer code",
    }
    PurityFix {
        en: "Move shared logic into a module at or below the lower purity, or inject the impure dependency at composition root.",
    }

    SiblingIsolation {
        en: "`{from}` imports its sibling `{to}`; siblings inside `{parent}` must stay independent",
    }
    SiblingIsolationFix {
        en: "Move the shared code down (a lower layer or a private zone of `{parent}`), or compose `{from}` and `{to}` in `{parent}`.",
    }

    SliceFanIn {
        en: "Slice {slice} is imported by {fan_in} other slices (limit {limit}): {list}",
    }
    SliceFanInFix {
        en: "Narrow the slice's entry or split it so each importer reaches only the part it needs; code every slice needs may belong in a lower layer.",
    }

    StageMixing {
        en: "Container '{name}' mixes segment folders ({segments}) with {others} on one level",
    }
    StageMixingSubUnits { en: "sub-units ({list})" }
    StageMixingOtherFolders { en: "other folders ({list})" }
    StageMixingFoldersFix {
        en: "Move the code of {folders} into the segment folders ({segments}), or turn the segments into sub-units: not both on one level.",
    }
    StageMixingSubUnitsFix {
        en: "Pick one split: move the segment folders' code into sub-units, or fold the sub-units back into segments.",
    }

    ZoneBreadth {
        en: "Private zone '{name}' of '{owner}' holds {kinds} kinds directly inside it (limit {max})",
    }
    ZoneBreadthFix {
        en: "Group the files of '{name}' into folders by purpose, or promote a cohesive group into a sub-unit with its own entry file.",
    }
}
