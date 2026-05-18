// Custom rule: flag any module that lives under src/legacy/.
fn check(m) {
    if m.path.contains("/legacy/") {
        return [
            #{
                message: "Module is under src/legacy/ — phase out per migration plan.",
                line: 1,
                suggestion: "Move to the corresponding stable layer.",
            }
        ];
    }
    return [];
}
