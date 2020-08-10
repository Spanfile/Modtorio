use super::MacroError;
use syn::{spanned::Spanned, Attribute, Data, DeriveInput, Fields, FieldsNamed, Lit, Meta};

pub(crate) fn get_fields(input: &DeriveInput) -> Result<&FieldsNamed, MacroError> {
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

pub(crate) fn sql_parameter(ident: &str) -> String {
    format!(":{}", ident)
}

pub(crate) fn sql_equals(ident: &str) -> String {
    format!("{} = {}", ident, sql_parameter(ident))
}

pub(crate) fn has_attribute(attrs: &[Attribute], attribute: &str) -> bool {
    for attr in attrs.iter() {
        if let Ok(Meta::Path(path)) = attr.parse_meta() {
            if path.is_ident(attribute) {
                return true;
            }
        }
    }

    false
}

pub(crate) fn get_attribute_value(attrs: &[Attribute], attribute: &str) -> Result<Option<String>, MacroError> {
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
