#![doc = include_str!("README.md")]

use crate::errors::LoadStateError;
use crate::registry::IdentifiableType;
use crate::{ServiceObject, WithId};
use async_trait::async_trait;
use serde::de::DeserializeOwned;
use serde::Serialize;

pub mod local;
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
        object_kind: &str,
        object_id: &str,
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
        object_kind: &str,
        object_id: &str,
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
        Self: State<T> + IdentifiableType + WithId + Send + Sync,
    {
        let object_kind = Self::user_defined_type_id();
        let object_id = self.id();
        let state_type = T::user_defined_type_id();
        let data: T = self
            .load(state_loader, object_kind, object_id, state_type)
            .await
            .or(Err(LoadStateError::ObjectNotFound))?;

        self.set_state(Some(data));
        Ok(())
    }

    async fn save_state<T, S>(&self, state_saver: &S) -> Result<(), LoadStateError>
    where
        T: IdentifiableType + Serialize + DeserializeOwned + Sync,
        S: StateSaver,
        Self: State<T> + IdentifiableType + WithId + Send + Sync,
    {
        let object_kind = Self::user_defined_type_id();
        let object_id = self.id();

        let state_type = T::user_defined_type_id();
        let state_value: Option<&T> = self.get_state();
        if let Some(state_value) = state_value {
            state_saver
                .save(object_kind, object_id, state_type, &state_value)
                .await
                .expect("TODO");
        }
        Ok(())
    }
}

// If an struct implements ServiceObject, it gets ObjectStateManager out of the box
impl<T> ObjectStateManager for T where T: ServiceObject {}

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
        object_kind: &str,
        object_id: &str,
        state_type: &str,
    ) -> Result<T, LoadStateError> {
        state_loader.load(object_kind, object_id, state_type).await
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use rio_macros::{ManagedState, TypeName, WithId};
    use serde::Deserialize;

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
        let local_state = local::LocalState::new();
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
        #[derive(Debug, Default, WithId, TypeName, ManagedState)]
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

        let local_state = local::LocalState::new();

        {
            let mut person = Person::default();
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
            let mut person = Person::default();
            person.load_all_states(&local_state).await?;
            assert!(person.person_state.is_some());
            assert!(person.legal_state.is_some());
        }
        Ok(())
    }
}
