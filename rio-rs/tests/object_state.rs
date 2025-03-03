use async_trait::async_trait;
use rio_macros::{ManagedState, TypeName, WithId};
use rio_rs::{
    registry::IdentifiableType,
    state::{local::LocalState, ObjectStateManager, State, StateLoader, StateSaver},
    ServiceObject, WithId,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, WithId, TypeName)]
struct TestService {
    id: String,
    state: TestState1,
}
impl ServiceObject for TestService {}

#[async_trait]
impl State<TestState1> for TestService {
    fn get_state(&self) -> &TestState1 {
        &self.state
    }
    fn set_state(&mut self, value: TestState1) {
        self.state = value;
    }
}

async fn persist_state_for_object_test(state_manager: impl StateLoader + StateSaver) {
    // Local Storage
    let mut svc_object = TestService::default();
    svc_object.set_id("one".to_string());

    svc_object
        .save_state::<TestState1, _>(&state_manager)
        .await
        .unwrap();

    assert_eq!(svc_object.state.data, "".to_string());

    svc_object.state.data = "data1".to_string();
    svc_object
        .save_state::<TestState1, _>(&state_manager)
        .await
        .unwrap();

    let mut svc_object = TestService::default();
    svc_object.set_id("one".to_string());
    svc_object
        .load_state::<TestState1, _>(&state_manager)
        .await
        .unwrap();
    assert_eq!(svc_object.state.data, "data1".to_string());
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct TestState1 {
    data: String,
}
impl IdentifiableType for TestState1 {}

#[cfg(feature = "local")]
mod local {
    use super::*;

    #[tokio::test]
    async fn persist_state_for_object() {
        let state_manager = LocalState::new();
        persist_state_for_object_test(state_manager).await;
    }
}

#[cfg(feature = "sqlite")]
mod sqlite {
    use super::*;
    use rio_rs::{
        state::{sqlite::SqliteState, ObjectStateManager, StateSaver},
        ServiceObject,
    };

    #[derive(Debug, Default, WithId, TypeName, ManagedState)]
    struct TestService2 {
        id: String,

        #[managed_state(provider = SqliteState)]
        state: TestState1,
    }

    impl ServiceObject for TestService2 {}

    #[tokio::test]
    async fn persist_state_for_object() {
        let pool = SqliteState::pool()
            .connect("sqlite://:memory:")
            .await
            .unwrap();
        let state_manager = SqliteState::new(pool);
        StateSaver::prepare(&state_manager).await;
        persist_state_for_object_test(state_manager).await;
    }

    /// Tests how macro and non-macro interoperate
    mod mixed_macro {
        use super::super::*;
        use super::*;
        use rio_rs::{prelude::AppData, ServiceObjectStateLoad};

        #[tokio::test]
        async fn persist_state_for_object() {
            // Save and load using save_state directly
            let state_manager = LocalState::new();

            let mut svc_object = TestService2::default();
            svc_object.set_id("one".to_string());
            svc_object.state.data = "data1".to_string();

            svc_object
                .save_state::<TestState1, _>(&state_manager)
                .await
                .unwrap();

            let mut svc_object = TestService2::default();
            svc_object.set_id("one".to_string());
            svc_object
                .load_state::<TestState1, _>(&state_manager)
                .await
                .unwrap();
            assert_eq!(svc_object.state.data, "data1".to_string());

            // And now save it using redis and ...
            let pool = SqliteState::pool()
                .connect("sqlite://:memory:")
                .await
                .unwrap();
            let state_manager = SqliteState::new(pool);
            StateSaver::prepare(&state_manager).await;

            svc_object
                .save_state::<TestState1, _>(&state_manager)
                .await
                .unwrap();

            let context = AppData::new();
            context.set(state_manager);

            // .. re-load it using `load` (loads all)
            let mut svc_object = TestService2::default();
            svc_object.set_id("one".to_string());
            ServiceObjectStateLoad::load(&mut svc_object, &context)
                .await
                .unwrap();
            assert_eq!(svc_object.state.data, "data1".to_string());
        }
    }
}
