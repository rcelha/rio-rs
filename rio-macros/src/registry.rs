use heck::AsSnakeCase;
use heck::ToSnakeCase;
use proc_macro::Ident;
use proc_macro::Span;
use proc_macro2::Ident as Ident2;
use proc_macro2::Span as Span2;
use proc_macro2::TokenStream as TokenStream2;
use quote::format_ident;
use quote::quote;
use syn::parse::Parse;
use syn::parse::ParseStream;
use syn::token::Group;
use syn::Token;

use crate::Codegen;

#[derive(Debug, Clone)]
pub(crate) struct RegistryItemHandler {
    input: Ident2,
    output: Ident2,
    error: Ident2,
}

impl Parse for RegistryItemHandler {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut self_ = RegistryItemHandler {
            input: Ident2::new("tmp", Span2::call_site()),
            output: Ident2::new("tmp", Span2::call_site()),
            error: Ident2::new("tmp", Span2::call_site()),
        };

        let lookahead = input.lookahead1();
        if lookahead.peek(syn::Ident) {
            // find the input type first (message)
            let input_type = input.parse::<syn::Ident>()?;
            self_.input = input_type;

            // Now find the ok and err types as in `=> (ok type, err type)`
            input.parse::<Token![=>]>()?;
            let res_types;
            syn::parenthesized!(res_types in input);
            let mut res_types = res_types.parse_terminated(Ident2::parse, Token![,])?;

            // And extract the types
            self_.error = res_types.pop().expect("Missing error type").into_value();
            self_.output = res_types.pop().expect("Missing output type").into_value();
        } else {
            return Err(lookahead.error());
        }

        Ok(self_)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct RegistryItemInput {
    pub(crate) service: Ident2,
    pub(crate) handlers: Vec<RegistryItemHandler>,
}

impl Parse for RegistryItemInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        let service_name = if lookahead.peek(syn::Ident) {
            let service_name = input.parse::<syn::Ident>()?;
            // Ensure we use `:` as a separator between the service and the handlers
            input.parse::<Token![:]>()?;
            service_name
        } else {
            return Err(lookahead.error());
        };

        let lookahead = input.lookahead1();
        let handlers = if lookahead.peek(syn::token::Bracket) {
            let content;
            syn::bracketed!(content in input);
            let handlers = content.parse_terminated(RegistryItemHandler::parse, Token![,])?;
            let handlers: Vec<RegistryItemHandler> = handlers.into_iter().collect();
            handlers
        } else {
            return Err(lookahead.error());
        };

        let registry_item = RegistryItemInput {
            service: service_name,
            handlers,
        };
        Ok(registry_item)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct RegistryInput {
    pub(crate) service: Vec<RegistryItemInput>,
}

impl Parse for RegistryInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut registry_input = RegistryInput { service: vec![] };
        let handlers = input.parse_terminated(RegistryItemInput::parse, Token![,])?;
        registry_input.service = handlers.into_iter().collect();
        Ok(registry_input)
    }
}

impl Codegen for RegistryInput {
    fn codegen(&self) -> TokenStream2 {
        // codegen for the server module
        let mut server_code_fragments = vec![];

        for service in self.service.iter() {
            // Adds the type to the registry
            let service_name = &service.service;
            let fragment = quote! {
                reg.add_type::<super::#service_name>();
            };
            server_code_fragments.push(fragment);

            // Now it will add each handler for this type
            for handlers_def in &service.handlers {
                let input_ = &handlers_def.input;
                let output_ = &handlers_def.output;
                let error_ = &handlers_def.error;
                let fragment = quote! {
                    reg.add_handler::<super::#service_name, super::#input_>();
                    assert_handler_type::<super::#service_name, super::#input_, super::#output_, super::#error_>();
                };
                server_code_fragments.push(fragment);
            }
        }
        // complete fragment for the registry fn
        let server_registry_fragment = quote! {
            pub fn registry() -> rio_rs::registry::Registry {
                let mut reg = rio_rs::registry::Registry::new();
                #(#server_code_fragments);*
                reg
            }
        };

        // codegen for the client module
        let mut client_code_fragments: Vec<TokenStream2> = vec![];

        for service in self.service.iter() {
            let mut module_fragment = vec![];
            // Adds the type to the registry
            let service_name = &service.service;
            let service_name_str = service_name.to_string();
            let service_snake = service_name_str.to_snake_case();
            let module = Ident2::new(&service_snake, Span2::call_site());
            for handlers_def in &service.handlers {
                let input_ = &handlers_def.input;
                let output_ = &handlers_def.output;
                let error_ = &handlers_def.error;

                let input_snake = input_.to_string().to_snake_case();
                let fn_name = format!("send_{}", input_snake);
                let fn_name = Ident2::new(&fn_name, Span2::call_site());

                let fragment = quote! {
                    pub async fn #fn_name<S>(
                        client: &mut rio_rs::client::Client<S>,
                        object_id: impl AsRef<str>,
                        msg: &super::super::#input_,
                    ) -> Result<super::super::#output_, rio_rs::protocol::RequestError<super::super::#error_>>
                    where S: rio_rs::cluster::storage::MembersStorage + 'static,
                    {
                        let ret_msg = client
                            .send(#service_name_str, object_id, msg)
                            .await?;
                        Ok(ret_msg)
                    }
                };
                module_fragment.push(fragment);
            }
            let module_fragment = quote! {
                pub mod #module {
                    #(#module_fragment)*
                }
            };
            client_code_fragments.push(module_fragment);
        }

        let client_fragment = quote! {
            #(#client_code_fragments)*
        };

        // Now we render both modules and return it
        quote! {
            pub mod server {
                fn assert_handler_type<T, I, O, E>() where
                    T: 'static + rio_rs::registry::Handler<I, Returns=O, Error=E> + Send + Sync,
                    I: rio_rs::registry::Message + Send + Sync,
                    O: Send + Sync,
                    E: Send + Sync,
                {}

                #server_registry_fragment
            }

            pub mod client {
                #client_fragment
            }
        }
    }
}
