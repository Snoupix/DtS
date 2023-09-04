use std::sync::{Arc, OnceLock};

use base64::{engine::general_purpose, Engine as _};
use reqwest::Client;
use tokio::sync::RwLock;

const TOKEN_URL: &str = "https://accounts.spotify.com/api/token";
const SCOPES: [&str; 2] = ["playlist-modify-private", "playlist-modify-public"];
const REDIRECT_URI: &str = "http://localhost:8080/Spotify";

pub static CODE: OnceLock<Arc<RwLock<String>>> = OnceLock::new();

#[derive(Debug, Default)]
pub struct Spotify {
    pub email: String,
    pub password: String,
    client: Client,
    access_token: String,
}

#[async_trait::async_trait]
impl crate::App for Spotify {
    type Error = String;

    // Keep in mind that, if you wanna use this, you need to handle the refresh token (every 60 minutes, the access token expires)
    async fn fetch_token(&mut self) -> Result<(), Self::Error> {
        let id = dotenv::var("SPOTIFY_CLIENT_ID")
            .map_err(|err| format!("Failed to get Spotify client ID from env: {err}"))?;
        let secret = dotenv::var("SPOTIFY_CLIENT_SECRET")
            .map_err(|err| format!("Failed to get Spotify client SECRET from env: {err}"))?;

        let res = self
            .client
            .post(format!(
                "{}/grant_type=authorization_code&code={}&client_id={}&client_secret={}",
                TOKEN_URL,
                CODE.get().unwrap().read().await,
                id,
                secret
            ))
            .header(
                "Authorization",
                format!(
                    "Basic {}",
                    general_purpose::STANDARD.encode(format!("{}:{}", id, secret))
                ),
            )
            .header("Content-Type", "application/x-www-form-urlencoded")
            .send()
            .await
            .map_err(|err| format!("Failed to send Spotify token request: {err}"))?;

        if !res.status().is_success() {
            return Err(format!(
                "Failed to fetch Spotify token: ({}) {:?}",
                res.status(),
                res.text().await
            ));
        }

        let body: serde_json::Value = res
            .json()
            .await
            .map_err(|err| format!("Failed to get Spotify token json result: {err}"))?;

        self.access_token = body["access_token"]
            .as_str()
            .ok_or_else(|| format!("Failed to get Spotify access token from json result: {body}"))?
            .to_owned();

        Ok(())
    }

    fn get_auth_url() -> String {
        let id = dotenv::var("SPOTIFY_CLIENT_ID")
            .map_err(|err| format!("Failed to get Spotify client ID from env {err}"))
            .unwrap();
        let scopes = SCOPES.join("%20");

        format!(
            "https://accounts.spotify.com/authorize?client_id={}&response_type=code&show_dialog=true&redirect_uri={}&scope={}",
            id, REDIRECT_URI, scopes
        )
    }
}
