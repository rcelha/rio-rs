use rio_macros::*;
use rio_rs::state::ObjectStateManager;
use rio_rs::ServiceObject;
use serde::{Deserialize, Serialize};

#[derive(TypeName, Serialize, Deserialize, Default)]
struct TestVec(Vec<usize>);

#[derive(Default, WithId, TypeName, ManagedState)]
struct TestService {
    id: String,
    // The provider below is not imported
    #[managed_state(provider = SqliteState)]
    tests: TestVec,
}

impl ServiceObject for TestService {}

fn main() {}
