use std::{error::Error, net::SocketAddr};

use dotenvy::dotenv;
use tracing::Level;
#[allow(unused_imports)]
use tracing::{debug, error, info, trace, warn};

mod handle_connection;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().expect(".env file not found");

    let tracing_level = &std::env::var("DEBUG_LEVEL").unwrap_or("INFO".to_string());
    tracing_subscriber::fmt()
        .with_max_level(
            <Level as std::str::FromStr>::from_str(tracing_level).unwrap_or(Level::INFO),
        )
        .init();

    let addr = &std::env::var("ADDRESS").expect("ADDRESS must be set.");
    let addr = addr.parse::<SocketAddr>()?;

    handle_connection::handle_connection(&addr).await?;

    Ok(())
}
