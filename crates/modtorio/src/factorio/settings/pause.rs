use serde::Deserialize;

#[derive(Deserialize, Debug, PartialEq)]
pub struct Pause {
    #[serde(rename = "auto_pause")]
    pub auto: bool,
    #[serde(rename = "only_admins_can_pause_the_game")]
    pub only_admins: bool,
}

impl Default for Pause {
    fn default() -> Self {
        Self {
            auto: true,
            only_admins: true,
        }
    }
}
