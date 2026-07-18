/// `base` plus caller spacing utilities; no trailing space when empty.
pub(crate) fn classed(base: &'static str, spacing: &'static str) -> String {
    if spacing.is_empty() {
        base.to_string()
    } else {
        format!("{base} {spacing}")
    }
}
