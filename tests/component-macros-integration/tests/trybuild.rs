//! trybuild compile-time tests for component_macros
//! trybuild UI tests for component_macros

#[test]
fn trybuild_component_macros() {
    let t = trybuild::TestCases::new();
    t.pass("tests/trybuild/ok_component.rs");
    t.compile_fail("tests/trybuild/fail_missing_lifecycle.rs");
}

#[test]
fn ui_component_macros() {
    let t = trybuild::TestCases::new();
    t.pass("tests/trybuild/component_ok.rs");
    t.compile_fail("tests/trybuild/component_fail.rs");
}
