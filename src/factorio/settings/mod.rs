mod autosave;
mod information;
mod limit;
mod pause;
mod publicity;
mod traffic;

use limit::Limit;
use serde::Deserialize;

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

#[derive(Deserialize, Debug, Default)]
pub struct ServerSettings {
    #[serde(flatten)]
    information: information::Information,
    publicity: publicity::Publicity,
    traffic: traffic::Traffic,
    autosave: autosave::Autosave,
    pause: pause::Pause,
    allow_commands: AllowCommands,
}

impl Default for AllowCommands {
    fn default() -> Self {
        Self::AdminsOnly
    }
}
