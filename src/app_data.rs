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
/// # use rio_rs::app_data::AppData;
/// let app_data = AppData::new();
/// app_data.set("Test".to_string());
///
/// let value = app_data.get::<String>();
/// assert_eq!(value, "Test");
/// ```
pub type AppData = Container!(Send + Sync);
