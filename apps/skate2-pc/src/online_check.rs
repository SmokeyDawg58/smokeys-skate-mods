//! A bounded lobby check using this executable's real build and configured server.
use skate2_net::{
    client::{Client, ClientEvent, Disconnect},
    crypto::Build,
    protocol::{ClientMessage, ServerMessage, refuse},
};
use std::time::{Duration, Instant};

pub(crate) fn run() -> Result<(), String> {
    let config = skate2_core::ui::online_config::OnlineConfig::load();
    let version = env!("CARGO_PKG_VERSION");
    println!("Game version: {version}");
    println!("Network build: {}", Build::current().id());
    println!("Wire protocol: {}", skate2_net::transport::PROTOCOL_VERSION);
    println!("Server: {}", config.server());
    let nickname = if config.has_nickname() {
        config.nickname.trim()
    } else {
        "PhysicsCheck"
    };
    let client = Client::connect(config.server(), nickname, version);
    let deadline = Instant::now() + Duration::from_secs(15);
    while Instant::now() < deadline {
        for event in client.poll() {
            match event {
                ClientEvent::SignedIn { .. } => {
                    println!("Sign-in succeeded.");
                    client.send(ClientMessage::ListLobbies);
                }
                ClientEvent::Message(ServerMessage::Lobbies(lobbies)) => {
                    println!(
                        "Lobby browser succeeded: {} available lobbies.",
                        lobbies.len()
                    );
                    if lobbies.is_empty() {
                        println!(
                            "The server answered successfully; no matching public lobbies are open."
                        );
                    }
                    return Ok(());
                }
                ClientEvent::Disconnected(reason) => return Err(explain(&reason)),
                _ => {}
            }
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    Err("Timed out waiting for the server's lobby list.".into())
}

fn explain(reason: &Disconnect) -> String {
    match reason {
        Disconnect::Refused(refuse::BUILD) => "The server rejected this executable's network build. A matching mod build is required; changing gameplay.json cannot fix this.".into(),
        Disconnect::Refused(refuse::VERSION) => "The server rejected this executable's wire protocol. The mod needs an update for this server.".into(),
        other => format!("Online check failed: {other:?}"),
    }
}
