use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse2, ItemStruct, Lit, Meta, MetaNameValue};

#[proc_macro_derive(TypeName, attributes(type_name))]
pub fn derive_type_name(tokens: TokenStream) -> TokenStream {
    let input = proc_macro2::TokenStream::from(tokens);
    let ast: ItemStruct = parse2(input).unwrap();
    let struct_name = format_ident!("{}", ast.ident);
    let mut type_name = ast.ident.to_string();

    for attr in ast.attrs {
        if !attr.path.is_ident("type_name") {
            continue;
        }
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

    let output = quote! {
        impl rio_rs::registry::IdentifiableType for #struct_name {
            fn user_defined_type_id() -> &'static str {
                #type_name
            }
        }
    };
    TokenStream::from(output)
}

#[proc_macro_derive(Message)]
pub fn derive_message(tokens: TokenStream) -> TokenStream {
    let input = proc_macro2::TokenStream::from(tokens);
    let ast: ItemStruct = parse2(input).unwrap();
    let struct_name = format_ident!("{}", ast.ident);

    let output = quote! {
        impl rio_rs::registry::Message for #struct_name {}
    };
    TokenStream::from(output)
}

#[proc_macro_derive(FromId)]
pub fn derive_from_id(tokens: TokenStream) -> TokenStream {
    let input = proc_macro2::TokenStream::from(tokens);
    let ast: ItemStruct = parse2(input).unwrap();
    let struct_name = format_ident!("{}", ast.ident);

    let output = quote! {
        impl rio_rs::grain::FromId for #struct_name {
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
