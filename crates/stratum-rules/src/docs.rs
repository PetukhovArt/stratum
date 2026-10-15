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
        "stratum/stage-purity" => (
            "Every module has a purity stage from 1 (pure) to 4 (side effects), default 2, set by a `// @stratum-stage N` line in the file's leading comments. A module may import only modules whose stage is <= its own.",
            "Purer code that imports impurer code inherits its side effects and stops being testable and reusable on its own.",
            "Move the shared logic into a module at or below the importer's stage, raise the importer's `@stratum-stage` if it really is impure, or inject the impure dependency from the composition root.",
            &[],
        ),
        "stratum/miller-limit" => (
            "Reports a module with fan-in above `max_children`: distinct modules from other containers (today one container per layer) whose layer may import it, i.e. is its own layer or lists it in `depends_on`. Importers whose layer does not declare it count toward stratum/promotion-pressure instead, so the two rules never count the same importer. Without any `depends_on` (zero-config) every importer is undeclared and this rule reports nothing.",
            "A module that many legitimate consumers reach is a change hotspot: every edit ripples into all of them.",
            "Split it so each consumer imports only the part it needs, or hide it behind a smaller facade.",
            &["max_children: max distinct declared importers from other containers (7)"],
        ),
        "stratum/depth-ratio" => (
            "Reports a module whose ratio (modules in its container) / (its exports) is below `min_ratio`. Exports are counted as in stratum/deep-module; modules that export nothing are skipped.",
            "A shallow module exposes many names relative to the code around it, so its callers must know more than the container hides.",
            "Hide exports that only the container uses, merge related ones behind one entry point, or split the module.",
            &["min_ratio: minimum container_size / exports ratio (3.0)"],
        ),
        "stratum/visibility-scope" => (
            "A module may import another only if the target's visibility admits it: public = anyone, layer = same layer, container = same container, shared = importers under a `/_shared/` path. Every module is public today, so this rule reports nothing yet.",
            "Visibility keeps a module's internals private to the scope that owns them.",
            "Import the target through a public entry point, move the importer into the allowed scope, or widen the target's visibility.",
            &[],
        ),
        "stratum/cross-entity-pattern" => (
            "Reports an entity (`/entities/<name>/`) imported by more than `max_fanout` distinct features (`/features/<name>/`). Reported on the entity's index.ts, else its first module.",
            "An entity that every feature reaches turns into a god object; any change to it ripples through all features.",
            "Narrow the entity's public API or split it so each feature imports only the slice it needs.",
            &["max_fanout: max distinct importing features per entity (5)"],
        ),
        "stratum/deep-module" => (
            "Reports a module that exports more than `max_exports` names. Counted: distinct top-level export names (declarations, `export { }`, `export { } from`, `export * as ns from`, types included, overloads and merged declarations once), `export default` and `export =` as 1, and every `export * from` as 1 (the message flags these: the real surface may be larger). CommonJS `module.exports` is not counted. Vue SFC: the exports of a plain `<script>` (its default export counts 1) plus, in `<script setup>`, each `defineProps` / `defineEmits` member, each `defineExpose` property and each `defineModel` call; a props or emits type that is not a type literal or a local interface / object type alias counts 1.",
            "A module with a wide interface is shallow: callers must learn many names instead of one small interface hiding the work.",
            "Hide names only the module's own container needs, group related exports behind one facade, or split the module along its responsibilities.",
            &["max_exports: max exported names (12)"],
        ),
        "stratum/promotion-pressure" => (
            "Reports a module imported by at least `threshold` distinct modules from other containers (today one container per layer) whose layer does not list the module's layer in `depends_on`. Declared importers count toward stratum/miller-limit instead.",
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
