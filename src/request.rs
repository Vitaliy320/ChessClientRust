use uuid::Uuid;
use serde::{Deserialize, Serialize};
use tokio::io::Join;
use crate::response::{GetGamesResponse, JoinGameResponse, MakeMoveResponse};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CreateGameRequest {
    pub user_id: String,
    pub color: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AuthorizeWebsocketConnectionRequest {
    pub game_id: Uuid,
    pub user_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MakeMoveRequest {
    pub game_id: Uuid,
    pub user_id: String,
    pub from: String,
    pub to: String,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GetGamesRequest {}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JoinGameRequest {
    pub game_id: Uuid,
    pub user_id: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub enum Request {
    CreateGameRequest(CreateGameRequest),
    MakeMoveRequest(MakeMoveRequest),
    AuthorizeWebsocketConnectionRequest(AuthorizeWebsocketConnectionRequest),
    GetGamesRequest(GetGamesRequest),
    JoinGameRequest(JoinGameRequest),
}