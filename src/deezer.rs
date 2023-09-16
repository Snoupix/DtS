use std::{
    sync::{Arc, OnceLock},
    time::Duration,
};

use loading::Loading;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::{sync::RwLock, time::sleep};

const TOKEN_URL: &str = "https://connect.deezer.com/oauth/access_token.php";
const REDIRECT_URI: &str = "http://localhost:8080/Deezer";
const PERMS: [&str; 2] = ["basic_access", "manage_library"];

pub static CODE: OnceLock<Arc<RwLock<String>>> = OnceLock::new();

#[derive(Debug)]
pub struct Deezer<'app> {
    pub email: String,
    pub password: String,
    client: &'app Client,
    access_token: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct DeezerPlaylistResponse {
    data: Vec<DeezerPlaylist>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DeezerPlaylist {
    pub title: String,
    pub tracks: Vec<DeezerTrack>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DeezerTrack {
    pub title: String,
    pub artist_name: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct DeezerUser {
    id: i64,
    name: String,
}

#[async_trait::async_trait]
impl<'app> crate::App for Deezer<'app> {
    type Error = String;

    async fn init(&mut self) {
        println!("{}", Deezer::get_auth_url());

        let deez_load = Loading::default();
        deez_load.text(String::from("Please sign in to Deezer with the link above"));

        let mut timeout = 0;
        while CODE.get().is_none() {
            if timeout == 150 {
                deez_load.fail(String::from("[5min timeout] Failed to login to Deezer"));
                std::process::exit(1);
            }

            timeout += 1;
            sleep(Duration::from_secs(2)).await;
        }

        match self.fetch_token().await {
            Ok(_) => deez_load.success(String::from("Logged in to Deezer!")),
            Err(err) => {
                deez_load.fail(format!("Failed to login Deezer! ({err})"));
                std::process::exit(1);
            }
        }

        deez_load.end();
    }

    async fn fetch_token(&mut self) -> Result<(), Self::Error> {
        let id = dotenv::var("DEEZER_APP_ID")
            .map_err(|err| format!("Failed to get Deezer app ID from env: {err}"))?;
        let secret = dotenv::var("DEEZER_SECRET_KEY")
            .map_err(|err| format!("Failed to get Deezer client SECRET from env: {err}"))?;

        let res = self
            .client
            .post(format!(
                "{}?app_id={}&secret={}&code={}&output=json",
                TOKEN_URL,
                id,
                secret,
                CODE.get().unwrap().read().await
            ))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .header("Content-Length", "0")
            .send()
            .await
            .map_err(|err| format!("Failed to send Deezer token request: {err}"))?;

        if !res.status().is_success() {
            return Err(format!(
                "Failed to fetch Deezer token: ({}) {:?}",
                res.status(),
                res.text().await
            ));
        }

        let body: serde_json::Value = res
            .json()
            .await
            .map_err(|err| format!("Failed to get Deezer token json result: {err}"))?;

        self.access_token = body["access_token"]
            .as_str()
            .ok_or_else(|| format!("Failed to get Deezer access token from json result: {body}"))?
            .to_owned();

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

impl<'app> Deezer<'app> {
    pub fn new(client: &'app Client) -> Self {
        Self {
            email: String::new(),
            password: String::new(),
            client,
            access_token: String::new(),
        }
    }

    async fn get_me(&self) -> Result<DeezerUser, <Deezer<'app> as crate::App>::Error> {
        let res = self
            .client
            .get(format!(
                "https://api.deezer.com/user/me?output=json&access_token={}",
                self.access_token
            ))
            .send()
            .await
            .map_err(|err| format!("Failed to send Deezer me request: {err}"))?;

        if !res.status().is_success() {
            return Err(format!(
                "Failed to fetch Deezer me: ({}) {:?}",
                res.status(),
                res.text().await
            ));
        }

        let body: serde_json::Value = res
            .json()
            .await
            .map_err(|err| format!("Failed to get Deezer me json result: {err}"))?;

        Ok(DeezerUser {
            id: body["id"].as_i64().unwrap(),
            name: body["name"].as_str().unwrap().to_owned(),
        })
    }

    async fn get_playlist_tracks(
        &self,
        id: i64,
    ) -> Result<Vec<DeezerTrack>, <Deezer<'app> as crate::App>::Error> {
        let res = self
            .client
            .get(format!(
                "https://api.deezer.com/playlist/{}/tracks?output=json&access_token={}",
                id, self.access_token
            ))
            .send()
            .await
            .map_err(|err| format!("Failed to send Deezer playlist tracks request: {err}"))?;

        if !res.status().is_success() {
            return Err(format!(
                "Failed to fetch Deezer playlist tracks: ({}) {:?}",
                res.status(),
                res.text().await
            ));
        }

        let body: serde_json::Value = res
            .json()
            .await
            .map_err(|err| format!("Failed to get Deezer playlist tracks json result: {err}"))?;

        let mut v = Vec::new();

        for track in body.get("data").unwrap_or(&json!([])).as_array().unwrap() {
            v.push(DeezerTrack {
                title: track["title"].as_str().unwrap().to_owned(),
                artist_name: track["artist"]["name"].as_str().unwrap().to_owned(),
            })
        }

        Ok(v)
    }

    pub async fn get_playlists(
        &self,
    ) -> Result<Vec<DeezerPlaylist>, <Deezer<'app> as crate::App>::Error> {
        let res = self
            .client
            .get(format!(
                "https://api.deezer.com/user/me/playlists?output=json&access_token={}",
                self.access_token
            ))
            .send()
            .await
            .map_err(|err| format!("Failed to send Deezer playlists request: {err}"))?;

        if !res.status().is_success() {
            return Err(format!(
                "Failed to fetch Deezer playlists: ({}) {:?}",
                res.status(),
                res.text().await
            ));
        }

        let body: serde_json::Value = res
            .json()
            .await
            .map_err(|err| format!("Failed to get Deezer playlists json result: {err}"))?;

        let owner = self.get_me().await.unwrap();

        let mut v = Vec::new();

        for playlist in body.get("data").unwrap_or(&json!([])).as_array().unwrap() {
            if playlist["type"].as_str().unwrap() != "playlist"
                || playlist["is_loved_track"].as_bool().unwrap()
                || playlist["creator"]["id"].as_i64().unwrap() != owner.id
            {
                continue;
            }

            v.push(DeezerPlaylist {
                title: playlist["title"].as_str().unwrap().to_owned(),
                tracks: self
                    .get_playlist_tracks(playlist["id"].as_i64().unwrap().to_owned())
                    .await
                    .unwrap(),
            })
        }

        Ok(v)
    }
}
