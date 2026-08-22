use proc_macro::TokenStream;
use quote::quote;
use syn::{Ident, parse_macro_input};

pub(crate) fn cast_cell_impl(input: TokenStream) -> TokenStream {
    let ty = parse_macro_input!(input as Ident);

    let expanded = quote! {
        hilen::refs::weak_from_ref(cell.downcast_mut::<#ty>().unwrap())
    };

    TokenStream::from(expanded)
}
