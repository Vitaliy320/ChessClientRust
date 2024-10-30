use std::collections::HashMap;
use uuid::Uuid;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateGameResponse {
    pub game_id: Uuid,
    pub message: String,
}


#[derive(Serialize, Deserialize, Debug)]
pub struct AuthorizeWebsocketConnectionResponse {
    pub message: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MakeMoveResponse {
    pub game_id: Uuid,
    pub message: String,
    pub columns: String,
    pub rows: String,
    pub board: HashMap<String, (String, Vec<String>)>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GetGamesResponse {
    pub game_ids: Vec<Uuid>,
}


#[derive(Serialize, Deserialize, Debug)]
pub struct JoinGameResponse {
    pub game_id: Uuid,
    pub message: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RequestFailedResponse {
    pub message: String,
}