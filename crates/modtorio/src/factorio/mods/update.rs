use util::HumanVersion;

#[derive(Debug)]
pub struct ModUpdate {
    pub name: String,
    pub current_version: HumanVersion,
    pub new_version: HumanVersion,
}
