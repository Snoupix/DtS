use std::sync::Arc;

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::RwLock,
};

use crate::deezer::CODE as DEEZER_CODE;
use crate::spotify::CODE as SPOTIFY_CODE;

const SOCKET: &str = "127.0.0.1:8080";

pub struct Server;

impl Server {
    pub async fn run(&self) {
        let listener = TcpListener::bind(SOCKET).await.unwrap();
        let s = Arc::new(RwLock::new(false));
        let d = Arc::new(RwLock::new(false));

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

                            let mut s = s.write().await;
                            *s = true;

                            break;
                        }

                        if line.starts_with("GET /Deezer") {
                            let code = line
                                .trim_start_matches("GET /Deezer?code=")
                                .trim_end_matches(" HTTP/1.1")
                                .to_string();

                            DEEZER_CODE.get_or_init(|| Arc::new(RwLock::new(code)));

                            let mut d = d.write().await;
                            *d = true;

                            let response = "HTTP/1.1 200 OK\r\nContent-Length: 45\r\n\r\nYou're connected to Spotify, you can close this tab now!";
                            if let Err(err) = stream.write_all(response.as_bytes()).await {
                                eprintln!("Error writing response: {}", err);
                            }

                            break;
                        }
                    }

                    let response = "HTTP/1.1 200 OK\r\nContent-Length: 45\r\n\r\nYou're connected, you can close this tab now!";
                    if let Err(err) = stream.write_all(response.as_bytes()).await {
                        eprintln!("Error writing response: {}", err);
                    }
                }

                // Both apps are connected, closing server
                if *s.read().await && *d.read().await {
                    break;
                }
            }
        });
    }
}
