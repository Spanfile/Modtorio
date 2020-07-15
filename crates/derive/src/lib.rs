#![feature(proc_macro_diagnostic)]

extern crate proc_macro;
use heck::SnakeCase;
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{
    parse_macro_input, spanned::Spanned, Attribute, Data, DeriveInput, Fields, FieldsNamed, Ident,
    Lit, Meta, Type,
};
use thiserror::Error;

const INDEX_ATTRIBUTE: &str = "index";
const IGNORE_IN_ALL_PARAMS_ATTRIBUTE: &str = "ignore_in_all_params";
const TABLE_NAME_ATTRIBUTE: &str = "table_name";

#[derive(Error, Debug)]
enum MacroError {
    #[error("the Model derive can only be used on structs")]
    NotAStruct(Span),
    #[error("the Model derive can only be used on structs with named fields")]
    NoNamedFields(Span),
    #[error("field has no identifier")]
    NoIdentOnField(Span),
    #[error("expected string literal")]
    ExpectedStringLiteral(Span),
    #[error(transparent)]
    SynError(#[from] syn::Error),
}

#[derive(Debug)]
struct MacroField {
    is_index: bool,
    ignore_in_all_params: bool,
    ident: Ident,
    ty: Type,
}

#[proc_macro_derive(Model, attributes(index, ignore_in_all_params, table_name))]
pub fn model(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    let output = match run_macro(input) {
        Ok(token_stream) => token_stream,
        Err(err) => match err {
            MacroError::NoNamedFields(span)
            | MacroError::NotAStruct(span)
            | MacroError::NoIdentOnField(span)
            | MacroError::ExpectedStringLiteral(span) => {
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
    let table_name =
        if let Some(table_name) = get_attribute_value(&input.attrs, TABLE_NAME_ATTRIBUTE)? {
            table_name
        } else {
            ident.to_string().to_snake_case()
        };

    let macro_fields = parse_fields(fields)?;

    let select = select_clause(&table_name, &macro_fields);
    let select_all = select_all_clause(&table_name);
    let replace_into = replace_into_clause(&table_name, &macro_fields);
    let insert = insert_into_clause(&table_name, &macro_fields);
    let update = update_clause(&table_name, &macro_fields);

    let params = params_fns(&macro_fields);
    let from_row = from_row_impl(&ident, &macro_fields);

    Ok(quote!(
        impl #ident {
            #params

            pub fn select() -> &'static str {
                #select
            }

            pub fn select_all() -> &'static str {
                #select_all
            }

            pub fn replace_into() -> &'static str {
                #replace_into
            }

            pub fn insert_into() -> &'static str {
                #insert
            }

            pub fn update() -> &'static str {
                #update
            }
        }

        #from_row
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

fn has_attribute(attrs: &[Attribute], attribute: &str) -> bool {
    for attr in attrs.iter() {
        if let Ok(Meta::Path(path)) = attr.parse_meta() {
            if path.is_ident(attribute) {
                return true;
            }
        }
    }

    false
}

fn get_attribute_value(attrs: &[Attribute], attribute: &str) -> Result<Option<String>, MacroError> {
    for attr in attrs.iter() {
        if let Ok(Meta::NameValue(name_value)) = attr.parse_meta() {
            if name_value.path.is_ident(attribute) {
                return match &name_value.lit {
                    Lit::Str(lit_str) => Ok(Some(lit_str.value())),
                    _ => Err(MacroError::ExpectedStringLiteral(attr.span())),
                };
            }
        }
    }

    Ok(None)
}

fn parse_fields(fields: &FieldsNamed) -> Result<Vec<MacroField>, MacroError> {
    let mut macro_fields = Vec::new();

    for field in &fields.named {
        let ident = field
            .ident
            .clone()
            .ok_or_else(|| MacroError::NoIdentOnField(field.span()))?;
        let ty = field.ty.clone();

        macro_fields.push(MacroField {
            is_index: has_attribute(&field.attrs, INDEX_ATTRIBUTE),
            ignore_in_all_params: has_attribute(&field.attrs, IGNORE_IN_ALL_PARAMS_ATTRIBUTE),
            ident,
            ty,
        });
    }

    Ok(macro_fields)
}

fn select_clause(table_name: &str, fields: &[MacroField]) -> String {
    // SELECT * FROM game_mod WHERE game_mod.game = :id
    let mut conditions = Vec::new();

    for field in fields {
        if !field.is_index {
            continue;
        }

        conditions.push(sql_equals(&field.ident.to_string()));
    }

    format!(
        "SELECT * FROM {} WHERE {}",
        table_name,
        conditions.join(" AND ")
    )
}

fn select_all_clause(table_name: &str) -> String {
    // SELECT * FROM game_mod
    format!("SELECT * FROM {}", table_name)
}

fn replace_into_clause(table_name: &str, fields: &[MacroField]) -> String {
    // REPLACE INTO game_mod (game, factorio_mod, mod_version, mod_zip, zip_checksum) VALUES(:game,
    // :factorio_mod, :mod_version, :mod_zip, :zip_checksum)

    let mut field_names = Vec::new();
    let mut values = Vec::new();

    for field in fields {
        let ident = field.ident.to_string();
        values.push(sql_parameter(&ident));
        field_names.push(ident);
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

    let mut field_names = Vec::new();
    let mut values = Vec::new();

    for field in fields {
        if field.is_index {
            continue;
        }

        let ident = field.ident.to_string();
        values.push(sql_parameter(&ident));
        field_names.push(ident);
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
        let ident = field.ident.to_string();

        if field.is_index {
            conditions.push(sql_equals(&ident));
        } else {
            updates.push(sql_equals(&ident));
        }
    }

    format!(
        "UPDATE {} SET {} WHERE {}",
        table_name,
        updates.join(", "),
        conditions.join(", "),
    )
}

fn params_fns(fields: &[MacroField]) -> TokenStream {
    let index_params = index_params_fn(fields);
    let all_params = all_params_fn(fields);

    quote!(
        #index_params
        #all_params
    )
}

fn index_params_fn(fields: &[MacroField]) -> TokenStream {
    let mut fn_params = Vec::new();
    let mut sql_params = Vec::new();

    for field in fields {
        if !field.is_index {
            continue;
        }

        let ident = &field.ident;
        let ty = &field.ty;
        let sql_param = sql_parameter(&ident.to_string());

        fn_params.push(quote!(#ident: &'a #ty));
        sql_params.push(quote!((#sql_param, #ident)));
    }

    quote!(
        pub fn select_params<'a>(#(#fn_params),*) -> Vec<(&'static str, &'a dyn ::rusqlite::ToSql)> {
            vec![#(#sql_params),*]
        }
    )
}

fn all_params_fn(fields: &[MacroField]) -> TokenStream {
    let mut sql_params = Vec::new();

    for field in fields {
        if field.ignore_in_all_params {
            continue;
        }

        let ident = &field.ident;
        let sql_param = sql_parameter(&ident.to_string());

        sql_params.push(quote!((#sql_param, &self.#ident)));
    }

    quote!(
        pub fn all_params<'a>(&'a self) -> Vec<(&'static str, &'a dyn ::rusqlite::ToSql)> {
            vec![#(#sql_params),*]
        }
    )
}

fn from_row_impl(ident: &Ident, fields: &[MacroField]) -> TokenStream {
    let mut field_setters = Vec::new();

    for (index, field) in fields.iter().enumerate() {
        let field_ident = &field.ident;
        field_setters.push(
            quote!(#field_ident: row.get(#index).expect("column conversion to field failed")),
        );
    }

    quote!(
        impl From<&::rusqlite::Row<'_>> for #ident {
            fn from(row: &::rusqlite::Row) -> Self {
                Self {
                    #(#field_setters),*
                }
            }
        }
    )
}
