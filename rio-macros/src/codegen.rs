use proc_macro2::TokenStream;

pub(crate) trait Codegen {
    fn codegen(&self) -> TokenStream;
}
