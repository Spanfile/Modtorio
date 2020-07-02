#![feature(proc_macro_diagnostic)]

extern crate proc_macro;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, spanned::Spanned, Data, DeriveInput};

#[proc_macro_derive(Model)]
pub fn model(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    let model_struct = if let Data::Struct(data_struct) = input.data {
        data_struct
    } else {
        input
            .span()
            .unwrap()
            .error("the trait can only be implemented for DICKS")
            .emit();
        return TokenStream::from(quote!());
    };

    unimplemented!()
}
