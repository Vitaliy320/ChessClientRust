use std::collections::HashMap;
use uuid::Uuid;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum Response {
    GameCreated { game_id: Option<Uuid>, message: String },
    MoveMade { game_id: Uuid, message: String, board: HashMap<String, (char, Vec<String>)> },
    RequestFailed { message: String, },
    // // Add other responses here
    // GameJoined { game_id: Uuid, message: String },
    // GameLeft { game_id: Uuid, message: String },
    // ChatReceived { game_id: Uuid, player_name: String, message: String },
    // GameStarted { game_id: Uuid, message: String },
    // GameEnded { game_id: Uuid, message: String },
    // GameState { game_id: Uuid, state: String },
    // GamesList { games: Vec<Uuid> },
    // Pong,
    // Error { message: String },
}