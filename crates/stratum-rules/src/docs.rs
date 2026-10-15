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
            "Reports every import cycle: two or more modules that reach each other through imports, or a module importing itself. One violation per cycle, listing its members.",
            "Modules in a cycle cannot be loaded, tested or changed in isolation, and cycles break initialisation order at runtime.",
            "Break one edge: extract the piece both sides need into a lower module, or invert one dependency (callback, interface, event). Per-file overrides do not apply; only the project-wide severity does.",
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
            &["max_children: max distinct declared importers from other containers (7)"],
        ),
        "stratum/depth-ratio" => (
            "Reports a module whose ratio (modules in its outer container, nested containers included: its slice in a `slices: true` layer, else its layer) / (its exports) is below `min_ratio`. Exports are counted as in stratum/deep-module; modules that export nothing are skipped.",
            "A shallow module exposes many names relative to the code around it, so its callers must know more than the container hides.",
            "Hide exports that only the container uses, merge related ones behind one entry point, or split the module.",
            &["min_ratio: minimum container_size / exports ratio (3.0)"],
        ),
        "stratum/visibility-scope" => (
            "A module inside a private-zone folder (named in `structure.private`, default `_shared`) may be imported only by modules under that folder's parent. When zones nest, the innermost one decides. Every other module is public.",
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
            "Reports a container with both segment folders (`structure.segments`) and sub-unit folders directly inside it.",
            "Segments (stage 3) and sub-units (stage 4) are two different ways to split a container; mixing them leaves code that belongs to no sub-unit next to code that does.",
            "Pick one: move the segment folders' code into sub-units, or fold the sub-units back into segments.",
            &["segments: segment folder names (`structure.segments`)"],
        ),
        "stratum/container-breadth" => (
            "Reports a container (layers are exempt) or private-zone folder with more than `max_kinds` kinds directly inside it. A kind is a child folder, or the child files sharing a stem before the first dot (`foo.ts`, `foo.test.ts` and `foo.vue` are one kind). The entry file and private zones do not count. The message names the stage to evolve to. Private-zone findings are reported at info at most, whatever severity the rule is set to.",
            "Past about seven kinds a reader can no longer hold a folder in mind (Miller's limit), so the container is due to move to its next stage.",
            "Group the files into segment folders (stage 3), or split them into sub-units with their own entry file (stage 4).",
            &["max_kinds: max kinds directly in one container (6)"],
        ),
        "stratum/deep-module" => (
            "Reports a module that exports more than `max_exports` names. Counted: distinct top-level export names (declarations, `export { }`, `export { } from`, `export * as ns from`, types included, overloads and merged declarations once), `export default` and `export =` as 1, and every `export * from` as 1 (the message flags these: the real surface may be larger). CommonJS `module.exports` is not counted. Vue SFC: the exports of a plain `<script>` (its default export counts 1) plus, in `<script setup>`, each `defineProps` / `defineEmits` member, each `defineExpose` property and each `defineModel` call; a props or emits type that is not a type literal or a local interface / object type alias counts 1.",
            "A module with a wide interface is shallow: callers must learn many names instead of one small interface hiding the work.",
            "Hide names only the module's own container needs, group related exports behind one facade, or split the module along its responsibilities.",
            &["max_exports: max exported names (12)"],
        ),
        "stratum/promotion-pressure" => (
            "Reports a module imported by at least `threshold` distinct modules outside its outer container (its slice in a `slices: true` layer, else its layer) whose layer does not list the module's layer in `depends_on`. Declared importers count toward stratum/miller-limit instead.",
            "Code reached from layers that do not declare it sits too high; moving it lower makes those imports legal instead of leaving each one a cross-layer violation.",
            "Move it into the lowest layer that all its dependents already depend on (often shared), or add the dependency to `depends_on` if it is intended.",
            &["threshold: min distinct undeclared importers from other containers (5)"],
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
