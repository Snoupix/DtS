use std::sync::Arc;

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::{Mutex, RwLock},
};

use crate::deezer::CODE as DEEZER_CODE;
use crate::spotify::CODE as SPOTIFY_CODE;

const SOCKET: &str = "127.0.0.1:8080";

pub struct Server;

impl Server {
    pub async fn run() {
        let listener = TcpListener::bind(SOCKET).await.unwrap();
        let s = Arc::new(Mutex::new(false));
        let d = Arc::new(Mutex::new(false));

        tokio::spawn(async move {
            let s = Arc::clone(&s);
            let d = Arc::clone(&d);

            while let Ok((mut stream, _)) = listener.accept().await {
                let mut buffer = [0; 1024];

                if let Ok(bytes_read) = stream.read(&mut buffer).await {
                    if bytes_read == 0 {
                        continue;
                    }

                    let request = String::from_utf8_lossy(&buffer[..bytes_read]);

                    for line in request.to_string().lines() {
                        if line.starts_with("GET /Spotify") {
                            let code = line
                                .trim_start_matches("GET /Spotify?code=")
                                .trim_end_matches(" HTTP/1.1")
                                .to_string();

                            SPOTIFY_CODE.get_or_init(|| Arc::new(RwLock::new(code)));

                            let mut s = s.lock().await;
                            *s = true;

                            break;
                        }

                        if line.starts_with("GET /Deezer") {
                            let code = line
                                .trim_start_matches("GET /Deezer?code=")
                                .trim_end_matches(" HTTP/1.1")
                                .to_string();

                            DEEZER_CODE.get_or_init(|| Arc::new(RwLock::new(code)));

                            let mut d = d.lock().await;
                            *d = true;

                            break;
                        }
                    }

                    let response = "HTTP/1.1 200 OK\r\nContent-Length: 45\r\n\r\nYou're connected, you can close this tab now!";
                    if let Err(err) = stream.write_all(response.as_bytes()).await {
                        eprintln!("Error writing response: {}", err);
                    }
                }

                // Both apps are connected, closing server
                if *s.lock().await && *d.lock().await {
                    break;
                }
            }
        });
    }
}
