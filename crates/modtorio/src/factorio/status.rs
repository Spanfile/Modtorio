use super::executable::GameState;
use rpc::instance_status::game::GameStatus;

#[derive(Debug)]
pub struct ServerStatus {
    game_status: GameStatus,
    game_state: GameState,
}

impl Default for ServerStatus {
    fn default() -> Self {
        Self {
            game_status: GameStatus::Shutdown,
            game_state: GameState::Initialising,
        }
    }
}

impl ServerStatus {
    pub fn game_status(&self) -> GameStatus {
        self.game_status
    }

    pub fn set_game_status(&mut self, status: GameStatus) {
        self.game_status = status
    }
}
