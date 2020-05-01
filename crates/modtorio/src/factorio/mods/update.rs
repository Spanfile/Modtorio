use chrono::{DateTime, Utc};
use util::HumanVersion;

#[derive(Debug)]
pub struct ModUpdate {
    pub name: String,
    pub title: String,
    pub current_version: HumanVersion,
    pub new_version: HumanVersion,
    pub released_on: DateTime<Utc>,
}
