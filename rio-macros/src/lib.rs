#![allow(dead_code)]
#![allow(unused_imports)]

use proc_macro::Ident;
use proc_macro::TokenStream;
use proc_macro2::Ident as Ident2;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::parse_quote;
use syn::AngleBracketedGenericArguments;
use syn::ExprAssign;
use syn::ExprPath;
use syn::GenericArgument;
use syn::PathArguments;
use syn::Stmt;
use syn::{parse2, ItemStruct, Lit, Meta, MetaNameValue};

fn get_crate_path(ast: &ItemStruct) -> Ident2 {
    let mut rio_rs = format_ident!("rio_rs");

    for attr in ast.attrs.iter() {
        if attr.path.is_ident("rio_path") {
            let meta_attr = attr.parse_meta().unwrap();
            match meta_attr {
                Meta::NameValue(MetaNameValue {
                    lit: Lit::Str(lit_str),
                    ..
                }) => {
                    rio_rs = format_ident!("{}", lit_str.value());
                }
                _ => panic!("Expected \"[type_name = \"...\"]\""),
            }
        }
    }
    rio_rs
}

#[proc_macro_derive(TypeName, attributes(type_name, rio_path))]
pub fn derive_type_name(tokens: TokenStream) -> TokenStream {
    let input = TokenStream2::from(tokens);
    let ast: ItemStruct = parse2(input).unwrap();
    let struct_name = format_ident!("{}", ast.ident);
    let mut type_name = ast.ident.to_string();
    let rio_rs = get_crate_path(&ast);

    for attr in ast.attrs {
        if attr.path.is_ident("type_name") {
            let meta_attr = attr.parse_meta().unwrap();
            match meta_attr {
                Meta::NameValue(MetaNameValue {
                    lit: Lit::Str(lit_str),
                    ..
                }) => {
                    type_name = lit_str.value();
                }
                _ => panic!("Expected \"[type_name = \"...\"]\""),
            }
        }
    }

    let output = quote! {
        impl #rio_rs::registry::IdentifiableType for #struct_name {
            fn user_defined_type_id() -> &'static str {
                #type_name
            }
        }
    };

    TokenStream::from(output)
}

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

#[proc_macro_derive(FromId, attributes(rio_path))]
pub fn derive_from_id(tokens: TokenStream) -> TokenStream {
    let input = TokenStream2::from(tokens);
    let ast: ItemStruct = parse2(input).unwrap();
    let struct_name = format_ident!("{}", ast.ident);
    let rio_rs = get_crate_path(&ast);

    let output = quote! {
        impl #rio_rs::grain::FromId for #struct_name {
            fn from_id(id: String) -> Self {
                Self {
                    id,
                    ..Self::default()
                }
            }

            fn id(&self) -> &str {
                self.id.as_ref()
            }
        }
    };
    TokenStream::from(output)
}

#[derive(Debug)]
struct StateDefinition {
    crate_path: Ident2,
    struct_name: Ident2,
    // attribute identifier + attribute type + state provider type
    attributes: Vec<(Ident2, GenericArgument, Option<Ident2>)>,
}

impl From<TokenStream2> for StateDefinition {
    fn from(value: TokenStream2) -> Self {
        let ast: ItemStruct = parse2(value).unwrap();

        let crate_path = get_crate_path(&ast);

        let struct_name = format_ident!("{}", ast.ident);
        let mut attributes = vec![];

        for field in ast.fields.iter() {
            if field.attrs.is_empty() {
                continue;
            }

            if !field.attrs.iter().any(|x| x.path.is_ident("managed_state")) {
                continue;
            }

            let attr_ident = match &field.ident {
                Some(ident) => format_ident!("{}", ident),
                _ => continue,
            };

            let mut attr_state_provider_type: Option<Ident2> = None;
            for attr in &field.attrs {
                if attr.path.is_ident("managed_state") {
                    let args: syn::Result<ExprAssign> = attr.parse_args();
                    if args.is_err() {
                        continue;
                    }
                    let args = args.unwrap();
                    match (args.left.as_ref(), args.right.as_ref()) {
                        (
                            syn::Expr::Path(syn::ExprPath {
                                path: left_path, ..
                            }),
                            syn::Expr::Path(syn::ExprPath {
                                path: right_path, ..
                            }),
                        ) => {
                            if !left_path.is_ident("provider") {
                                panic!(
                                    "Only `provider` is supported ({} given)",
                                    left_path.get_ident().unwrap()
                                );
                            }
                            let right_identifier = right_path
                                .get_ident()
                                .expect("provider must be an identifier");
                            attr_state_provider_type = Some(right_identifier.clone());
                        }
                        (_, _) => panic!("not supported"),
                    }
                }
            }

            match &field.ty {
                syn::Type::Path(syn::TypePath {
                    path: syn::Path { segments, .. },
                    ..
                }) => {
                    let segment = segments
                        .first()
                        .expect(&format!("No path value for field {:#?}", field));

                    if segment.ident != "Option" {
                        panic!("Attributes decorated with [managed_state] need to be options");
                    }

                    match &segment.arguments {
                        PathArguments::AngleBracketed(AngleBracketedGenericArguments {
                            args,
                            ..
                        }) => {
                            let type_ = args.first().expect("Couldn't find a type for field");
                            attributes.push((attr_ident, type_.clone(), attr_state_provider_type));
                        }
                        _ => panic!("Value not supported: {:?}", segment.arguments),
                    };
                }
                ty => panic!("Value not supported: {:?}", ty),
            }
        }

        StateDefinition {
            crate_path,
            struct_name,
            attributes,
        }
    }
}

/// Implements State for you struct's attributes. Creating set_state and get_state for each
/// attributed decorated with #[managed_state]
///
/// ```ignore
/// #[derive(Default, FromId, TypeName, ManagedState)]
/// struct Test2 {
///     id: String,
///     #[managed_state(provider = Option)]
///     tests: Option<Vec<u32>>,
/// }
/// ```
#[proc_macro_derive(ManagedState, attributes(rio_path, managed_state))]
pub fn derive_managed_state(tokens: TokenStream) -> TokenStream {
    let input = proc_macro2::TokenStream::from(tokens);
    let managed_state = StateDefinition::from(input);
    let mut states = vec![];
    let mut state_providers = vec![];
    let crate_path = &managed_state.crate_path;
    let struct_name = &managed_state.struct_name;
    for (attribute_ident, attribute_type, attribute_state_provider_ident) in
        managed_state.attributes.iter()
    {
        states.push(quote! {
            impl #crate_path::state_provider::State<#attribute_type> for #struct_name {
                fn get_state(&self) -> Option<&#attribute_type> {
                    self.#attribute_ident.as_ref()
                }
                fn set_state(&mut self, value: Option<#attribute_type>) {
                    self.#attribute_ident = value;
                }
            }
        });

        if let Some(state_provider) = attribute_state_provider_ident {
            state_providers.push(quote! {

                let state_loader = app_data.get::<#state_provider>();
                match self.load_state::<#attribute_type, #state_provider>(state_loader).await  {
                    Ok(_) | Err(#crate_path::errors::LoadStateError::ObjectNotFound)=> (),
                    Err(e) => panic!("Cannot load grain state {:?}", e),
                }

            });
        }
    }

    let state_loader = if state_providers.is_empty() {
        quote! {}
    } else {
        quote! {

            #[async_trait::async_trait]
            impl #crate_path::grain::GrainStateLoad for #struct_name {
                async fn load(&mut self, app_data: &#crate_path::app_data::AppData) -> Result<(), #crate_path::errors::GrainLifeCycleError> {
                    #(#state_providers)*
                    Ok(())
                }
            }

        }
    };

    let output = quote! {
        #(#states)*


        #state_loader
    };
    TokenStream::from(output)
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
            let attr_type: GenericArgument = parse_quote! { StateStruct };
            assert_eq!(state_attr.1, attr_type);
            assert_eq!(state_attr.2, None);
        }
        #[test]
        fn managed_state_impl_with_provider() {
            let input = quote! {
                struct Test {
                    #[managed_state(provider = Option)]
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
            let attr_type: GenericArgument = parse_quote! { StateStruct };
            assert_eq!(state_attr.1, attr_type);
            assert_eq!(
                state_attr.2,
                Some(Ident2::new("Option", Span::mixed_site()))
            );
        }

        #[test]
        #[should_panic]
        fn panic_non_option_managed_state() {
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
