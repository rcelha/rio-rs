#![allow(dead_code)]
#![allow(unused_imports)]

use proc_macro2::Ident as Ident2;
use quote::{format_ident, ToTokens};
use syn::{Expr, ItemStruct, Lit, LitStr, Meta, MetaNameValue, Path};

/// Helper function to allow changing what is the
/// name of the [rio_rs] crate
pub(crate) fn get_crate_path(ast: &ItemStruct) -> Ident2 {
    let mut rio_rs = format_ident!("rio_rs");

    for attr in ast.attrs.iter() {
        let attr_path = attr.path();
        if attr_path.is_ident("rio_path") {
            // Get the value portion of `rio_path = "VALUE PORTION"` as string
            // to identify how to import/use rio_rs
            rio_rs = get_value_from_meta_name_value(&attr.meta)
                .expect("Expected \"[rio_path = \"...\"]\"");
        }
    }
    rio_rs
}

/// Extract the value portion of `key = "VALUE"` as an Ident
pub(crate) fn get_value_from_meta_name_value(meta: &Meta) -> Result<Ident2, &'static str> {
    match meta {
        Meta::NameValue(MetaNameValue {
            value: Expr::Lit(val),
            ..
        }) => {
            // Extract the value as string...
            let attr_value = match &val.lit {
                Lit::Str(lit_str) => lit_str.value(),
                _ => return Err("Invalid literal type"),
            };
            // ...then convert it to Ident
            Ok(format_ident!("{}", attr_value))
        }
        _ => return Err("Invalid literal type"),
    }
}
