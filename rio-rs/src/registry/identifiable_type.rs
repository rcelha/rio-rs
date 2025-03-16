//! Trait object registry
//!
//! Provides storage for objects and maps their callables to handle registered message types

use std::any::type_name;

/// Define a name for a given Struct so it can be used at runtime.
///
/// By default this will use [std::any::type_name] (which might not be compatible across all your
/// infrastructure), so it is advised to implement your own 'user_defined_type_id'
///
/// <div class="warning">This won't deal with duplicates. But the registry takes care of it</div>
pub trait IdentifiableType {
    fn user_defined_type_id() -> &'static str {
        type_name::<Self>()
    }

    /// Same as IdentifiableType::user_defined_type_id, but it can be
    /// called directly from the struct instance. This is handy for when
    /// one uses impl Trait instead of generic
    fn instance_type_id(&self) -> &'static str {
        Self::user_defined_type_id()
    }
}
