use proc_macro2::Ident;
use proc_macro2::TokenStream;
use quote::format_ident;
use quote::quote;
use syn::parse2;
use syn::ItemStruct;
use syn::Lit;
use syn::Meta;
use syn::MetaNameValue;

use crate::crate_utilities::get_value_from_meta_name_value;
use crate::get_crate_path;
use crate::Codegen;

pub(crate) struct TypeNameInput {
    rio_rs: Ident,
    struct_name: Ident,
    type_name: String,
}

impl Codegen for TypeNameInput {
    fn codegen(&self) -> TokenStream {
        let rio_rs = &self.rio_rs;
        let struct_name = &self.struct_name;
        let type_name = &self.type_name;

        quote! {
            impl #rio_rs::registry::IdentifiableType for #struct_name {
                fn user_defined_type_id() -> &'static str {
                    #type_name
                }
            }
        }
    }
}

impl From<TokenStream> for TypeNameInput {
    fn from(value: TokenStream) -> Self {
        let ast: ItemStruct = parse2(value).unwrap();
        let struct_name = format_ident!("{}", ast.ident);
        let mut type_name = ast.ident.to_string();
        let rio_rs = get_crate_path(&ast);

        for attr in ast.attrs {
            if attr.path().is_ident("type_name") {
                type_name = get_value_from_meta_name_value(&attr.meta)
                    .expect("Expected \"[type_name = \"...\"]\"")
                    .to_string();
            }
        }

        Self {
            rio_rs,
            struct_name,
            type_name,
        }
    }
}
