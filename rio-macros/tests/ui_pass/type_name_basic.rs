use rio_macros::*;
use rio_rs::registry::IdentifiableType;

#[derive(TypeName)]
struct Test {
    pub a: u32,
}

fn main() {
    assert_eq!(Test::user_defined_type_id(), "Test");
}
