mod deezer;
mod server;
mod spotify;

use std::time::Duration;

use dotenv::dotenv;
use loading::Loading;
use reqwest::Client;
use tokio::time::sleep;

use crate::deezer::Deezer;
use crate::server::Server;
use crate::spotify::Spotify;

#[async_trait::async_trait]
pub trait App {
    type Error;

    async fn init(&mut self);

    async fn fetch_token(&mut self) -> Result<(), Self::Error>;

    fn get_auth_url() -> String;
}

#[tokio::main]
async fn main() {
    dotenv().expect("Failed to load .env file");

    let reqwest_client = Client::new();
    let mut deezer = Deezer::new(&reqwest_client);
    let mut spotify = Spotify::new(&reqwest_client);

    Server::run().await;

    deezer.init().await;
    // spotify.init().await;

    let playlists = deezer.get_playlists().await.unwrap();

    println!("{:?}", playlists);
}
