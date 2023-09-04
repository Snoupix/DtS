use std::sync::{Arc, OnceLock};

use reqwest::Client;
use tokio::sync::RwLock;

const REDIRECT_URI: &str = "http://localhost:8080/Deezer";
const PERMS: [&str; 2] = ["basic_access", "manage_library"];

pub static CODE: OnceLock<Arc<RwLock<String>>> = OnceLock::new();

#[derive(Debug, Default)]
pub struct Deezer {
    pub email: String,
    pub password: String,
    client: Client,
    access_token: String,
}

#[async_trait::async_trait]
impl crate::App for Deezer {
    type Error = String;

    async fn fetch_token(&mut self) -> Result<(), Self::Error> {
        std::thread::sleep(std::time::Duration::from_secs(5));
        Ok(())
    }

    fn get_auth_url() -> String {
        let id = dotenv::var("DEEZER_APP_ID")
            .map_err(|err| format!("Failed to get Deezer app ID from env {err}"))
            .unwrap();
        let perms = PERMS.join(",");

        format!(
            "https://connect.deezer.com/oauth/auth.php?app_id={}&redirect_uri={}&perms={}",
            id, REDIRECT_URI, perms
        )
    }
}

impl Deezer {
    pub fn get_playlists(&self) -> Result<Vec<serde_json::Value>, <Self as crate::App>::Error> {
        let mut playlists = Vec::new();

        Ok(playlists)
    }
}
