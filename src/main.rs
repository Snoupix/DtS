mod deezer;
mod server;
mod spotify;

use dotenv::dotenv;
use loading::Loading;

use crate::deezer::Deezer;
use crate::server::Server;
use crate::spotify::{Spotify, CODE as SPOTIFY_CODE};

#[async_trait::async_trait]
pub trait App {
    type Error;

    // fn get_credentials(&self, name: &str) -> (String, String) {
    //     let mut e = String::new();
    //
    //     println!("Please enter your {name} login email: ");
    //     if let Err(err) = stdin().read_line(&mut e) {
    //         eprintln!("Failed to read line for {name}: {err}");
    //         std::process::exit(1);
    //     }
    //     e = e.trim_end().to_string();
    //
    //     println!("Please enter your {name} login password: ");
    //     let p = try_scanpw(Some('*'))
    //         .map_err(|err| {
    //             eprintln!("Failed to read password for {name}: {err}");
    //             std::process::exit(1);
    //         })
    //         .unwrap();
    //
    //     (e, p)
    // }

    async fn fetch_token(&mut self) -> Result<(), Self::Error>;

    fn get_auth_url() -> String;
}

#[tokio::main]
async fn main() {
    dotenv().expect("Failed to load .env file");

    let server = Server;
    let mut deezer = Deezer::default();
    let mut spotify = Spotify::default();

    server.run().await;

    // (deezer.email, deezer.password) = deezer.get_credentials("Deezer");

    // let deez_load = Loading::default();
    //
    // deez_load.text(format!(
    //     "Please sign in to Deezer here: {}",
    //     Deezer::get_auth_url()
    // ));
    //
    // match deezer.login().await {
    //     Ok(_) => deez_load.success(String::from("Logged in to Deezer!")),
    //     Err(err) => {
    //         deez_load.fail(format!("Failed to login Deezer! ({err})"));
    //         std::process::exit(1);
    //     }
    // }
    //
    // deez_load.end();

    // (spotify.email, spotify.password) = spotify.get_credentials("Spotify");

    let spot_load = Loading::default();

    println!("{}", Spotify::get_auth_url());
    spot_load.text(String::from(
        "Please sign in to Spotify with the link above",
    ));

    let mut timeout = 0;
    while SPOTIFY_CODE.get().is_none() {
        if timeout == 12 {
            spot_load.fail(String::from("[2min timeout] Failed to login to Spotify"));
            std::process::exit(1);
        }

        timeout += 1;
        tokio::time::sleep(std::time::Duration::from_secs(10)).await;
    }

    match spotify.fetch_token().await {
        Ok(_) => spot_load.success(String::from("Logged in to Spotify!")),
        Err(err) => {
            spot_load.fail(format!("Failed to login to Spotify ({err})"));
            std::process::exit(1);
        }
    }

    spot_load.end();

    println!("{:?} {:?}", deezer, spotify);
}
