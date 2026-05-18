use rhai::Engine;

/// Construct a locked-down Rhai engine: includes the safe-by-default
/// `StandardPackage` (strings, arrays, maps, math) but disables `eval` and
/// bounds recursion/memory. Rhai's standard package excludes FS/network/process
/// access by construction.
#[must_use]
pub fn build_engine() -> Engine {
    let mut engine = Engine::new();
    engine.disable_symbol("eval");
    engine.set_max_expr_depths(64, 32);
    engine.set_max_call_levels(32);
    engine.set_max_operations(100_000);
    engine.set_max_string_size(8_192);
    engine.set_max_array_size(4_096);
    engine
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn arithmetic_evaluates() {
        let engine = build_engine();
        let result: i64 = engine.eval("40 + 2").unwrap();
        assert_eq!(result, 42);
    }

    #[test]
    fn eval_symbol_is_disabled() {
        let engine = build_engine();
        let err = engine.eval::<i64>(r#"eval("1 + 2")"#);
        assert!(err.is_err(), "eval must be disabled in the sandbox");
    }
}
