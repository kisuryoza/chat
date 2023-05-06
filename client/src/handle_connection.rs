use futures::{future, SinkExt, StreamExt};
use std::{error::Error, net::SocketAddr};
use tokio::net::TcpStream;
use tokio_util::codec::{FramedRead, FramedWrite, LinesCodec};

#[allow(unused_imports)]
use tracing::{debug, error, info, trace, warn};

use lib::Message;

pub async fn handle_connection(addr: &SocketAddr) -> Result<(), Box<dyn Error>> {
    let stream = TcpStream::connect(addr).await?;
    let (r, w) = stream.into_split();

    println!("Enter your username:");
    let mut buff = String::new();
    std::io::stdin()
        .read_line(&mut buff)
        .expect("reading from stdin failed");

    let h1: tokio::task::JoinHandle<Result<(), tokio_util::codec::LinesCodecError>> =
        tokio::spawn(async move {
            let username = buff.trim();
            let mut sink = FramedWrite::new(w, LinesCodec::new());
            let mut buff = String::new();
            loop {
                std::io::stdin()
                    .read_line(&mut buff)
                    .expect("reading from stdin failed");
                let msg = buff.trim();
                if msg == ":q" {
                    break;
                }
                let msg = Message::new(&username, msg);
                let serialized = serde_json::to_string(&msg).unwrap();
                sink.send(serialized).await?;
                buff.clear();
            }
            Ok(())
        });

    let h2: tokio::task::JoinHandle<Result<(), ()>> = tokio::spawn(async move {
        let mut stream = FramedRead::new(r, LinesCodec::new());
        loop {
            match stream.next().await {
                Some(line) => match line {
                    Ok(line) => match serde_json::from_str::<Message>(&line) {
                        Ok(deserialized) => {
                            println!(
                                "{}: {}: {}",
                                deserialized.timestamp(),
                                deserialized.sender(),
                                deserialized.text(),
                            )
                        }
                        Err(e) => debug!("Recieved bad json: {}", e),
                    },
                    Err(_) => todo!(),
                },
                None => {
                    error!("Error while recieving/writing to stdout");
                    return Ok(());
                }
            }
        }
    });

    match future::join(h1, h2).await {
        // (Err(e), _) | (_, Err(e)) => Err(e.into()),
        (Err(e), _) => Err(e.into()),
        _ => Ok(()),
    }
}
