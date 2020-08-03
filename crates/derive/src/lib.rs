#![feature(proc_macro_diagnostic)]

mod helpers;

extern crate proc_macro;
use heck::SnakeCase;
use helpers::*;
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{parse_macro_input, spanned::Spanned, DeriveInput, FieldsNamed, Ident, Type};
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

/// Implements functions to build SQL queries and parameters based on a given model struct.
///
/// The implemented functions are:
///  * `select()` returns an SQL query string that selects rows based on the marked index (or
///    indices)
///  * `select_all()` returns an SQL query string that selects all rows
///  * `replace_into()` returns an SQL query string that calls REPLACE INTO with a single complete
///    model struct
///  * `insert_into()` returns an SQL query string that calls INSERT INTO with a single complete
///    model struct
///  * `update()` returns an SQL query string that updates existing rows with given values
///  * `select_params(field, ...)` returns a collection of parameters suitable for direct use with
///    rusqlite. The function's parameters are values for the fields tagged as `index` with the
///    corresponding attribute (see below).
///  * `all_params(&self)` returns a collection of parameters suitable for direct use with rusqlite
///    from a certain model struct object. Commonly used together with `replace_into()` and
///    `insert_into()`. Certain fields can be ignored with the `ignore_in_all_params` attribute (see
///    below).
///
/// By default, the derive macro will derive the database table name from model struct's
/// name by converting it into snake case. This can be overridden with the `table_name` attribute
/// (see below).
///
/// The following attributes can additionally be used on either the model struct or its fields:
///  * `#[table_name = "..."]` used on the model struct. Specifies the table name to use instead of
///    the automatically derived default.
///  * `#[index]` specify a certain field to be used as an index when selecting and updating rows.
///    Does not necessarily have to correspond to primary keys on the table. Can be set on multiple
///    fields to use them all together in queries.
///  * `#[ignore_in_all_params]` used together with the `index` attribute to ignore the field when
///    calling `all_params()` on a struct object. Commonly used to ignore an autoincrementing
///    primary key.
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

            /// Returns an SQL `SELECT`-clause that selects rows based on the model's marked indices.
            pub fn select() -> &'static str {
                #select
            }

            /// Returns an SQL `SELECT`-clause that selects all rows.
            pub fn select_all() -> &'static str {
                #select_all
            }

            /// Returns an SQL `REPLACE INTO`-clause that replaces a single row with all columns as values, except those marked with `#[ignore_in_all_params]`.
            pub fn replace_into() -> &'static str {
                #replace_into
            }

            /// Returns an SQL `INSERT INTO`-clause that inserts a single row with all columns as values, except those marked with `#[ignore_in_all_params]`.
            pub fn insert_into() -> &'static str {
                #insert
            }

            /// Returns an SQL `UPDATE`-clause that updates a single row with all columns as values except those marked with `#[ignore_in_all_params]`, selecting the row based on the model's marked indices.
            pub fn update() -> &'static str {
                #update
            }
        }

        #from_row
    ))
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
    let index_params = select_params_fn(fields);
    let all_params = all_params_fn(fields);

    quote!(
        #index_params
        #all_params
    )
}

fn select_params_fn(fields: &[MacroField]) -> TokenStream {
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

    // ptr_arg lint is allowed to allow passing &String (otherwise clippy would complain about
    // passing &str instead). the problem with that is because of the return type, the reference is
    // converted into a dyn ::rusqlite::ToSql trait object, which in case of &str would mean trying
    // to convert the unsized str object into the trait object (not gonna happen). I guess it'd be
    // possible to figure out a way to instead handle the special &str case but this is easier so...
    quote!(
        /// Returns a vector of tuples consisting of a parameter name and its value based on the model's marked indices.
        #[allow(clippy::ptr_arg)]
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
        /// Returns a vector of tuples consisting of a parameter name and its value based on each
        /// field in the model. Fields marked with `#[ignore_in_all_params]` won't be returned.
        pub fn all_params<'a>(&'a self) -> Vec<(&'static str, &'a dyn ::rusqlite::ToSql)> {
            vec![#(#sql_params),*]
        }
    )
}

fn from_row_impl(ident: &Ident, fields: &[MacroField]) -> TokenStream {
    let mut field_setters = Vec::new();

    for (index, field) in fields.iter().enumerate() {
        let field_ident = &field.ident;
        let error_message = format!(
            "column conversion to {}.{} failed",
            ident.to_string(),
            field_ident.to_string()
        );

        field_setters.push(quote!(#field_ident: row.get(#index).expect(#error_message)));
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
