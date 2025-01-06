//! Derive macros to automatically implement the most common
//! traits from [rio_rs](../rio_rs/index.html)

#![allow(dead_code)]
#![allow(unused_imports)]

use proc_macro::Ident;
use proc_macro::Span;
use proc_macro::TokenStream;
use proc_macro2::Ident as Ident2;
use proc_macro2::TokenStream as TokenStream2;
use quote::ToTokens;
use quote::{format_ident, quote};
use registry::RegistryInput;
use registry::RegistryItemHandler;
use syn::parse::Parse;
use syn::parse_macro_input;
use syn::parse_quote;
use syn::punctuated::Punctuated;
use syn::AngleBracketedGenericArguments;
use syn::ExprAssign;
use syn::ExprPath;
use syn::GenericArgument;
use syn::PathArguments;
use syn::PathSegment;
use syn::Stmt;
use syn::Token;
use syn::{parse2, ItemStruct, Lit, Meta, MetaNameValue};

mod codegen;
mod crate_utilities;
mod managed_state;
mod registry;
mod type_name;
mod with_id;

use codegen::Codegen;
use crate_utilities::get_crate_path;
use managed_state::StateDefinition;
use registry::RegistryItemInput;
use type_name::TypeNameInput;
use with_id::WithIdInput;

/// Implements the trait [rio_rs::registry::IdentifiableType]
///
/// # Examples
///
/// ```
/// # use rio_macros::TypeName;
/// # use rio_rs::registry::IdentifiableType;
/// #[derive(Default, TypeName)]
/// struct MyType {
///     attr1: String
/// }
/// assert_eq!(MyType::user_defined_type_id(), "MyType");
/// ```
///
/// You can also override the type name (to avoid collision):
///
/// ```
/// # use rio_macros::TypeName;
/// # use rio_rs::registry::IdentifiableType;
/// mod mod1 {
///     # use rio_macros::TypeName;
///     # use rio_rs::registry::IdentifiableType;
///     #[derive(Default, TypeName)]
///     pub struct MyType {
///         attr1: String
///     }
/// }
/// mod mod2 {
///     # use rio_macros::TypeName;
///     # use rio_rs::registry::IdentifiableType;
///     #[derive(Default, TypeName)]
///     #[type_name = "MySecondType"]
///     pub struct MyType {
///         attr1: String
///     }
/// }
/// assert_eq!(mod1::MyType::user_defined_type_id(), "MyType");
/// assert_eq!(mod2::MyType::user_defined_type_id(), "MySecondType");
/// ```
#[proc_macro_derive(TypeName, attributes(type_name, rio_path))]
pub fn derive_type_name(tokens: TokenStream) -> TokenStream {
    let input = TokenStream2::from(tokens);
    let input = TypeNameInput::from(input);
    let output = input.codegen();
    TokenStream::from(output)
}

/// Implements the [rio_rs::registry::Message] trait for the
/// struct. This is a blank implementation
///
///
/// # Examples
///
/// ```
/// # use rio_macros::Message;
/// # use serde::{Serialize, Deserialize};
/// # use rio_rs::registry::Message;
/// #[derive(Default, Message, Serialize, Deserialize)]
/// struct MyMessage {
///     name: String
/// }
///
/// fn print_message(message: impl Message) {
///     println!("{}", serde_json::to_string_pretty(&message).unwrap());
/// }
/// fn main() {
///     let message = MyMessage::default();
///     print_message(message);
/// }
/// ```
#[proc_macro_derive(Message, attributes(rio_path))]
pub fn derive_message(tokens: TokenStream) -> TokenStream {
    let input = TokenStream2::from(tokens);
    let ast: ItemStruct = parse2(input).unwrap();
    let struct_name = format_ident!("{}", ast.ident);
    let rio_rs = get_crate_path(&ast);

    let output = quote! {
        impl #rio_rs::registry::Message for #struct_name {}
    };
    TokenStream::from(output)
}

/// This macro implements [rio_rs::service_object::WithId], which is needed
/// for writing services using the framework
///
/// Rio relies on messages with identifiable types and types
/// that expose and id field
///
/// To get this macro to work, the struct needs to have an `id` attribute (String)
///
/// If you want to have another type as the id (although its external APIs *needs* to be a
/// [String] and [str]), you will need to implement the trait manually and handle the converstion
///
/// # Example
///
/// ```
/// # use rio_rs::WithId;
/// # use rio_macros::*;
/// #
/// #[derive(Default, WithId)]
/// struct MyService {
///     id: String,
///     name: String
/// }
///
/// let mut my_service_one = MyService::default();
/// assert_eq!(my_service_one.id, "");
/// my_service_one.set_id("one".to_string());
/// assert_eq!(my_service_one.id, "one");
/// ```
#[proc_macro_derive(WithId, attributes(rio_path))]
pub fn derive_with_id(tokens: TokenStream) -> TokenStream {
    let input = TokenStream2::from(tokens);
    let input = WithIdInput::from(input);
    let output = input.codegen();
    TokenStream::from(output)
}

/// Implements State for you struct's attributes. Creating set_state and get_state for each
/// attribute decorated with `#[managed_state]`
///
/// ```rust
/// # use rio_macros::*;
/// # use rio_rs::state::local::LocalState;
/// # use rio_rs::state::ObjectStateManager;
/// # use rio_rs::ServiceObject;
/// # use serde::{Serialize, Deserialize};
/// # #[derive(TypeName, Serialize, Deserialize, Default)]
/// # struct StoredAttribute(Vec<usize>);
/// #[derive(Default, WithId, TypeName, ManagedState)]
/// struct TestService {
///     id: String,
///     #[managed_state(provider = LocalState)]
///     tests: StoredAttribute,
/// }
/// # impl ServiceObject for TestService {}
/// ```
#[proc_macro_derive(ManagedState, attributes(rio_path, managed_state))]
pub fn derive_managed_state(tokens: TokenStream) -> TokenStream {
    let input = proc_macro2::TokenStream::from(tokens);
    let managed_state = StateDefinition::from(input);
    let output = managed_state.codegen();
    TokenStream::from(output)
}

/// This will define a registry, at the same time it will create the registry to be used by the server
/// and the types that the client can use for such registry
#[proc_macro]
pub fn make_registry(tokens: TokenStream) -> TokenStream {
    let input = parse_macro_input!(tokens as RegistryInput);
    let output = input.codegen();
    output.into()
}

#[cfg(test)]
mod test {
    use super::*;

    mod state_definition {
        use proc_macro2::Span;
        use syn::{Type, TypePath};

        use super::*;

        fn struct_naming() {
            let input = quote! {
                struct Test {}
            };
            let state_defo: StateDefinition = StateDefinition::from(input);
            assert_eq!(state_defo.struct_name.to_string(), "Test".to_string());
        }

        #[test]
        fn managed_state_impl() {
            let input = quote! {
                struct Test {
                    #[managed_state]
                    state: Option<StateStruct>,
                }
            };
            let state_defo: StateDefinition = StateDefinition::from(input);

            assert_eq!(
                state_defo.struct_name,
                Ident2::new("Test", Span::mixed_site())
            );

            let state_attr = state_defo.attributes.first().expect("no attribute found");
            assert_eq!(state_attr.0, Ident2::new("state", Span::mixed_site()));
            let attr_type: PathSegment = parse_quote! { Option<StateStruct> };
            assert_eq!(state_attr.1, attr_type);
            // assert_eq!(state_attr.2, None);
        }
        #[test]
        fn managed_state_impl_with_provider() {
            let input = quote! {
                struct Test {
                    #[managed_state(provider = Option)]
                    state: StateStruct,
                }
            };
            let state_defo: StateDefinition = StateDefinition::from(input);

            assert_eq!(
                state_defo.struct_name,
                Ident2::new("Test", Span::mixed_site())
            );

            let state_attr = state_defo.attributes.first().expect("no attribute found");
            assert_eq!(state_attr.0, Ident2::new("state", Span::mixed_site()));
            let attr_type: PathSegment = parse_quote! { StateStruct };
            assert_eq!(state_attr.1, attr_type);
            assert_eq!(
                state_attr.2,
                Some(Ident2::new("Option", Span::mixed_site()))
            );
        }

        #[test]
        fn non_option_managed_state() {
            let input = quote! {
                struct Test {
                    #[managed_state]
                    not_state: String,
                }
            };
            let _: StateDefinition = StateDefinition::from(input);
        }

        #[test]
        fn ignore_non_managed_state_attrs() {
            let input = quote! {
                struct Test {
                    id: String,
                    #[not_managed_state]
                    not_state: String,
                }
            };
            let state_defo: StateDefinition = StateDefinition::from(input);
            assert_eq!(state_defo.struct_name.to_string(), "Test".to_string());
        }
    }
}
