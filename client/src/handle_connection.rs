use bytes::Bytes;
use futures::{future, SinkExt, StreamExt};
use std::{error::Error, net::SocketAddr};
use tokio::net::TcpStream;
use tokio_util::codec::{BytesCodec, FramedRead, FramedWrite};

use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit},
    Aes256Gcm, Nonce,
};
use p256::{ecdh::EphemeralSecret, EncodedPoint, PublicKey};
use rand_core::OsRng;

#[allow(unused_imports)]
use tracing::{debug, error, info, trace, warn};

use lib::Message;

#[derive(Debug)]
pub enum MyError {
    TokError(tokio::io::Error),
    InvalidLength(aes_gcm::aes::cipher::InvalidLength),
    Serde(serde_json::Error),
    AeadError(aead::Error),
    Utf8Error(std::str::Utf8Error),
}

pub async fn handle_connection(addr: &SocketAddr) -> Result<(), MyError> {
    let tcp_stream = TcpStream::connect(addr)
        .await
        .map_err(|e| MyError::TokError(e))?;
    let (r, w) = tcp_stream.into_split();
    let mut stream = FramedRead::new(r, BytesCodec::new());
    let mut sink = FramedWrite::new(w, BytesCodec::new());

    println!("Enter your username:");
    let mut buff = String::new();
    std::io::stdin()
        .read_line(&mut buff)
        .expect("reading from stdin failed");

    let shared_key = key_exchange(&mut stream, &mut sink).await;
    let cipher1 = Aes256Gcm::new_from_slice(&shared_key).map_err(|e| MyError::InvalidLength(e))?;
    let cipher2 = cipher1.clone();

    let send: tokio::task::JoinHandle<Result<(), MyError>> = tokio::spawn(async move {
        let username = buff.trim();
        let mut buff = String::new();

        loop {
            std::io::stdin()
                .read_line(&mut buff)
                .expect("reading from stdin failed");
            let message = buff.trim();
            if message == ":q" {
                break;
            }
            let message = Message::new(username, message);
            let serialized = serde_json::to_string(&message).map_err(|e| MyError::Serde(e))?;

            let nonce = Aes256Gcm::generate_nonce(&mut OsRng); // 96-bits; unique per message
            let mut ciphertext = cipher1
                .encrypt(&nonce, serialized.as_ref())
                .map_err(|e| MyError::AeadError(e))?;

            let mut to_send: Vec<u8> = Vec::new();
            to_send.append(&mut nonce.to_vec());
            to_send.append(&mut ciphertext);
            let bytes = Bytes::from(to_send);
            sink.send(bytes).await.map_err(|e| MyError::TokError(e))?;
            trace!("Message sent");

            buff.clear();
        }
        Ok(())
    });

    let recieve: tokio::task::JoinHandle<Result<(), MyError>> = tokio::spawn(async move {
        loop {
            match stream.next().await {
                Some(line) => match line {
                    Ok(recieved) => {
                        let (nonce, ciphertext) = recieved.split_at(12);
                        let nonce = Nonce::from_slice(nonce);
                        let plaintext = cipher2
                            .decrypt(nonce, ciphertext.as_ref())
                            .map_err(|e| MyError::AeadError(e))?;
                        let plaintext =
                            std::str::from_utf8(&plaintext).map_err(|e| MyError::Utf8Error(e))?;
                        match serde_json::from_str::<Message>(plaintext) {
                            Ok(deserialized) => {
                                println!(
                                    "{}: {}: {}",
                                    deserialized.timestamp(),
                                    deserialized.sender(),
                                    deserialized.text(),
                                )
                            }
                            Err(e) => debug!("Recieved bad json: {}", e),
                        }
                    }
                    Err(err) => panic!("{:?}", err),
                },
                None => {
                    error!("Error while recieving/writing to stdout");
                    return Ok(());
                }
            }
        }
    });

    match future::join(send, recieve).await {
        (Err(e), _) | (_, Err(e)) => Err(MyError::TokError(e.into())),
        _ => Ok(()),
    }
}

async fn key_exchange(
    stream: &mut FramedRead<tokio::net::tcp::OwnedReadHalf, BytesCodec>,
    sink: &mut FramedWrite<tokio::net::tcp::OwnedWriteHalf, BytesCodec>,
) -> Vec<u8> {
    let ephemeral_secret = EphemeralSecret::random(&mut OsRng);
    let ephemeral_public = EncodedPoint::from(ephemeral_secret.public_key());

    let bytes = Bytes::from(ephemeral_public.to_bytes());
    sink.send(bytes).await.unwrap();
    trace!("Sent ephemeral pub key");

    let mut shared_key = Vec::new();
    while let Some(line) = stream.next().await {
        match line {
            Ok(recieved) => {
                trace!("Recieved ephemeral pub key");
                let sender_ephemeral_public =
                    PublicKey::from_sec1_bytes(recieved.as_ref()).expect("public key is invalid!"); // In real usage, don't panic, handle this!
                let shared = ephemeral_secret.diffie_hellman(&sender_ephemeral_public);
                shared_key = shared.raw_secret_bytes().to_vec();
                break;
            }
            Err(err) => panic!("{:?}", err),
        }
    }
    shared_key
}
