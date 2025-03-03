use proc_macro2::Ident as Ident2;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::ExprAssign;
use syn::ExprPath;
use syn::PathSegment;
use syn::{parse2, ItemStruct};

use crate::get_crate_path;
use crate::Codegen;

#[derive(Debug)]
pub(crate) struct StateDefinition {
    pub(crate) crate_path: Ident2,
    pub(crate) struct_name: Ident2,
    // attribute identifier + attribute type + state provider type
    pub(crate) attributes: Vec<(Ident2, PathSegment, Option<Ident2>)>,
}

impl Codegen for StateDefinition {
    fn codegen(&self) -> TokenStream2 {
        let mut states = vec![];
        let mut state_providers = vec![];
        let crate_path = &self.crate_path;
        let struct_name = &self.struct_name;
        for (attribute_ident, attribute_type, attribute_state_provider_ident) in
            self.attributes.iter()
        {
            states.push(quote! {
                impl #crate_path::state::State<#attribute_type> for #struct_name {
                    fn get_state(&self) -> &#attribute_type {
                        &self.#attribute_ident
                    }
                    fn set_state(&mut self, value: #attribute_type) {
                        self.#attribute_ident = value;
                    }
                }
            });

            if let Some(state_provider) = attribute_state_provider_ident {
                state_providers.push(quote! {

                    let state_loader = app_data.get::<#state_provider>();
                    match self.load_state::<#attribute_type, #state_provider>(state_loader).await  {
                        Ok(_) | Err(#crate_path::errors::LoadStateError::ObjectNotFound)=> (),
                        Err(e) => panic!("Cannot load ServiceObject state {:?}", e),
                    }

                });
            }
        }

        let state_loader = if state_providers.is_empty() {
            quote! {}
        } else {
            quote! {

                #[async_trait::async_trait]
                impl #crate_path::service_object::ServiceObjectStateLoad for #struct_name {
                    async fn load(&mut self, app_data: &#crate_path::app_data::AppData) -> Result<(), #crate_path::errors::ServiceObjectLifeCycleError> {
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
        output
    }
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

            if !field
                .attrs
                .iter()
                .any(|x| x.path().is_ident("managed_state"))
            {
                continue;
            }

            let attr_ident = match &field.ident {
                Some(ident) => format_ident!("{}", ident),
                _ => continue,
            };

            let mut attr_state_provider_type: Option<Ident2> = None;
            for attr in &field.attrs {
                if attr.path().is_ident("managed_state") {
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
                    attributes.push((attr_ident, segment.clone(), attr_state_provider_type));
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
