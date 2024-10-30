mod request;
mod response;
mod client;
mod ws_client;

use std::ops::Deref;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::io::stdin;
use futures_util::{SinkExt, StreamExt};
use serde::Serialize;

use crate::client::Client;
use crate::ws_client::run_client;



#[tokio::main]
async fn main() {
    let stdin = stdin();
    let reader = BufReader::new(stdin);
    let mut lines = reader.lines();

    println!("Type your user ID: ");
    let user_id = lines.next_line().await.expect("").unwrap();

    let mut client = Client::new(user_id);

    println!("Your user ID is: {}", client.user_id.clone());

    client.run_client().await;
    // run_client().await;
}
