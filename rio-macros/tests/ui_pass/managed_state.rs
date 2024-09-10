use rio_macros::*;
use rio_rs::state::local::LocalState;
use rio_rs::state::ObjectStateManager;
use rio_rs::ServiceObject;

#[derive(ManagedState)]
struct Test {}

#[derive(TypeName, serde::Serialize, serde::Deserialize, Default)]
struct TestVec(Vec<usize>);

#[derive(TypeName, serde::Serialize, serde::Deserialize, Default)]
struct NotTestVec(Vec<usize>);

#[derive(Default, WithId, TypeName, ManagedState)]
struct Test2 {
    id: String,
    #[managed_state]
    tests: Option<TestVec>,
    // #[managed_state]
    // not_tests: Option<NotTestVec>,
}

#[derive(Default, WithId, TypeName, ManagedState)]
struct TestProvider {
    id: String,
    #[managed_state(provider = LocalState)]
    tests: TestVec,
}

impl ServiceObject for TestProvider {}

fn main() {}
