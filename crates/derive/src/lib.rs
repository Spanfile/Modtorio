#![feature(proc_macro_diagnostic)]

extern crate proc_macro;
use heck::SnakeCase;
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{
    parse_macro_input, spanned::Spanned, Data, DeriveInput, Field, Fields, FieldsNamed, Meta,
};
use thiserror::Error;

const PRIMARY_KEY_ATTRIBUTE: &str = "primary_key";

#[derive(Error, Debug)]
enum MacroError {
    #[error("the Model derive can only be used on structs")]
    NotAStruct(Span),
    #[error("the Model derive can only be used on structs with named fields")]
    NoNamedFields(Span),
    #[error("field has no identifier")]
    NoIdentOnField(Span),
    #[error(transparent)]
    SynError(#[from] syn::Error),
}

#[derive(Debug)]
struct MacroField {
    is_primary_key: bool,
    identifier: String,
}

#[proc_macro_derive(Model, attributes(primary_key))]
pub fn model(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    let output = match run_macro(input) {
        Ok(token_stream) => token_stream,
        Err(err) => match err {
            MacroError::NoNamedFields(span)
            | MacroError::NotAStruct(span)
            | MacroError::NoIdentOnField(span) => {
                syn::Error::new(span, err.to_string()).to_compile_error()
            }
            MacroError::SynError(err) => err.to_compile_error(),
        },
    };

    proc_macro::TokenStream::from(output)
}

fn run_macro(input: DeriveInput) -> Result<TokenStream, MacroError> {
    let fields = get_fields(&input)?;
    let ident = &input.ident;
    let table_name = ident.to_string().to_snake_case();

    let macro_fields = parse_fields(fields)?;
    let select = select_clause(&table_name, &macro_fields);
    let replace_into = replace_into_clause(&table_name, &macro_fields);
    let insert = insert_into_clause(&table_name, &macro_fields);
    let update = update_clause(&table_name, &macro_fields);

    Ok(quote!(
        impl Model for #ident {
            fn select() -> &'static str {
                #select
            }

            fn replace_into() -> &'static str {
                #replace_into
            }

            fn insert_into() -> &'static str {
                #insert
            }

            fn update() -> &'static str {
                #update
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

fn sql_parameter(ident: &str) -> String {
    format!(":{}", ident)
}

fn sql_equals(ident: &str) -> String {
    format!("{} = {}", ident, sql_parameter(ident))
}

fn has_primary_key_attribute(field: &Field) -> bool {
    for attr in &field.attrs {
        if let Ok(Meta::Path(path)) = attr.parse_meta() {
            if path.is_ident(PRIMARY_KEY_ATTRIBUTE) {
                return true;
            }
        }
    }

    false
}

fn parse_fields(fields: &FieldsNamed) -> Result<Vec<MacroField>, MacroError> {
    let mut macro_fields = Vec::new();

    for field in &fields.named {
        let identifier = field
            .ident
            .as_ref()
            .ok_or_else(|| MacroError::NoIdentOnField(field.span()))?
            .to_string();

        macro_fields.push(MacroField {
            is_primary_key: has_primary_key_attribute(field),
            identifier,
        });
    }

    Ok(macro_fields)
}

fn select_clause(table_name: &str, fields: &[MacroField]) -> String {
    // SELECT * FROM game_mod WHERE game_mod.game = :id
    let mut conditions = Vec::new();

    for field in fields {
        if !field.is_primary_key {
            continue;
        }

        conditions.push(sql_equals(&field.identifier));
    }

    format!(
        "SELECT * FROM {} WHERE {}",
        table_name,
        conditions.join(" AND ")
    )
}

fn replace_into_clause(table_name: &str, fields: &[MacroField]) -> String {
    // REPLACE INTO game_mod (game, factorio_mod, mod_version, mod_zip, zip_checksum) VALUES(:game,
    // :factorio_mod, :mod_version, :mod_zip, :zip_checksum)

    let mut field_names: Vec<&str> = Vec::new();
    let mut values = Vec::new();

    for field in fields {
        field_names.push(&field.identifier);
        values.push(sql_parameter(&field.identifier));
    }

    format!(
        "REPLACE INTO {} ({}) VALUES ({})",
        table_name,
        field_names.join(", "),
        values.join(", "),
    )
}

fn insert_into_clause(table_name: &str, fields: &[MacroField]) -> String {
    // INSERT INTO game (path) VALUES (:path)

    let mut field_names: Vec<&str> = Vec::new();
    let mut values = Vec::new();

    for field in fields {
        if field.is_primary_key {
            continue;
        }

        field_names.push(&field.identifier);
        values.push(sql_parameter(&field.identifier));
    }

    format!(
        "INSERT INTO {} ({}) VALUES ({})",
        table_name,
        field_names.join(", "),
        values.join(", "),
    )
}

fn update_clause(table_name: &str, fields: &[MacroField]) -> String {
    // UPDATE game SET path = :path WHERE id = :id

    let mut updates = Vec::new();
    let mut conditions = Vec::new();

    for field in fields {
        if field.is_primary_key {
            conditions.push(sql_equals(&field.identifier));
        } else {
            updates.push(sql_equals(&field.identifier));
        }
    }

    format!(
        "UPDATE {} SET {} WHERE {}",
        table_name,
        updates.join(", "),
        conditions.join(", "),
    )
}
