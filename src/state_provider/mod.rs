#![doc = include_str!("README.md")]

use crate::errors::LoadStateError;
use crate::registry::IdentifiableType;
use crate::{FromId, Grain};
use async_trait::async_trait;
use dashmap::DashMap;
use serde::de::DeserializeOwned;
use serde::Serialize;

pub mod sql;

/// The `StateLoader` defines an interface to load serialized state from a source
///
/// **important** This trait is not responsible for serializing it back to its
/// original type
///
/// TODO use a reader type instead of String on load fn
#[async_trait]
pub trait StateLoader: Sync + Send {
    async fn load<T: DeserializeOwned>(
        &self,
        grain_type: &str,
        grain_id: &str,
        state_type: &str,
    ) -> Result<T, LoadStateError>;
}

/// The `StateSave` defines an interface to save serialized data into a persistence
/// backend (memory, sql server, etc)
///
/// **important** This trait is not responsible for serializing the state from
/// its original type
#[async_trait]
pub trait StateSaver: Sync + Send {
    async fn save(
        &self,
        grain_type: &str,
        grain_id: &str,
        state_type: &str,
        data: &(impl Serialize + Send + Sync),
    ) -> Result<(), LoadStateError>;
}

/// Reponsible for managing states for a specific object
///
/// With this trait one can load/save individual states from an orig (Self) object
#[async_trait]
pub trait ObjectStateManager {
    async fn load_state<T, S>(&mut self, state_loader: &S) -> Result<(), LoadStateError>
    where
        T: IdentifiableType + Serialize + DeserializeOwned,
        S: StateLoader,
        Self: State<T> + IdentifiableType + FromId + Send + Sync,
    {
        let grain_type = Self::user_defined_type_id();
        let grain_id = self.id();
        let state_type = T::user_defined_type_id();
        let data: T = self
            .load(state_loader, grain_type, grain_id, state_type)
            .await
            .or(Err(LoadStateError::ObjectNotFound))?;

        self.set_state(Some(data));
        Ok(())
    }

    async fn save_state<T, S>(&self, state_saver: &S) -> Result<(), LoadStateError>
    where
        T: IdentifiableType + Serialize + DeserializeOwned + Sync,
        S: StateSaver,
        Self: State<T> + IdentifiableType + FromId + Send + Sync,
    {
        let grain_type = Self::user_defined_type_id();
        let grain_id = self.id();

        let state_type = T::user_defined_type_id();
        let state_value: Option<&T> = self.get_state();
        if let Some(state_value) = state_value {
            state_saver
                .save(grain_type, grain_id, state_type, &state_value)
                .await
                .expect("TODO");
        }
        Ok(())
    }
}

// If an struct implements Grain, it gets ObjectStateManager out of the box
impl<T> ObjectStateManager for T where T: Grain {}

/// Trait to define how to get and set states in and out of an object
///
/// One need to implement this trait for each state a object holds. Although the state itself
/// doesn't need to be an Option, this trait works with `Option<T>` only
#[async_trait]
pub trait State<T>
where
    T: Serialize + DeserializeOwned,
{
    fn get_state(&self) -> Option<&T>;
    fn set_state(&mut self, value: Option<T>);

    async fn load<S: StateLoader + Sync + Send>(
        &self,
        state_loader: &S,
        grain_type: &str,
        grain_id: &str,
        state_type: &str,
    ) -> Result<T, LoadStateError> {
        state_loader.load(grain_type, grain_id, state_type).await
    }
}

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
        grain_type: &str,
        grain_id: &str,
        state_type: &str,
    ) -> Result<T, LoadStateError> {
        let grain_type = grain_type.to_string();
        let grain_id = grain_id.to_string();
        let state_type = state_type.to_string();
        let k = (grain_type, grain_id, state_type);

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
        grain_type: &str,
        grain_id: &str,
        state_type: &str,
        data: &(impl Serialize + Send + Sync),
    ) -> Result<(), LoadStateError> {
        let grain_type = grain_type.to_string();
        let grain_id = grain_id.to_string();
        let state_type = state_type.to_string();
        let k = (grain_type, grain_id, state_type);
        self.data
            .insert(k, serde_json::to_string(&data).expect("TODO"));
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use rio_macros::{FromId, ManagedState, TypeName};
    use serde::Deserialize;

    use crate::FromId;

    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[derive(TypeName, Debug, Serialize, Deserialize, PartialEq)]
    #[rio_path = "crate"]
    struct PersonState {
        name: String,
        age: u8,
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq, TypeName)]
    #[rio_path = "crate"]
    struct LegalPersonState {
        legal_name: String,
        id_document: String,
    }

    #[tokio::test]
    async fn sanity_check() -> TestResult {
        let local_state = LocalState::new();
        let state = PersonState {
            name: "Foo".to_string(),
            age: 21,
        };
        local_state.save("a", "1", "PersonState", &state).await?;
        let new_state: PersonState = local_state.load("a", "1", "PersonState").await?;
        assert_eq!(state, new_state);
        Ok(())
    }

    #[tokio::test]
    async fn model_call() -> TestResult {
        #[derive(Debug, Default, FromId, TypeName, ManagedState)]
        #[rio_path = "crate"]
        struct Person {
            id: String,
            #[managed_state]
            person_state: Option<PersonState>,
            #[managed_state]
            legal_state: Option<LegalPersonState>,
        }
        impl ObjectStateManager for Person {}

        impl Person {
            async fn load_all_states<S: StateLoader>(
                &mut self,
                state_loader: &S,
            ) -> Result<(), LoadStateError> {
                self.load_state::<PersonState, _>(state_loader).await?;
                self.load_state::<LegalPersonState, _>(state_loader).await?;
                Ok(())
            }

            async fn save_all_states<S: StateSaver>(
                &mut self,
                state_saver: &S,
            ) -> Result<(), LoadStateError> {
                self.save_state::<PersonState, _>(state_saver).await?;
                self.save_state::<LegalPersonState, _>(state_saver).await?;
                Ok(())
            }
        }

        let local_state = LocalState::new();

        {
            let mut person = Person::from_id("foo".to_string());
            person.person_state = Some(PersonState {
                name: "Foo".to_string(),
                age: 22,
            });
            person.legal_state = Some(LegalPersonState {
                legal_name: "Foo Bla".to_string(),
                id_document: "123.123.123-12".to_string(),
            });
            person.save_all_states(&local_state).await?;
        }
        {
            let mut person = Person::from_id("foo".to_string());
            person.load_all_states(&local_state).await?;
            assert!(person.person_state.is_some());
            assert!(person.legal_state.is_some());
        }
        Ok(())
    }
}
