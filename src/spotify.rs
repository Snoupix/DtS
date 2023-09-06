use std::{
    sync::{Arc, OnceLock},
    time::Duration,
};

use base64::{engine::general_purpose, Engine as _};
use loading::Loading;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::{sync::RwLock, time::sleep};

use crate::deezer::DeezerPlaylist;

const TOKEN_URL: &str = "https://accounts.spotify.com/api/token";
const SCOPES: [&str; 4] = [
    "playlist-modify-private",
    "playlist-modify-public",
    "user-read-email",
    "user-read-private",
];
const REDIRECT_URI: &str = "http://localhost:8080/Spotify";

pub static CODE: OnceLock<Arc<RwLock<String>>> = OnceLock::new();

#[derive(Debug)]
pub struct Spotify<'app> {
    pub email: String,
    pub password: String,
    client: &'app Client,
    access_token: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SpotifyTrack {
    id: String,
    title: String,
    artist_name: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SpotifyPlaylist {
    pub title: String,
    pub tracks: Vec<SpotifyTrack>,
}

#[async_trait::async_trait]
impl<'app> crate::App for Spotify<'app> {
    type Error = String;

    async fn init(&mut self) {
        println!("{}", Spotify::get_auth_url());

        let spot_load = Loading::default();
        spot_load.text(String::from(
            "Please sign in to Spotify with the link above",
        ));

        let mut timeout = 0;
        while CODE.get().is_none() {
            if timeout == 12 {
                spot_load.fail(String::from("[2min timeout] Failed to login to Spotify"));
                std::process::exit(1);
            }

            timeout += 1;
            sleep(Duration::from_secs(10)).await;
        }

        match self.fetch_token().await {
            Ok(_) => spot_load.success(String::from("Logged in to Spotify!")),
            Err(err) => {
                spot_load.fail(format!("Failed to login to Spotify ({err})"));
                std::process::exit(1);
            }
        }

        spot_load.end();
    }

    // Keep in mind that, if you wanna use this, you need to handle the refresh token (every 60 minutes, the access token expires)
    async fn fetch_token(&mut self) -> Result<(), Self::Error> {
        let id = dotenv::var("SPOTIFY_CLIENT_ID")
            .map_err(|err| format!("Failed to get Spotify client ID from env: {err}"))?;
        let secret = dotenv::var("SPOTIFY_CLIENT_SECRET")
            .map_err(|err| format!("Failed to get Spotify client SECRET from env: {err}"))?;

        let res = self
            .client
            .post(format!(
                "{}?grant_type=client_credentials&code={}&client_id={}&client_secret={}",
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
            .header("Content-Length", "0")
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

impl<'app> Spotify<'app> {
    pub fn new(client: &'app Client) -> Self {
        Self {
            email: String::new(),
            password: String::new(),
            client,
            access_token: String::new(),
        }
    }

    pub async fn get_tracks_from_deezer(
        &self,
        playlist: Vec<DeezerPlaylist>,
    ) -> Result<Vec<SpotifyPlaylist>, <Spotify<'app> as crate::App>::Error> {
        let mut p = Vec::new();

        for playlist in playlist {
            let mut curr_playlist = SpotifyPlaylist {
                title: playlist.title.clone(),
                tracks: Vec::new(),
            };

            for track in playlist.tracks {
                let res = self
                    .client
                    .get(format!(
                        "https://api.spotify.com/v1/search?q={}%20artist:{}&type=track&limit=1",
                        track.title,
                        track.artist_name.replace(' ', "%20")
                    ))
                    .header("Authorization", format!("Bearer {}", self.access_token))
                    .send()
                    .await
                    .map_err(|err| format!("Failed to send Spotify search request: {err}"))?;

                if !res.status().is_success() {
                    return Err(format!(
                        "Failed to fetch Spotify search: ({}) {:?}",
                        res.status(),
                        res.text().await
                    ));
                }

                let body: serde_json::Value = res
                    .json()
                    .await
                    .map_err(|err| format!("Failed to get Spotify search json result: {err}"))?;

                let items_found = body["tracks"]["items"].as_array();

                if items_found.is_none() {
                    println!(
                        "Track not found on Spotify: {} - {} for playlist: {}",
                        track.title, track.artist_name, playlist.title
                    );
                    println!("DEBUG: {:#?}", body);
                    continue;
                }

                for item in items_found.unwrap() {
                    if item["type"].as_str().is_some_and(|t| t == "track") {
                        curr_playlist.tracks.push(SpotifyTrack {
                            id: item["id"].as_str().unwrap().to_owned(),
                            title: item["name"].as_str().unwrap().to_owned(),
                            artist_name: item["artists"][0]["name"].as_str().unwrap().to_owned(),
                        });
                    }
                }
            }
            p.push(curr_playlist);
        }

        Ok(p)
    }

    async fn get_my_id(&self) -> Result<String, <Spotify<'app> as crate::App>::Error> {
        let res = self
            .client
            .get("https://api.spotify.com/v1/me")
            .header("Authorization", format!("Bearer {}", self.access_token))
            .send()
            .await
            .map_err(|err| format!("Failed to send Spotify user info request: {err}"))?;

        if !res.status().is_success() {
            return Err(format!(
                "Failed to fetch Spotify user info: ({}) {:?}",
                res.status(),
                res.text().await
            ));
        }

        let body: serde_json::Value = res
            .json()
            .await
            .map_err(|err| format!("Failed to get Spotify user info json result: {err}"))?;

        Ok(body["id"].as_str().unwrap().to_owned())
    }

    pub async fn create_playlists(
        &self,
        playlists: Vec<SpotifyPlaylist>,
    ) -> Result<(), <Spotify<'app> as crate::App>::Error> {
        let id = self.get_my_id().await?;

        for playlist in playlists {
            let res = self
                .client
                .post(format!("https://api.spotify.com/v1/users/{id}/playlists"))
                .header("Authorization", format!("Bearer {}", self.access_token))
                .send()
                .await
                .map_err(|err| {
                    format!("Couldn't send Spotify post resquest to create playlist {err}")
                })?;

            if !res.status().is_success() {
                return Err(format!(
                    "Failed to create Spotify playlist: ({}) {:?}",
                    res.status(),
                    res.text().await
                ));
            }

            let body: serde_json::Value = res
                .json()
                .await
                .map_err(|err| format!("Failed to get Spotify playlist json result: {err}"))?;

            let playlist_id = body["id"].as_str().unwrap().to_owned();

            let uris = playlist
                .tracks
                .iter()
                .map(|t| format!("spotify:track:{}", t.id))
                .collect::<Vec<String>>();

            let res = self
                .client
                .post(format!(
                    "https://api.spotify.com/v1/playlists/{playlist_id}/tracks",
                ))
                .header("Authorization", format!("Bearer {}", self.access_token))
                .json(&json!({ "uris": uris }))
                .send()
                .await
                .map_err(|err| {
                    format!(
                        "Couldn't send Spotify post resquest to add tracks to playlist id: {} {}",
                        playlist_id, err
                    )
                })?;

            if !res.status().is_success() {
                return Err(format!(
                    "Failed to add tracks to Spotify playlist id: {} ({}) {:?}",
                    playlist_id,
                    res.status(),
                    res.text().await
                ));
            }
        }
        Ok(())
    }
}
