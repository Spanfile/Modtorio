use serde::Deserialize;

#[derive(Deserialize, Debug, PartialEq)]
pub struct Autosave {
    #[serde(rename = "autosave_interval")]
    pub interval: u32,
    #[serde(rename = "autosave_slots")]
    pub slots: u32,
    #[serde(rename = "autosave_only_on_server")]
    pub only_on_server: bool,
    #[serde(rename = "non_blocking_saving")]
    pub non_blocking: bool,
}

impl Default for Autosave {
    fn default() -> Self {
        Self {
            interval: 5,
            slots: 5,
            only_on_server: true,
            non_blocking: false,
        }
    }
}
