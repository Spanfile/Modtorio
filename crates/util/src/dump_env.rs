pub fn dump_env(prefix: &str) -> String {
    dump_env_lines(prefix).join("\n")
}

pub fn dump_env_lines(prefix: &str) -> Vec<String> {
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
