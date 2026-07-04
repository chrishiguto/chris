//! Unsupported prop types must fail compilation with a clear, scoped error.
#![cfg(feature = "dispatch")]

#[test]
fn unsupported_prop_type_is_a_clear_compile_error() {
    trybuild::TestCases::new().compile_fail("tests/compile_fail/*.rs");
}
