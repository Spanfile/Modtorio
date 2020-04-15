mod autosave;
mod information;
mod pause;
mod publicity;
mod traffic;

use serde::Deserialize;

#[derive(Deserialize, Debug, PartialEq)]
enum Limit {
    Unlimited,
    Limited(u32),
}

#[derive(Deserialize, Debug)]
enum AllowCommands {
    All,
    None,
    AdminsOnly,
}

#[derive(Deserialize, Debug)]
struct Range {
    min: u32,
    max: u32,
}

#[derive(Deserialize)]
pub struct ServerSettings {
    #[serde(flatten)]
    information: information::Information,
    publicity: publicity::Publicity,
    traffic: traffic::Traffic,
    autosave: autosave::Autosave,
    pause: pause::Pause,
    allow_commands: AllowCommands,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::from_str;

    #[test]
    fn deserialize_limit() -> anyhow::Result<()> {
        let unlimited: Limit = from_str("0")?;
        let limited: Limit = from_str("1")?;

        assert_eq!(unlimited, Limit::Unlimited);
        assert_eq!(limited, Limit::Limited(1));

        Ok(())
    }
}
