#[derive(Debug, PartialEq)]
pub enum Limit {
    Unlimited,
    Limited(u32),
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::from_str;

    #[test]
    fn deserialize() -> anyhow::Result<()> {
        let unlimited: Limit = from_str("0")?;
        let limited: Limit = from_str("1")?;

        assert_eq!(unlimited, Limit::Unlimited);
        assert_eq!(limited, Limit::Limited(1));

        Ok(())
    }
}
