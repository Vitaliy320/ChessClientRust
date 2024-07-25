use uuid::Uuid;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum Request {
    CreateGame { user_id: String, color: String, },
    MakeMove { game_id: Uuid, user_id: String, from: String, to: String, },
    // Add other requests here
    // JoinGame { game_id: Uuid, player_name: String },
    // LeaveGame { game_id: Uuid, player_name: String },
    // ChatMessage { game_id: Uuid, player_name: String, message: String },
    // StartGame { game_id: Uuid },
    // EndGame { game_id: Uuid },
    // GetGameState { game_id: Uuid },
    // ListGames,
    // Ping,
}