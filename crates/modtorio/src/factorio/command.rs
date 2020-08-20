//! Provides the `Command` enum, which represents the various console commands in the server.

/// Represents the various console commands in the server.
#[derive(Debug)]
pub enum Command {
    /// The raw command: `/<arguments>`.
    Raw(Vec<String>),
    /// The say command: `<message>`.
    Say(String),
    /// The save command: `/save <save name>`.
    Save(String),
    /// The quit command: `/quit`.
    Quit,
}

impl From<rpc::send_command_request::Command> for Command {
    fn from(comm: rpc::send_command_request::Command) -> Self {
        match comm {
            rpc::send_command_request::Command::Raw(raw_command) => Self::Raw(raw_command.arguments),
            rpc::send_command_request::Command::Save(save_command) => Self::Save(save_command.save_name),
            rpc::send_command_request::Command::Quit(_) => Self::Quit,
            rpc::send_command_request::Command::Say(say_command) => Self::Say(say_command.message),
        }
    }
}

impl Command {
    /// Returns the in-game command string for this command.
    pub fn get_command_string(&self) -> String {
        let command = match self {
            Self::Raw(arguments) => format!("/{}", arguments.join(" ")),
            Self::Say(message) => message.to_string(),
            Self::Save(save_name) => format!("/save {}", save_name),
            Self::Quit => "/quit".to_string(),
        };

        format!("{}\n", command)
    }
}
