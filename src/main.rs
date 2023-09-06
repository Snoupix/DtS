mod deezer;
mod server;
mod spotify;

use dotenv::dotenv;
use loading::Loading;
use reqwest::Client;

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
    spotify.init().await;

    let loader = Loading::default();
    loader.text("Importing your Deezer playlists to Spotify...");

    let deez_playlists = deezer.get_playlists().await.unwrap();
    let new_playlists = spotify
        .get_tracks_from_deezer(deez_playlists)
        .await
        .unwrap();
    spotify.create_playlists(new_playlists).await.unwrap();

    loader.success("Your Deezer playlists are now imported to Spotify!");
    loader.end();
}
