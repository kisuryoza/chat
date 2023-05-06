use mio::{net::TcpStream, Events, Interest, Poll, Token};
use std::{error::Error, net::SocketAddr};

use dotenvy::dotenv;
use tracing::Level;
#[allow(unused_imports)]
use tracing::{debug, error, info, trace, warn};

// Some tokens to allow us to identify which event is for which socket.
const SERVER: Token = Token(0);
const CLIENT: Token = Token(1);

fn main() -> Result<(), Box<dyn Error>> {
    dotenv().expect(".env file not found");

    let tracing_level = &std::env::var("DEBUG_LEVEL").unwrap_or("INFO".to_string());
    tracing_subscriber::fmt()
        .with_max_level(
            <Level as std::str::FromStr>::from_str(tracing_level).unwrap_or(Level::INFO),
        )
        .init();

    // Create a poll instance.
    let mut poll = Poll::new()?;
    // Create storage for events.
    let mut events = Events::with_capacity(128);

    // Setup the client socket.
    let addr = &std::env::var("ADDRESS").expect("ADDRESS must be set.");
    let addr = addr.parse::<SocketAddr>()?;
    let mut client = TcpStream::connect(addr)?;

    // Register the socket.
    poll.registry()
        .register(&mut client, CLIENT, Interest::READABLE | Interest::WRITABLE)?;

    // Start an event loop.
    loop {
        // Poll Mio for events, blocking until we get an event.
        poll.poll(&mut events, None)?;

        // Process each event.
        for event in events.iter() {
            // We can use the token we previously provided to `register` to
            // determine for which socket the event is.
            match event.token() {
                SERVER => {
                    // If this is an event for the server, it means a connection
                    // is ready to be accepted.
                    //
                    // Accept the connection and drop it immediately. This will
                    // close the socket and notify the client of the EOF.
                    // let connection = server.accept();
                    // drop(connection);
                }
                CLIENT => {
                    if event.is_writable() {
                        // We can (likely) write to the socket without blocking.
                    }

                    if event.is_readable() {
                        // We can (likely) read from the socket without blocking.
                    }
                    debug!("hello");
                    // Since the server just shuts down the connection, let's
                    // just exit from our event loop.
                    return Ok(());
                }
                // We don't expect any events with tokens other than those we provided.
                _ => unreachable!(),
            }
        }
    }
}
