use std::env;
use std::fs::File;
use std::io::Read;
use std::process::exit;

use serde::Deserialize;

use librespot::core::{
    config::SessionConfig, 
    session::Session,
    spotify_id::SpotifyId,
    SpotifyUri,
};
use librespot::playback::{
    audio_backend,
    config::{AudioFormat, PlayerConfig},
    mixer::NoOpVolume,
    player::Player,
};

#[derive(Deserialize, Debug)]
struct SpotifyCredentialsCache {
    username: String,
    auth_type: u32,
    auth_data: String,
}

// URL-safe base64 decoder to turn the JSON string back into raw key bytes
fn decode_url_safe_base64(input: &str) -> Option<Vec<u8>> {
    let mut output = Vec::new();
    let mut buffer = 0u32;
    let mut bits = 0;
    
    for byte in input.bytes() {
        let val = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'-' => 62,
            b'_' => 63,
            b'=' | b'\r' | b'\n' | b' ' => continue,
            _ => return None,
        };
        buffer = (buffer << 6) | val as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            output.push((buffer >> bits) as u8);
        }
    }
    Some(output)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <PATH_TO_CREDENTIALS_JSON> <TRACK_ID>", args[0]);
        std::process::exit(1);
    }

    let cred_path = &args[1];
    let track_id_str = &args[2];

    let mut file = File::open(cred_path)
        .expect("Failed to open credentials cache file");
    let mut file_contents = String::new();
    file.read_to_string(&mut file_contents).expect("Failed to read credentials file");

    let cache: SpotifyCredentialsCache = serde_json::from_str(&file_contents)
        .expect("Failed to parse credentials JSON");

    if cache.auth_type != 1 {
        eprintln!("Error: Expected auth_type 1, found {}", cache.auth_type);
        exit(1);
    }

    // Decode the string token back into raw encrypted bytes
    let decoded_auth_bytes = decode_url_safe_base64(&cache.auth_data)
        .expect("Failed to decode auth_data blob from base64");

    // Instantiate the Credentials struct directly matching the compiler's blueprint
    let credentials = librespot::core::authentication::Credentials {
        username: Some(cache.username),
        auth_data: decoded_auth_bytes,
        auth_type: librespot_protocol::authentication::AuthenticationType::AUTHENTICATION_STORED_SPOTIFY_CREDENTIALS,
    };

    let session_config = SessionConfig::default();
    let player_config = PlayerConfig::default();
    let audio_format = AudioFormat::default();

    let track = SpotifyUri::Track {
        id: SpotifyId::from_base62(track_id_str).expect("Invalid Track ID"),
    };

    let backend = audio_backend::find(None).expect("No audio backend found");

    eprintln!("Connecting to Spotify...");
    let session = Session::new(session_config, None);
    
    if let Err(e) = session.connect(credentials, false).await {
        eprintln!("Error connecting: {e}");
        exit(1);
    }

    eprintln!("Streaming track {} to stdout...", track_id_str);
    let player = Player::new(player_config, session, Box::new(NoOpVolume), move || {
        backend(Some("/dev/stdout".to_string()), audio_format)
    });

    player.load(track, true, 0);
    player.await_end_of_track().await;

    // eprintln!("Done!");
    Ok(())
}
/*
 * ORIGINAL FILE FROM HERE
 *
use std::{env, process::exit};

use librespot::{
    core::{
        SpotifyUri, authentication::Credentials, config::SessionConfig, session::Session,
        spotify_id::SpotifyId,
    },
    playback::{
        audio_backend,
        config::{AudioFormat, PlayerConfig},
        mixer::NoOpVolume,
        player::Player,
    },
};

#[tokio::main]
async fn main() {
    let session_config = SessionConfig::default();
    let player_config = PlayerConfig::default();
    let audio_format = AudioFormat::default();

    let args: Vec<_> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} ACCESS_TOKEN TRACK", args[0]);
        return;
    }
    let credentials = Credentials::with_access_token(&args[1]);

    let track = SpotifyUri::Track {
        id: SpotifyId::from_base62(&args[2]).unwrap(),
    };

    let backend = audio_backend::find(None).unwrap();

    println!("Connecting...");
    let session = Session::new(session_config, None);
    if let Err(e) = session.connect(credentials, false).await {
        println!("Error connecting: {e}");
        exit(1);
    }

    let player = Player::new(player_config, session, Box::new(NoOpVolume), move || {
        backend(None, audio_format)
    });

    player.load(track, true, 0);

    println!("Playing...");

    player.await_end_of_track().await;

    println!("Done");
}
*/
