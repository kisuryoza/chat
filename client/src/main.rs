use std::net::SocketAddr;
use std::process::{ExitCode, Termination};

use dotenvy::dotenv;
use tracing::Level;
#[allow(unused_imports)]
use tracing::{debug, error, info, trace, warn};

mod handle_connection;
mod message;

#[repr(u8)]
pub enum GitBisectResult {
    Good = 0,
    Bad = 1,
    Skip = 125,
    Abort = 255,
}

impl Termination for GitBisectResult {
    fn report(self) -> ExitCode {
        // Maybe print a message here
        ExitCode::from(self as u8)
    }
}

#[tokio::main]
async fn main() -> GitBisectResult {
    dotenv().expect(".env file not found");

    let tracing_level = &std::env::var("DEBUG_LEVEL").unwrap_or("INFO".to_string());
    tracing_subscriber::fmt()
        .with_max_level(
            <Level as std::str::FromStr>::from_str(tracing_level).unwrap_or(Level::INFO),
        )
        .init();

    let addr = &std::env::var("ADDRESS").expect("ADDRESS must be set.");
    let addr = addr.parse::<SocketAddr>().unwrap();

    handle_connection::handle_connection(&addr).await.unwrap();

    GitBisectResult::Good
}
