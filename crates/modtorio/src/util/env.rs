//! Provides several utilities related to the running program's environment variables.

use std::collections::HashMap;

/// Returns all environment variables of the current process with a given prefix as a string with
/// each variable on its own line.
pub fn dump_string(prefix: &str) -> String {
    dump_lines(prefix).join("\n")
}

/// Returns all environment variables of the current process with a given prefix as a vector with
/// each variable being a single `String` element.
pub fn dump_lines(prefix: &str) -> Vec<String> {
    std::env::vars()
        .filter_map(|(k, v)| {
            if k.starts_with(prefix) {
                Some(format!("{}={}", k, v))
            } else {
                None
            }
        })
        .collect::<Vec<String>>()
}

/// Returns all environment variables of the current process with a given prefix as a map of
/// variable name to its value.
pub fn dump_map(prefix: &str) -> HashMap<String, String> {
    std::env::vars()
        .filter_map(|(k, v)| if k.starts_with(prefix) { Some((k, v)) } else { None })
        .collect::<HashMap<String, String>>()
}
