use std::sync::{Arc, Mutex};
use tokio::io::{self, AsyncBufReadExt, BufReader};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use tokio::net::TcpStream;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use tokio::io::stdin;
use futures_util::{SinkExt, StreamExt};

use std::collections::HashMap;
use reqwest::Client as ReqwestClient;
use reqwest::Response as ReqwestResponse;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::error::Error;
use std::ops::Deref;
use futures_util::stream::{SplitSink, SplitStream};
use uuid::Uuid;
use http::StatusCode;
use surf;

use crate::request::Request;
use crate::response::Response;


pub struct Client {
    pub user_id: String,
    in_game: bool,
    active_games: Vec<Uuid>,
    awaiting_opponent_games: HashMap<i32, Uuid>,
    current_active_game_id: Option<Uuid>,
    api_url: String,
    ws_url: String,
    http_client: ReqwestClient,
    ws_write: Option<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>,
    ws_read: Option<SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>>,
}

impl Client {
    pub fn new(user_id: String) -> Self {
        Client {
            user_id,
            in_game: false,
            active_games: Vec::new(),
            awaiting_opponent_games: HashMap::new(),
            current_active_game_id: None,
            api_url: "http://127.0.0.1:8080".to_string(),
            ws_url: "ws://127.0.0.1:8081".to_string(),
            http_client: ReqwestClient::new(),
            ws_write: None,
            ws_read: None,
        }
    }

    pub async fn run_client(&mut self) {
        // Read messages from the terminal and send them to the server
        let stdin = BufReader::new(io::stdin());
        let mut lines = stdin.lines();

        let mut run = true;
        while run {
        // while let Ok(Some(line)) = lines.next_line().await {
            let line = lines.next_line().await.unwrap().unwrap();

            match line.as_str() {
                "exit" => {
                    run = false;
                },
                _ => {
                    let request = self.parse_command(line.clone().as_str());
                    match request {
                        None => (),
                        Some(req) => {
                            self.send_request(req).await;
                        }
                    }
                }
            }
        }
    }

    async fn send_request(&mut self, request: Request){
        match (request.clone(), &self.in_game, &self.ws_write, &self.ws_read) {
            (Request::GetGames, _, _, _) => {
                self.get_games(request).await;
            },

            (Request::CreateGame {user_id: _, color: _}, _, _, _) => {
                self.create_game(request).await;
            },

            (Request::JoinGame { game_id: _, user_id: _}, _, _, _) => {
                self.join_game(request).await;
            },

            (Request::MakeMove {game_id: _, user_id: _, from: _, to: _}, true, Some(_), Some(_)) =>
                self.make_move(request).await,

            _ => println!("Wrong request"),
        }
    }

    async fn get_games(&mut self, request: Request) {
        let http_response = self.http_client
            .get(self.api_url.clone() + "/get_games")
            .json(&request)
            .send()
            .await;

        let http_response = http_response.unwrap();
        match http_response.status() {
            StatusCode::OK => {
                let parsed_response = serde_json::from_str::<Response>(&http_response.text().await.unwrap()).unwrap();
                match &parsed_response {
                    Response::GetGamesResponse { game_ids } => {
                        self.awaiting_opponent_games = game_ids.into_iter()
                            .enumerate()
                            .map(|(index, uuid)| (index as i32, *uuid))
                            .collect();
                        println!("Available games: ");
                        for (index, uuid) in self.awaiting_opponent_games.clone() {
                            println!("{}: {}", index, uuid.to_string());
                        }
                        println!("\n\n");
                    },
                    _ => println!("Could not get games\n\n"),
                }

            },
            _ => println!("{}\n\n", http_response.status()),
        }
    }

    async fn join_game(&mut self, request: Request) {
        let http_response = self.http_client
            .put(self.api_url.clone() + "/join_game")
            .json(&request)
            .send()
            .await.unwrap();

        match http_response.status() {
            StatusCode::OK => {
                let parsed_response = serde_json::from_str::<Response>(&http_response.text().await.unwrap()).unwrap();
                match parsed_response {
                    Response::JoinGameResponse { game_id, message } => {
                        self.start_websocket_connection().await;
                        self.in_game = true;
                        self.current_active_game_id = Some(game_id);
                        self.active_games.push(game_id);
                        println!("Connected to the server");
                        println!("{}\n\n", message);
                    },
                    _ => println!("Could not join game"),
                }
            },
            _ => println!("Could not join game\n\n"),
        }
    }

    async fn create_game(&mut self, request: Request) {
        let Request::CreateGame { user_id, color: _} = request.clone() else { return };
        let http_response = self.http_client
            .post(self.api_url.clone() + "/create_game")
            .json(&request)
            .send()
            .await.unwrap();

        match (http_response.status(), serde_json::from_str::<Response>(&http_response.text().await.unwrap())) {
            (StatusCode::OK, Ok(resp)) => {
                match resp {
                    Response::CreateGameResponse { game_id, message} => {
                        println!("{}\n\n", message);
                        self.join_game(Request::JoinGame { game_id, user_id }).await;
                    },
                    _ => println!("Could not create game\n\n"),
                }
            },
            _ => println!("Could not create game\n\n"),
        }
    }

    async fn make_move(&mut self, request: Request) {
        match (request.clone(), serde_json::to_string(&request), self.ws_write.as_mut(), self.ws_read.as_mut()) {
            (Request::MakeMove {game_id, user_id, from, to}, Ok(message), Some(ws_write), Some(ws_read)) => {
                ws_write.send(Message::Text(message.clone())).await.expect("Failed to send message");

                // waiting for the server response
                let message = ws_read.next().await.unwrap();
                // while let Some(message) = ws_read.next().await {
                match message {
                    Ok(msg) => match msg {
                        Message::Text(text) => {
                            match serde_json::from_str::<Response>(text.as_str()) {
                                Ok(Response::MakeMoveResponse {
                                       game_id: server_game_id,
                                       message,
                                       board
                                   }) => println!("Received: {}", message),
                                Ok(Response::RequestFailedResponse {
                                       message
                                   }) => println!("Received: {}", message),
                                _ => println!("Invalid response"),
                            }
                        },
                        Message::Binary(bin) => println!("Received binary: {:?}", bin),
                        _ => (),
                    },
                    Err(e) => {
                        eprintln!("Error reading message: {:?}", e);
                        // break;
                    }
                }
                // }
            },
            (_, _, _, _) => println!("Could not make move"),
        }
    }

    async fn start_websocket_connection(&mut self) {
        let (ws_stream, _) = connect_async(&self.ws_url).await.expect("Failed to connect");

        println!("Connected to the server");

        let (write, read) = ws_stream.split();

        self.ws_write = Some(write);
        self.ws_read = Some(read);
    }
    async fn send_websocket_request(&mut self, request: Request) -> Response {
        match (request.clone(), serde_json::to_string(&request), self.ws_write.as_mut()) {
            (Request::MakeMove {game_id, user_id, from, to}, Ok(message), Some(ws_write)) => {
                ws_write.send(Message::Text(message.clone())).await.expect("Failed to send message");
                Response::MakeMoveResponse { game_id, message: message.clone(), board: HashMap::new() }
            },
            (_, _, _) => Response::RequestFailedResponse { message: "Invalid command".to_string() },
        }
    }

    fn parse_command(&self, input: &str) -> Option<Request> {
        let parts: Vec<&str> = input.split_whitespace().collect();

        match parts.as_slice() {
            ["get_games"] => Some(Request::GetGames),
            ["create_game", color] => Some(Request::CreateGame {
                user_id: self.user_id.clone(),
                color: color.to_string()
            }),
            ["join_game", game_index] => {
                let index = game_index.to_string().parse::<i32>().expect("");
                Some(Request::JoinGame {
                    game_id: *self.awaiting_opponent_games.clone().get(&index)?,
                    user_id: self.user_id.clone(),
                })
            },
            ["make_move", from, to] => {
                match self.current_active_game_id {
                    None => None,
                    Some(uuid) => Some(Request::MakeMove {
                        game_id: uuid,
                        user_id: self.user_id.clone(),
                        from: from.to_string(),
                        to: to.to_string(),
                    }),
                }
            }
            _ => None,
        }
    }
}