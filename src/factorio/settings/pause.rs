use serde::Deserialize;

#[derive(Deserialize)]
pub struct Pause {
    auto: bool,
    only_admins: bool,
}
