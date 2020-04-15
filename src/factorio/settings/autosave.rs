use serde::Deserialize;

#[derive(Deserialize)]
pub struct Autosave {
    interval: u32,
    slots: u32,
    only_on_server: bool,
    non_blocking: bool,
}
