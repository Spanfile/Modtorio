#![feature(proc_macro_diagnostic)]

extern crate proc_macro;
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{parse_macro_input, spanned::Spanned, Data, DeriveInput, Fields, FieldsNamed};
use thiserror::Error;

#[derive(Error, Debug)]
enum MacroError {
    #[error("the Model derive can only be used on structs")]
    NotAStruct(Span),
    #[error("the Model derive can only be used on structs with named fields")]
    NoNamedFields(Span),
}

#[proc_macro_derive(Model, attributes(table_name))]
pub fn model(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    let output = match run_macro(input) {
        Ok(token_stream) => token_stream,
        Err(err) => match err {
            MacroError::NoNamedFields(span) | MacroError::NotAStruct(span) => {
                syn::Error::new(span, err.to_string()).to_compile_error()
            }
        },
    };

    proc_macro::TokenStream::from(output)
}

fn run_macro(input: DeriveInput) -> Result<TokenStream, MacroError> {
    let fields = get_fields(&input);
    let ident = &input.ident;

    Ok(quote!(
        impl Model for #ident {
            fn select() -> &'static str {
                unimplemented!()
            }

            fn replace_into() -> &'static str {
                unimplemented!()
            }

            fn insert_into() -> &'static str {
                unimplemented!()
            }

            fn update() -> &'static str {
                unimplemented!()
            }
        }
    ))
}

fn get_fields(input: &DeriveInput) -> Result<&FieldsNamed, MacroError> {
    let data_struct = if let Data::Struct(data_struct) = &input.data {
        data_struct
    } else {
        return Err(MacroError::NotAStruct(input.span()));
    };

    match &data_struct.fields {
        Fields::Named(fields) => Ok(fields),
        _ => Err(MacroError::NoNamedFields(input.span())),
    }
}
