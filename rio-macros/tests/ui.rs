#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.pass("tests/ui_pass/*.rs");
    t.compile_fail("tests/ui_fail/*.rs");
}
