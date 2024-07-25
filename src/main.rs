mod request;
mod response;

use std::ops::Deref;
use std::sync::{Arc, Mutex};
use tokio::io::{self, AsyncBufReadExt, BufReader};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use tokio::fs::File;
use tokio::io::stdin;
use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use uuid::Uuid;

use crate::request::Request;
use crate::response::Response;


struct Client {
    game_id: Option<Uuid>,
    user_id: String,
}

impl Client {
    pub fn new() -> Self {
        Client {
            game_id: None,
            user_id: String::new(),
        }
    }

    pub fn parse_command(&self, input: &str) -> Option<Request> {
        let parts: Vec<&str> = input.split_whitespace().collect();

        match parts.as_slice() {
            ["create_game", color] => Some(Request::CreateGame {
                user_id: self.user_id.clone(),
                color: color.to_string()
            }),
            ["make_move", from, to] => {
                match self.game_id {
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

    pub fn handle_response(response: Option<Response>) {

    }
}





#[tokio::main]
async fn main() {
    // Replace with your WebSocket server URL
    let url = "ws://127.0.0.1:8080";
    let mut client = Arc::new(Mutex::new(Client::new()));
    println!("Type your user ID: ");

    let stdin = stdin();
    let reader = BufReader::new(stdin);
    let mut lines = reader.lines();

    if let Ok(Some(line)) = lines.next_line().await {
        client.lock().unwrap().user_id = line;
    }
    println!("Your user ID is: {}", client.lock().unwrap().user_id.clone());

    let (ws_stream, _) = connect_async(url).await.expect("Failed to connect");
    println!("Connected to the server");

    let (mut write, mut read) = ws_stream.split();

    // Spawn a task to read messages from the server
    let client_read = Arc::clone(&client);
    tokio::spawn(async move {
        while let Some(message) = read.next().await {
            match message {
                Ok(msg) => match msg {
                    Message::Text(text) => {
                        match serde_json::from_str::<Response>(text.as_str()) {
                            Ok(Response::GameCreated {
                                   game_id: server_game_id,
                                   message}) => {
                                println!("Received: {}", message);
                                match server_game_id {
                                    None => (),
                                    Some(uuid) => {
                                        client_read.lock().unwrap().game_id = Some(uuid);
                                    }
                                }
                            },
                            Ok(Response::MoveMade {
                                game_id: server_game_id,
                                message,
                                board
                            }) => println!("Received: {}", message),
                            Ok(Response::RequestFailed {
                                message
                            }) => println!("Received: {}", message),
                            Err(_) => println!("Invalid response"),
                        }
                    },
                    Message::Binary(bin) => println!("Received binary: {:?}", bin),
                    _ => (),
                },
                Err(e) => {
                    eprintln!("Error reading message: {:?}", e);
                    break;
                }
            }
        }
    });

    // Read messages from the terminal and send them to the server
    let stdin = BufReader::new(io::stdin());
    let mut lines = stdin.lines();

    while let Ok(Some(line)) = lines.next_line().await {
        let client_write = Arc::clone(&client);
        let request: Option<Request> = client_write.lock().unwrap().parse_command(line.clone().as_str());

        match request {
            None => (),
            Some(req) => {
                match serde_json::to_string(&req){
                    Ok(message) => write.send(Message::Text(message)).await.expect("Failed to send message"),
                    Err(_) => println!("Invalid command"),
                };


            }
        }
    }
}
