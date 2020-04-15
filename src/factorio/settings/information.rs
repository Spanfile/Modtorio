use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Information {
    name: String,
    description: String,
    tags: Vec<String>,
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::from_str;

    #[test]
    fn deserialize() -> anyhow::Result<()> {
        let obj: Information = from_str(
            r#"{
"name": "test",
"description": "test",
"tags": [
    "1",
    "2"
]}"#,
        )?;

        assert_eq!(obj.name, "test");
        assert_eq!(obj.description, "test");
        assert_eq!(obj.tags, vec!["1", "2"]);

        Ok(())
    }
}
