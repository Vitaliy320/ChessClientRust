use std::sync::{Arc, Mutex};
use tokio::io::{self, AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use tokio::net::TcpStream;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use tokio::io::stdin;
use futures_util::{future, pin_mut, SinkExt, StreamExt};

use std::collections::HashMap;
use reqwest::Client as ReqwestClient;
use reqwest::Response as ReqwestResponse;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::error::Error;
use std::fmt::format;
use std::ops::Deref;
use futures_util::stream::{SplitSink, SplitStream};
use uuid::Uuid;
use http::StatusCode;
use surf;

use crate::request::{
    Request,
    CreateGameRequest,
    AuthorizeWebsocketConnectionRequest,
    MakeMoveRequest,
    GetGamesRequest,
    JoinGameRequest,
};

use crate::response::{
    CreateGameResponse,
    AuthorizeWebsocketConnectionResponse,
    MakeMoveResponse,
    GetGamesResponse,
    JoinGameResponse,
    RequestFailedResponse
};


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
            api_url: "http://localhost:8080".to_string(),
            ws_url: "ws://localhost:8081".to_string(),
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

    async fn send_request(&mut self, request: Request) {
        match (request.clone(), &self.in_game, &self.ws_write, &self.ws_read) {
            (Request::GetGamesRequest(GetGamesRequest {}), _, _, _) => {
                self.get_games(GetGamesRequest {}).await;
            },

            (Request::CreateGameRequest(CreateGameRequest{user_id, color}), _, _, _) => {
                self.create_game(CreateGameRequest{user_id, color}).await;
            },

            (Request::JoinGameRequest(JoinGameRequest{ game_id, user_id}), _, _, _) => {
                self.join_game(JoinGameRequest{game_id, user_id}).await;
            },

            (Request::AuthorizeWebsocketConnectionRequest (AuthorizeWebsocketConnectionRequest{
                game_id, user_id }), true, Some(_), _) => {
                let _ = self.authorize_game(AuthorizeWebsocketConnectionRequest{ game_id, user_id }).await;
            },

            (Request::MakeMoveRequest (MakeMoveRequest{game_id, user_id, from, to}), true, Some(_), _) =>
                self.make_move(MakeMoveRequest{game_id, user_id, from, to}).await,

            _ => println!("Wrong request"),
        }
    }

    async fn get_games(&mut self, request: GetGamesRequest) {
        let http_response = self.http_client
            .get(self.api_url.clone() + "/get_games")
            .json(&request)
            .send()
            .await;

        let http_response = http_response.unwrap();
        match http_response.status() {
            StatusCode::OK => {
                let parsed_response = serde_json::from_str::<GetGamesResponse>(&http_response.text().await.unwrap()).unwrap();
                let GetGamesResponse { game_ids } = parsed_response;

                self.awaiting_opponent_games = game_ids.into_iter()
                    .enumerate()
                    .map(|(index, uuid)| (index as i32, uuid))
                    .collect();

                println!("Available games: ");
                for (index, uuid) in self.awaiting_opponent_games.clone() {
                    println!("{}: {}", index, uuid.to_string());
                }
                println!("\n\n");
            },
            _ => println!("{}\n\n", http_response.status()),
        }
    }

    async fn join_game(&mut self, request: JoinGameRequest) {
        let http_response = self.http_client
            .put(self.api_url.clone() + "/join_game")
            .json(&request)
            .send()
            .await;

        let http_response = http_response.unwrap();
        match http_response.status() {
            StatusCode::OK => {
                let a = http_response.text().await.unwrap();
                let parsed_response = serde_json::from_str::<CreateGameResponse>(&a).unwrap();
                let CreateGameResponse { game_id, message } = parsed_response;
                self.start_websocket_connection().await;

                let res = self.authorize_game(AuthorizeWebsocketConnectionRequest {
                    game_id,
                    user_id: self.user_id.clone(),
                }).await;

                match res {
                    Ok(message) => {
                        self.in_game = true;
                        self.current_active_game_id = Some(game_id);
                        self.active_games.push(game_id);

                        println!("Connected to the server");
                        println!("{}\n\n", message);
                    },
                    Err(message) => println!("{}", message),
                }
            },
            _ => println!("Could not join game\n\n"),
        }
    }

    async fn create_game(&mut self, request: CreateGameRequest) {
        let CreateGameRequest { user_id, color: _} = request.clone();
        let http_response = self.http_client
            .post(self.api_url.clone() + "/create_game")
            .json(&request)
            .send()
            .await;
        let http_response = http_response.unwrap();

        match (http_response.status(), serde_json::from_str::<CreateGameResponse>(&http_response.text().await.unwrap().as_str())) {
            (StatusCode::OK, Ok(resp)) => {
                let CreateGameResponse {game_id, message} = resp;
                println!("{}\n\n", message);
                self.join_game(JoinGameRequest { game_id, user_id }).await;
            },
            _ => println!("Could not create game\n\n"),
        }
    }

    fn board_dict_to_string(&self, columns: String, rows: String, board: HashMap<String, (String, Vec<String>)>) -> String {
        let board_string: String = rows.chars()
            .rev()
            .enumerate()
            .map(|(row_index, row)| {
                let mut row_string = format!("{} ", 8 - row_index);
                row_string.push_str(&columns.chars()
                    .map(|col| {
                        let coordinates = format!("{}{}", col, row);
                        match board.get(&coordinates) {
                            Some((piece, possible_moves)) => format!("{} ", piece.to_string()),
                            None => "  ".to_string(),
                        }
                    })
                    .collect::<String>()
                );
                row_string + "\n"
            })
            .collect();

        format!("{}{}", board_string, format!("  {}", columns.chars()
            .map(|column| {
                format!("{} ", column.to_uppercase())
            }).collect::<String>()
        ))
    }

    async fn authorize_game(&mut self, request: AuthorizeWebsocketConnectionRequest) -> Result<String, String> {
        if let (
            Ok(message),
            Some(ws_write),
            _,
        ) = (
            serde_json::to_string(&request),
            self.ws_write.as_mut(),
            self.ws_read.as_mut(),
        ) {
            ws_write.send(Message::Text(message.clone())).await.expect("Failed to send message");
            Ok("".to_string())
            // let message = ws_read.next().await.unwrap();
            // match message {
            //     Ok(msg) => match msg {
            //         Message::Text(text) => {
            //             match serde_json::from_str::<AuthorizeWebsocketConnectionResponse>(text.as_str()) {
            //                 Ok(AuthorizeWebsocketConnectionResponse {
            //                     message,
            //                 }) => Ok(message),
            //                 _ => Err("Could not authorize".to_string()),
            //             }
            //         },
            //         _ => Err("Could not authorize".to_string()),
            //     }
            //     Err(e) => Err("Could not authorize".to_string()),
            // }
        } else {
            Err("Could not authorize".to_string())
        }
    }

    async fn make_move(&mut self, request: MakeMoveRequest) {
        if let (
            Ok(message),
            Some(ws_write),
            _
            // Some(ws_read),
        ) = (
            serde_json::to_string(&request),
            self.ws_write.as_mut(),
            self.ws_read.as_mut(),
        ) {
            ws_write.send(Message::Text(message.clone())).await.expect("Failed to send message");


            // waiting for the server response
            // let message = ws_read.next().await.unwrap();
            // match message {
            //     Ok(msg) => match msg {
            //         Message::Text(text) => {
            //             match serde_json::from_str::<MakeMoveResponse>(text.as_str()) {
            //                 Ok(MakeMoveResponse {
            //                        game_id: server_game_id,
            //                        message,
            //                        columns,
            //                        rows,
            //                        board
            //                    }) => {
            //                     let board_string = self.board_dict_to_string(columns, rows, board);
            //                     println!("Received: {}\n{}", message, board_string);
            //
            //                 },
            //                 _ => println!("Invalid response"),
            //             }
            //         },
            //         Message::Binary(bin) => println!("Received binary: {:?}", bin),
            //         _ => (),
            //     },
            //     Err(e) => {
            //         eprintln!("Error reading message: {:?}", e);
            //         // break;
            //     }
            // }
        }
    }


    async fn start_websocket_connection_new(&mut self) {
        // let (stdin_tx, stdin_rx) = futures_channel::mpsc::unbounded();
        // tokio::spawn(self.read_stdin(stdin_tx));
        //
        // let (ws_stream, _) = connect_async(&self.ws_url).await.expect("Failed to connect");
        // println!("WebSocket handshake has been successfully completed");
        //
        // let (write, read) = ws_stream.split();
        //
        // let stdin_to_ws = stdin_rx.map(Ok).forward(write);
        // let ws_to_stdout = {
        //     read.for_each(|message| async {
        //         let data = message.unwrap().into_data();
        //         tokio::io::stdout().write_all(&data).await.unwrap();
        //     })
        // };
        //
        // pin_mut!(stdin_to_ws, ws_to_stdout);
        // future::select(stdin_to_ws, ws_to_stdout).await;
    }

    async fn read_stdin(&self, tx: futures_channel::mpsc::UnboundedSender<Message>) {
        let mut stdin = tokio::io::stdin();
        loop {
            let mut buf = vec![0; 1024];
            let n = match stdin.read(&mut buf).await {
                Err(_) | Ok(0) => break,
                Ok(n) => n,
            };
            buf.truncate(n);
            tx.unbounded_send(Message::binary(buf)).unwrap();
        }
    }

    async fn start_websocket_connection(&mut self) {
        let (ws_stream, _) = connect_async(&self.ws_url).await.expect("Failed to connect");

        println!("Connected to the server");

        let (write, mut read) = ws_stream.split();

        self.ws_write = Some(write);

        // self.ws_read = Some(read);
        let mut read_clone = read.next();
        tokio::spawn(async move {
            while let Some(message) = read.next().await {
                match message {
                    Ok(msg) =>
                        match msg {
                            Message::Text(text) => {
                                println!("Received text message: {}", text);
                            },
                            _ => (),
                        }
                    Err(e) => println!("Error reading message: {}", e),
                }
            }
        });
    }

    fn parse_command(&self, input: &str) -> Option<Request> {
        let parts: Vec<&str> = input.split_whitespace().collect();

        match parts.as_slice() {
            ["get_games"] => Some(Request::GetGamesRequest(GetGamesRequest {})),
            ["create_game", color] => Some(Request::CreateGameRequest(CreateGameRequest {
                user_id: self.user_id.clone(),
                color: color.to_string()
            })),
            ["join_game", game_index] => {
                let index = game_index.to_string().parse::<i32>().expect("");
                Some(Request::JoinGameRequest(JoinGameRequest {
                    game_id: *self.awaiting_opponent_games.clone().get(&index)?,
                    user_id: self.user_id.clone(),
                }))
            },
            ["make_move", from, to] => {
                match self.current_active_game_id {
                    None => None,
                    Some(uuid) => Some(Request::MakeMoveRequest(MakeMoveRequest {
                        game_id: uuid,
                        user_id: self.user_id.clone(),
                        from: from.to_string(),
                        to: to.to_string(),
                    })),
                }
            }
            _ => None,
        }
    }
}