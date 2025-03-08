use rio_macros::*;
use rio_rs::state::local::LocalState;
use rio_rs::state::ObjectStateManager;

#[derive(TypeName, serde::Serialize, serde::Deserialize, Default)]
struct TestVec(Vec<usize>);

#[derive(Default, WithId, TypeName, ManagedState)]
struct TestService {
    id: String,
    #[managed_state(provider = LocalState)]
    tests: TestVec,
}

// This is the missing bit:
// impl ServiceObject for TestService {}

fn main() {}
