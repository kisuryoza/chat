use tokio::{net::TcpListener, sync::Mutex};

use std::{error::Error, net::SocketAddr, sync::Arc};

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

    // Create the shared state. This is how all the peers communicate.
    //
    // The server task will hold a handle to this. For every new client, the
    // `state` handle is cloned and passed into the task that processes the
    // client connection.
    let state = Arc::new(Mutex::new(handle_connection::Shared::new()));

    // Bind a TCP listener to the socket address.
    //
    // Note that this is the Tokio TcpListener, which is fully async.
    let addr = &std::env::var("ADDRESS").expect("ADDRESS must be set.");
    let addr = addr.parse::<SocketAddr>().unwrap();
    let listener = TcpListener::bind(&addr).await?;

    info!("Server is running on {}", addr);

    loop {
        // Asynchronously wait for an inbound TcpStream.
        let (stream, addr) = listener.accept().await?;

        // Clone a handle to the `Shared` state for the new connection.
        let state = Arc::clone(&state);

        // Spawn our handler to be run asynchronously.
        tokio::spawn(async move {
            info!("Accepted connection from: {}", addr);
            if let Err(e) = handle_connection::process(state, stream, addr).await {
                error!("an error occurred; error = {:?}", e);
            }
            info!("Connection closed: {}", &addr);
        });
    }
}
