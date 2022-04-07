use rio_macros::*;
use rio_rs::state_provider::*;

#[derive(ManagedState)]
struct Test {}

#[derive(TypeName, serde::Serialize, serde::Deserialize)]
struct TestVec(Vec<usize>);

#[derive(TypeName, serde::Serialize, serde::Deserialize)]
struct NotTestVec(Vec<usize>);

#[derive(Default, FromId, TypeName, ManagedState)]
struct Test2 {
    id: String,
    #[managed_state]
    tests: Option<TestVec>,
    // #[managed_state]
    // not_tests: Option<NotTestVec>,
}

#[derive(Default, FromId, TypeName, ManagedState)]
struct TestProvider {
    id: String,
    #[managed_state(provider = LocalState)]
    tests: Option<TestVec>,
}

fn main() {}
