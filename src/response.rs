use std::collections::HashMap;
use uuid::Uuid;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum Response {
    CreateGameResponse { game_id: Uuid, message: String },
    MakeMoveResponse { game_id: Uuid, message: String, board: HashMap<String, (char, Vec<String>)> },
    GetGamesResponse { game_ids: Vec<Uuid>},
    RequestFailedResponse { message: String, },
    JoinGameResponse { game_id: Uuid, message: String, },

    // // Add other responses here
    // GameLeft { game_id: Uuid, message: String },
    // ChatReceived { game_id: Uuid, player_name: String, message: String },
    // GameStarted { game_id: Uuid, message: String },
    // GameEnded { game_id: Uuid, message: String },
    // GameState { game_id: Uuid, state: String },
    // Pong,
    // Error { message: String },
}