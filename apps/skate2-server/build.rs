//! The build's secret: see `../build_secret.rs`. Only a release key gives it the secret of the
//! game builds of its version.

#[path = "../build_secret.rs"]
mod build_secret;

fn main() {
    build_secret::emit(&["build.rs", "../build_secret.rs", "Cargo.toml", "src", "../../crates/skate2-net", "../../crates/skate2-update", "../../Cargo.toml", "../../Cargo.lock"]);
}
