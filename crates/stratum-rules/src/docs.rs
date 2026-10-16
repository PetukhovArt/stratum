#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct RuleDoc {
    pub summary: String,
    pub why: String,
    pub fix: String,
    /// One `name: meaning (default)` line per option; empty when the rule takes none.
    pub options: Vec<String>,
}

#[must_use]
pub fn builtin(slug: &str) -> Option<RuleDoc> {
    let (summary, why, fix, options): (&str, &str, &str, &[&str]) = match slug {
        "stratum/no-cross-layer-import" => (
            "A module may import only from its own layer and from the layers listed in its layer's `depends_on`.",
            "Layers fix the direction of dependencies; one import outside `depends_on` couples a layer to code it must not know, and the layering stops holding.",
            "Move the imported code into a layer the importer may reach (usually a lower one such as shared), pass it in from above instead of importing it, or add the target layer to `depends_on` if the dependency is intended.",
            &[],
        ),
        "stratum/no-circular-deps" => (
            "Reports every import cycle: two or more modules that reach each other through imports, or a module importing itself. One finding per cycle, at its anchor (the member with the smallest project-relative path), on the line of the anchor's first import into the cycle. The message gives the number of modules and about how many imports close the cycle; the fix names up to three imports to break, best first, each with how many modules breaking it frees from the cycle. The finding's modules start with the endpoints of those imports.",
            "Modules in a cycle cannot be loaded, tested or changed in isolation, and cycles break initialisation order at runtime.",
            "Break the cycle at one of the imports the fix names: extract the piece both sides need into a lower module, or invert the dependency (callback, interface, event). Per-file overrides do not apply; only the project-wide severity does.",
            &[],
        ),
        "stratum/purity" => (
            "Every module has a purity rank from 1 (pure) to 4 (side effects), default 2, set by a `// @stratum-purity N` line in the file's leading comments. A module may import only modules whose purity is <= its own.",
            "Purer code that imports impurer code inherits its side effects and stops being testable and reusable on its own.",
            "Move the shared logic into a module at or below the importer's purity, raise the importer's `@stratum-purity` if it really is impure, or inject the impure dependency from the composition root.",
            &[],
        ),
        "stratum/miller-limit" => (
            "Reports a module with fan-in above `max_children`: distinct modules outside its outer container (its slice in a `slices: true` layer, else its layer) whose layer may import it, i.e. is its own layer or lists it in `depends_on`. Importers whose layer does not declare it count toward stratum/promotion-pressure instead, so the two rules never count the same importer. Without any `depends_on` (zero-config) every importer is undeclared and this rule reports nothing.",
            "A module that many legitimate consumers reach is a change hotspot: every edit ripples into all of them.",
            "Split it so each consumer imports only the part it needs, or hide it behind a smaller facade.",
            &["max_children: max distinct declared importers from other containers (20)"],
        ),
        "stratum/depth-ratio" => (
            "Reports a module whose ratio (modules in its outer container, nested containers included: its slice in a `slices: true` layer, else its layer) / (its exports) is below `min_ratio`. Exports are counted as in stratum/deep-module; modules that export nothing are skipped.",
            "A shallow module exposes many names relative to the code around it, so its callers must know more than the container hides.",
            "Hide exports that only the container uses, merge related ones behind one entry point, or split the module.",
            &["min_ratio: minimum container_size / exports ratio (1.0)"],
        ),
        "stratum/visibility-scope" => (
            "A module inside a private-zone folder (named in `structure.private`; `stratum:recommended` sets `_shared`) may be imported only by modules under that folder's parent. When zones nest, the innermost one decides. Every other module is public.",
            "A private zone holds what a folder's own children share; folder-structure makes it visible to direct siblings only (iso:148), so outside code cannot grow a dependency on those internals.",
            "Import through the entry of the container that owns the zone, move the importer under the zone's parent, or move the code out of the zone if it is meant to be shared.",
            &[],
        ),
        "stratum/slice-fan-in" => (
            "Reports a slice imported by more than `max_fan_in` distinct other slices, from any layer; several modules of one importing slice count once. Slices are the direct children of a layer declared `slices: true`. The message lists the importers as `<layer>/<slice>`.",
            "A slice that many slices reach turns into a god object; any change to it ripples through all of them.",
            "Narrow the slice's entry or split it so each importer reaches only the part it needs; code every slice needs may belong in a lower layer.",
            &["max_fan_in: max distinct importing slices per slice (5)"],
        ),
        "stratum/public-api" => (
            "An import from outside a container (a slice, sub-unit or nested container; layers are exempt) must target that container's entry file (`structure.entries`, e.g. `index.ts`). The outermost container that holds the target but not the importer is checked. When that container has no entry file, the message says so.",
            "folder-structure lets the outside reach a unit only through its `index.ts` (bas:228); a deep import binds the importer to internals, so every internal move becomes a breaking change.",
            "Import from the container's entry, export the needed name from the entry, or add an entry file if the container has none.",
            &[],
        ),
        "stratum/sibling-isolation" => (
            "Reports an import between two different slices or sub-units that share a parent container, at any depth. Nested containers, and the entry-bearing children of a layer without `slices`, are not isolated.",
            "Siblings that import each other stop being independent units: neither can change, move or be deleted alone, and their parent no longer is the one place that composes them.",
            "Move the shared code down (to a lower layer or the parent's private zone), or compose the two siblings in their parent. To allow chosen cross-imports, set this rule to off in an `overrides` block matching those importers.",
            &[],
        ),
        "stratum/restricted-calls" => (
            "Reports a call to a listed function name outside the files matched by `allow`, at the call's line. A plain identifier callee matches, and so does a local alias of an imported listed name (`import { inject as i }`, then `i()`). Member calls such as `x.inject()` do not.",
            "Some functions (DI `inject`/`provide`, global state setters) must be called from one designated place so that the wiring stays in one spot.",
            "Call the wrapper the policy's message names, or move the call into a file matched by `allow`.",
            &[
                "options: a list of policies; a call is reported by the first policy that lists its name and does not allow its file, e.g. [{ \"calls\": [\"inject\", \"provide\"], \"allow\": [\"**/_shared/*-scope.ts\"], \"message\": \"Use useXxxScope()\" }]",
                "calls: function names; a local alias of an imported name matches too (required)",
                "allow: globs of files where the calls are allowed, forward slashes relative to the project root ([])",
                "message: text of the violation; the called name is appended (\"Call restricted by restricted-calls\")",
            ],
        ),
        "stratum/stage-mixing" => (
            "Reports a container (layers are exempt) with both segment folders (`structure.segments`) and, directly inside it, sub-unit folders or other plain folders (no entry file, not a segment: a domain folder next to the segments).",
            "Segments (stage 3) and sub-units (stage 4) are two different ways to split a container; mixing them leaves code that belongs to no sub-unit next to code that does. A domain folder beside the segments splits the same code a second way.",
            "Pick one: move the segment folders' code into sub-units, or fold the sub-units and domain folders back into segments.",
            &[],
        ),
        "stratum/container-breadth" => (
            "Reports a folder that holds too much directly inside it, at any depth, by three limits. Heterogeneous: a container (layers are exempt) with more than `max_kinds` kinds; a kind is a child folder, or the child files sharing a stem before the first dot (`foo.ts`, `foo.test.ts` and `foo.vue` are one kind), and the entry file and private zones do not count. Homogeneous: a segment or collection folder, or a sliced layer root counting its slices, with more than `max_items` items. Composer: a stage-4 container (it has sub-units) with more than `max_root_files` files directly at its root (counted by stem, its entry file excluded), since a composer wires its sub-units together and holds no logic of its own. A folder with a single file is reported at info at most. The message names the stage to evolve to. Private zones are checked by stratum/zone-breadth.",
            "Past about seven kinds a reader can no longer hold a folder in mind (Miller's limit), so the container is due to move to its next stage; a long flat list of like things hides which of them matter, and logic in a composer's root hides the composition.",
            "Group the files into segment folders (stage 3), or split them into sub-units with their own entry file (stage 4); split a long homogeneous folder by theme; move a composer's logic into a sub-unit; inline a folder that holds one file.",
            &[
                "max_kinds: max kinds directly in one container (6)",
                "max_items: max items in one segment, collection or sliced layer root (20)",
                "max_root_files: max files directly at the root of a stage-4 container (4)",
            ],
        ),
        "stratum/deep-module" => (
            "Reports a module that exports more than `max_exports` names. Counted: distinct top-level export names (declarations, `export { }`, `export { } from`, `export * as ns from`, types included, overloads and merged declarations once), `export default` and `export =` as 1, and every `export * from` as 1 (the message flags these: the real surface may be larger). CommonJS `module.exports` is not counted. Vue SFC: the exports of a plain `<script>` (its default export counts 1) plus, in `<script setup>`, each `defineProps` / `defineEmits` member, each `defineExpose` property and each `defineModel` call; a props or emits type that is not a type literal or a local interface / object type alias counts 1.",
            "A module with a wide interface is shallow: callers must learn many names instead of one small interface hiding the work.",
            "Hide names only the module's own container needs, group related exports behind one facade, or split the module along its responsibilities.",
            &["max_exports: max exported names (20)"],
        ),
        "stratum/promotion-pressure" => (
            "Reports a module imported by at least `threshold` distinct modules outside its outer container (its slice in a `slices: true` layer, else its layer) whose layer does not list the module's layer in `depends_on`. Declared importers count toward stratum/miller-limit instead.",
            "Code reached from layers that do not declare it sits too high; moving it lower makes those imports legal instead of leaving each one a cross-layer violation.",
            "Move it into the lowest layer that all its dependents already depend on (often shared), or add the dependency to `depends_on` if it is intended.",
            &["threshold: min distinct undeclared importers from other containers (5)"],
        ),
        "stratum/zone-breadth" => (
            "Reports a private-zone folder (`structure.private`) with more than `max_kinds` kinds directly inside it, at any depth. Kinds are counted as in stratum/container-breadth.",
            "A private zone holds what its siblings share; once it grows past what a reader can hold in mind (Miller's limit) it is a second, unstructured container.",
            "Group the zone's files into folders by purpose, or promote a cohesive group into a sub-unit with its own entry file.",
            &["max_kinds: max kinds directly in one private zone (6)"],
        ),
        "stratum/missing-entry" => (
            "Reports a folder with internal structure (a segment folder or a private zone directly inside it) but no entry file (`structure.entries`).",
            "A folder with parts is a unit, and a unit needs one file to be reached through; without it importers reach into its segments and its public surface is undefined.",
            "Add an entry file that exports the unit's public surface, or flatten the folder if it is not a unit.",
            &[],
        ),
        "stratum/no-barrel" => (
            "Reports an entry file (`structure.entries`) used as a barrel where it hurts: directly inside a segment folder, and, in the layers named in `layers`, at the layer root.",
            "A segment is a group of files, not a unit. A barrel over it, or over a layer every other layer imports, makes each importer load all of it, hides which file provides a name and invites import cycles.",
            "Delete the barrel and import the files directly; in a global layer, import from the file or from a sub-unit's entry.",
            &[
                "layers: ids of layers whose root may not hold a barrel; stratum:fsd sets [\"shared\"] ([])",
            ],
        ),
        "stratum/composer-scope" => (
            "Reports a stage-4 container (it has sub-units) whose private zone holds no file matching `scope_file`. It reports nothing while `scope_file` is unset.",
            "A composer wires its sub-units together; the scope file in its private zone is the one place that wiring lives, so sub-units never reach for each other.",
            "Add a file matching `scope_file` to the composer's private zone and move the wiring into it.",
            &[
                "scope_file: glob of the file name each composer must have in its private zone; stratum:fsd sets \"*-scope.*\" (unset)",
            ],
        ),
        "stratum/naming" => (
            "Reports folder names that break the naming convention: a name not in the `folder_case`, a name listed in `reserved`, and with `duplicate_slices` a slice whose name another slice of any layer already has, or differs from one only by a plural `s`. Each check is off until its option is set.",
            "Names are how a reader finds code; catch-all names say nothing about content, and two slices with near-identical names split one concept in two.",
            "Rename the folder to the convention, name it after what it holds instead of its kind, or merge the duplicate slices.",
            &[
                "folder_case: required case of folder names, `kebab`; stratum:fsd sets kebab (unset)",
                "reserved: folder names not allowed; stratum:fsd sets hooks, helpers, types, utils, components ([])",
                "duplicate_slices: report duplicate and plural slice names; stratum:fsd sets true (false)",
            ],
        ),
        "stratum/module-docs" => (
            "Checks navigation docs against the folder tree. It reports nothing while `nav_file` is unset. Reported: a collection or container folder without a `nav_file`, a `nav_file` inside a segment folder, and a `nav_file` whose `requires_key` frontmatter names modules that disagree with the direction of the folder's real imports.",
            "Navigation docs let an agent or a newcomer find code without reading it; one that is missing, misplaced or contradicts the imports misleads.",
            "Add the nav file to the folder, move it out of the segment into the folder that owns it, or correct its `requires` list to the real imports.",
            &[
                "nav_file: file name of a folder's navigation doc; the rule is inert without it (unset)",
                "requires_key: frontmatter key listing the modules the folder requires (\"requires\")",
            ],
        ),
        "stratum/unused-exports" => (
            "Reports an export that no module reachable from `project.entry_points` imports. Names are followed through `export { x } from` and `export * from`; an entry point's own exports count as used, and so does a `.vue` default export that is imported. Needs `project.entry_points`: enabling the rule without them is a config error. Blind spots: importers in files outside the layers or ignored by `project.ignore` (such as tests) are invisible, and so are non-literal dynamic imports.",
            "An export nobody takes widens a module's interface for no caller and keeps code alive that nothing runs.",
            "Delete the export, and the code when nothing else uses it, or make it module-private. If it is reached in a way Stratum cannot see, add the importing file to `project.entry_points`.",
            &[],
        ),
        "stratum/unreachable-modules" => (
            "Reports a module that no file in `project.entry_points` reaches through imports of any kind (static, dynamic, type-only, side-effect). Needs `project.entry_points`: enabling the rule without them is a config error. Blind spots: importers in files outside the layers or ignored by `project.ignore` (such as tests) are invisible, and so are non-literal dynamic imports.",
            "A module nothing reaches is dead weight: it is maintained, linted and read for no effect.",
            "Delete the module, or import it from where it is used. If it is started some other way (a script, a worker), add it to `project.entry_points`.",
            &[],
        ),
        "stratum/suppression" => (
            "Checks `// stratum-ignore <slug>[, <slug>] -- <reason>` directives (also `/* */`). A directive covers the next line only; there is no file scope, and findings on a folder or the whole project cannot be suppressed. Reported as an error, with the directive suppressing nothing: a missing reason or an unknown slug. Reported as a warning: a valid directive that suppresses nothing.",
            "A suppression without a reason, or one that no longer hides anything, is noise that rots into a permanent exception and hides real problems.",
            "Write the reason after `--`, correct the slug, or delete the directive once the line no longer breaks the rule.",
            &[],
        ),
        _ => return None,
    };
    Some(RuleDoc {
        summary: summary.into(),
        why: why.into(),
        fix: fix.into(),
        options: options.iter().map(|o| (*o).to_string()).collect(),
    })
}

#[cfg(test)]
mod tests {
    use crate::registry::RuleRegistry;

    #[test]
    fn every_builtin_rule_is_documented() {
        let registry = RuleRegistry::with_builtins();
        let undocumented: Vec<&str> = registry
            .rules()
            .iter()
            .filter(|r| r.doc().is_none())
            .map(|r| r.slug())
            .collect();
        assert!(undocumented.is_empty(), "{undocumented:?}");
    }
}
