use rio_macros::*;
use rio_rs::registry::IdentifiableType;

#[derive(TypeName)]
#[type_name = "NotTest"]
struct Test {
    pub a: u32,
}

#[derive(TypeName)]
struct Test2 {
    pub a: u32,
}

fn main() {
    assert_eq!(Test::user_defined_type_id(), "NotTest");
    assert_eq!(Test2::user_defined_type_id(), "Test2");
}
