//! Provides the `Command` enum, which represents the various console commands in the server.

use rpc::send_command_request;

/// Represents the various console commands in the server.
#[derive(Debug)]
pub enum Command {
    /// The raw command: `/<arguments>`.
    Raw(Vec<String>),
    /// The say command: `<message>`.
    Say(String),
    /// The whisper command: `/whisper <player> <message>`.
    Whisper {
        /// The player to whisper to.
        player: String,
        /// The message.
        message: String,
    },
    /// The save command: `/save <save name>`.
    Save(String),
    /// The quit command: `/quit`.
    Quit,
    /// The ban command: `/ban <player> <reason>`.
    Ban {
        /// The player to ban.
        player: String,
        /// Reason for the ban.
        reason: String,
    },
    /// The unban command: `/unban <player>`.
    Unban(String),
    /// The kick command: `/kick <player> <reason>`.
    Kick {
        /// The player to kick.
        player: String,
        /// Reason for the kick.
        reason: String,
    },
    /// The mute command: `/mute <player>`.
    Mute(String),
    /// The unmute command: `/unmute <player>`.
    Unmute(String),
}

impl From<send_command_request::Command> for Command {
    fn from(comm: send_command_request::Command) -> Self {
        match comm {
            send_command_request::Command::Raw(raw) => Self::Raw(raw.arguments),
            send_command_request::Command::Save(save) => Self::Save(save.save_name),
            send_command_request::Command::Quit(_) => Self::Quit,
            send_command_request::Command::Say(say) => Self::Say(say.message),
            send_command_request::Command::Whisper(whisper) => Self::Whisper {
                player: whisper.player,
                message: whisper.message,
            },
            send_command_request::Command::Ban(ban) => Self::Ban {
                player: ban.player,
                reason: ban.reason,
            },
            send_command_request::Command::Unban(unban) => Self::Unban(unban.player),
            send_command_request::Command::Kick(kick) => Self::Kick {
                player: kick.player,
                reason: kick.reason,
            },
            send_command_request::Command::Mute(mute) => Self::Mute(mute.player),
            send_command_request::Command::Unmute(unmute) => Self::Unmute(unmute.player),
        }
    }
}

impl Command {
    /// Returns the in-game command string for this command.
    pub fn get_command_string(&self) -> String {
        let command = match self {
            Command::Raw(arguments) => format!("/{}", arguments.join(" ")),
            Command::Say(message) => message.to_string(),
            Command::Whisper { player, message } => format!("/whisper {} {}", player, message),
            Command::Save(save_name) => format!("/save {}", save_name),
            Command::Quit => "/quit".to_string(),
            Command::Ban { player, reason } => format!("/ban {} {}", player, reason),
            Command::Unban(player) => format!("/unban {}", player),
            Command::Kick { player, reason } => format!("/kick {} {}", player, reason),
            Command::Mute(player) => format!("/mute {}", player),
            Command::Unmute(player) => format!("/unmute {}", player),
        };

        format!("{}\n", command)
    }
}
