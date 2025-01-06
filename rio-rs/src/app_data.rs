//! Shared data for Rio applications
//!
//! Provides an Container in which one can store data that will be
//! accessed across the application's handlers

use state::Container;

/// Alias for data Container
///
/// It will usually be wrapped in a [`std::sync::Arc`]. This Container does not
/// provide interior mutability.
///
/// More about the container [`struct@state::Container`]
///
/// # Example
/// ```rust
/// # use rio_rs::app_data::{AppData, AppDataExt};
/// let app_data = AppData::new();
/// app_data.set("Test".to_string());
///
/// let value = app_data.get::<String>();
/// assert_eq!(value, "Test");
///
/// let value = app_data.get_or_default::<usize>();
/// assert_eq!(value, &0);
/// ```
pub type AppData = Container!(Send + Sync);

/// Set of utilities to work with [state] crate
pub trait AppDataExt {
    /// Attempts to retrieve the global state for type `T`.
    /// If it hasn't been initialize, a new one `T` will be created
    /// using [Default::default]
    fn get_or_default<T: Default + Send + Sync + 'static>(&self) -> &T;
}

impl AppDataExt for AppData {
    fn get_or_default<T: Default + Send + Sync + 'static>(&self) -> &T {
        match self.try_get::<T>() {
            Some(value) => value,
            None => {
                let value = T::default();
                self.set(value);
                self.get::<T>()
            }
        }
    }
}
