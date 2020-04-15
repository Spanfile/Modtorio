use super::Limit;
use serde::Deserialize;

#[derive(Deserialize, Debug, PartialEq)]
enum PublicVisiblity {
    UsernamePassword { username: String, password: String },
    Token(String),
}

#[derive(Deserialize, Debug, PartialEq)]
struct PlayerLimit {
    max: Limit,
    ignore_for_returning: bool,
    autokick: Limit,
}

#[derive(Deserialize)]
pub struct Publicity {
    public: PublicVisiblity,
    lan: bool,
    require_user_verification: bool,
    player_limit: PlayerLimit,
    password: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::from_str;

    #[test]
    fn deserialize() -> anyhow::Result<()> {
        let obj: Publicity = from_str(
            r#"{
"visibility": {
    "public": true,
    "lan": true,
},
"username": "test",
"password": "test",
"token": "test",
"require_user_verification": true,
"max_players": 8,
"ignore_player_limit_for_returning_players": true,
"afk_autokick_interval": 5,
"game_password": "test",
}"#,
        )?;

        assert_eq!(obj.public, PublicVisiblity::Token(String::from("test")));
        assert_eq!(obj.lan, true);
        assert_eq!(obj.require_user_verification, true);
        assert_eq!(
            obj.player_limit,
            PlayerLimit {
                max: Limit::Limited(8),
                ignore_for_returning: true,
                autokick: Limit::Limited(5)
            }
        );
        assert_eq!(obj.password, "test");

        Ok(())
    }
}
