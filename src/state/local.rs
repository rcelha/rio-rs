use super::{StateLoader, StateSaver};
use crate::errors::LoadStateError;
use async_trait::async_trait;
use dashmap::DashMap;
use serde::de::DeserializeOwned;
use serde::Serialize;

/// `LocalState` is a state provider for testing purposes
///
/// It stores all the serialized states into a single `DashMap`
#[derive(Debug)]
pub struct LocalState {
    data: DashMap<(String, String, String), String>,
}

impl LocalState {
    pub fn new() -> LocalState {
        LocalState {
            data: DashMap::new(),
        }
    }
}

#[async_trait]
impl StateLoader for LocalState {
    async fn load<T: DeserializeOwned>(
        &self,
        object_kind: &str,
        object_id: &str,
        state_type: &str,
    ) -> Result<T, LoadStateError> {
        let object_kind = object_kind.to_string();
        let object_id = object_id.to_string();
        let state_type = state_type.to_string();
        let k = (object_kind, object_id, state_type);

        if let Some(x) = self.data.get(&k) {
            Ok(serde_json::from_str(&x).expect("TODO"))
        } else {
            Err(LoadStateError::Unknown)
        }
    }
}

#[async_trait]
impl StateSaver for LocalState {
    async fn save(
        &self,
        object_kind: &str,
        object_id: &str,
        state_type: &str,
        data: &(impl Serialize + Send + Sync),
    ) -> Result<(), LoadStateError> {
        let object_kind = object_kind.to_string();
        let object_id = object_id.to_string();
        let state_type = state_type.to_string();
        let k = (object_kind, object_id, state_type);
        self.data
            .insert(k, serde_json::to_string(&data).expect("TODO"));
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use rio_macros::TypeName;
    use serde::Deserialize;

    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[derive(TypeName, Debug, Serialize, Deserialize, PartialEq)]
    #[rio_path = "crate"]
    struct TestState {
        name: String,
    }

    #[tokio::test]
    async fn sanity_check() -> TestResult {
        let local_state = LocalState::new();
        let state = TestState {
            name: "Foo".to_string(),
        };
        local_state.save("a", "1", "TestState", &state).await?;
        let new_state: TestState = local_state.load("a", "1", "TestState").await?;
        assert_eq!(state, new_state);
        Ok(())
    }
}
