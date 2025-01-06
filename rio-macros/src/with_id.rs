use proc_macro::Ident;
use proc_macro::Span;
use proc_macro::TokenStream;
use proc_macro2::Ident as Ident2;
use proc_macro2::TokenStream as TokenStream2;
use quote::ToTokens;
use quote::{format_ident, quote};
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

use crate::get_crate_path;
use crate::Codegen;

pub(crate) struct WithIdInput {
    rio_rs: Ident2,
    struct_name: Ident2,
}

impl Codegen for WithIdInput {
    fn codegen(&self) -> TokenStream2 {
        let rio_rs = &self.rio_rs;
        let struct_name = &self.struct_name;

        quote! {
            impl #rio_rs::service_object::WithId for #struct_name {
                fn set_id(&mut self, id: String) {
                    self.id = id;
                }

                fn id(&self) -> &str {
                    self.id.as_ref()
                }
            }
        }
    }
}

impl From<TokenStream2> for WithIdInput {
    fn from(value: TokenStream2) -> Self {
        let ast: ItemStruct = parse2(value).unwrap();
        let struct_name = format_ident!("{}", ast.ident);
        let rio_rs = get_crate_path(&ast);

        // It has a field `id: String`
        //
        // This check here is not ideal yet, as I need to refine the type detection
        let has_field_id = ast.fields.iter().any(|field| {
            field
                .ident
                .as_ref()
                .is_some_and(|field_ident| field_ident.to_string() == "id".to_string())
                && match &field.ty {
                    syn::Type::Path(path) => {
                        path.path.segments.last().unwrap().ident.to_string() == "String"
                    }
                    _ => false,
                }
        });
        if !has_field_id {
            panic!(
                "{} doesn't have an `id` attribute of type `String`",
                struct_name
            );
        };
        WithIdInput {
            rio_rs,
            struct_name,
        }
    }
}
