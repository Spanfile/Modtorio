use serde::Deserialize;

#[derive(Deserialize, Debug, PartialEq)]
pub struct Information {
    pub name: String,
    pub description: String,
    pub tags: Vec<String>,
}

impl Default for Information {
    fn default() -> Self {
        Self {
            name: String::from("A Factorio server"),
            description: String::from("A Factorio server"),
            tags: Vec::new(),
        }
    }
}
