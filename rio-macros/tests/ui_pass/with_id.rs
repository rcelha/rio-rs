use rio_macros::*;
use rio_rs::service_object::WithId;

#[derive(Default, WithId)]
struct StructWithId {
    id: String,
    name: String,
}

fn main() {
    let mut a = StructWithId::default();
    a.set_id("1".into());
    assert_eq!(a.id(), "1");
}
