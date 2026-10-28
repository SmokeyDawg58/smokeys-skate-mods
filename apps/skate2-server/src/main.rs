//! The dedicated server alone: `skate2-pc --server` without the game, for a host with no display
//! (a Linux VPS). It takes the game builds of its own release (`../build_secret.rs`).
//!
//! `skate2-server [ip:port | ip | :port]`, by default every address at port 32082. A release build
//! says when a newer release is out, and `skate2-server --update` puts the newest release's Linux
//! zip in place of this one (`docs/release/auto-update.md`); the server then has to be started
//! again.

use std::process::ExitCode;

fn main() -> ExitCode {
    let build = skate2_net::crypto::Build::from_hex(env!("SKATE2_BUILD_SECRET")).unwrap_or(skate2_net::crypto::Build::DEVELOPMENT);
    build.install();
    let update = skate2_update::Config::for_build(
        option_env!("SKATE2_UPDATE_PUBLIC_KEY"),
        option_env!("SKATE2_UPDATE_REPOSITORY"),
        env!("CARGO_PKG_VERSION"),
        "skate2-server",
    );
    if std::env::args().any(|argument| argument == "--update") {
        return if skate2_update::update_from_command_line(update.as_ref(), "skate2-server") { ExitCode::SUCCESS } else { ExitCode::FAILURE };
    }
    skate2_update::announce_newer_release(update, "skate2-server");
    let bind = std::env::args().skip(1).find(|argument| !argument.starts_with("--"));
    let Some(bind) = skate2_net::server::parse_bind(bind.as_deref()) else {
        eprintln!("usage: skate2-server [ip:port | ip | :port]");
        return ExitCode::FAILURE;
    };
    match skate2_net::server::run(&skate2_net::server::ServerConfig { bind, build }) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("dedicated server failed: {error}");
            ExitCode::FAILURE
        }
    }
}
