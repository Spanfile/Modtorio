use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Autosave {
    interval: u32,
    slots: u32,
    only_on_server: bool,
    non_blocking: bool,
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
