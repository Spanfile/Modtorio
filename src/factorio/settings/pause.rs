use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Pause {
    auto: bool,
    only_admins: bool,
}

impl Default for Pause {
    fn default() -> Self {
        Self {
            auto: true,
            only_admins: true,
        }
    }
}
