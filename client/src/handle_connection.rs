use bytes::Bytes;
use futures::{future, SinkExt, StreamExt};
use std::net::SocketAddr;
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

use crate::message::Message;

#[derive(Debug)]
pub enum MyError {
    Io(tokio::io::Error),
    Quit,
}

pub async fn handle_connection(addr: &SocketAddr) -> Result<(), MyError> {
    let tcp_stream = TcpStream::connect(addr).await.map_err(MyError::Io)?;
    let (r, w) = tcp_stream.into_split();
    let mut stream = FramedRead::new(r, BytesCodec::new());
    let mut sink = FramedWrite::new(w, BytesCodec::new());

    println!("Enter your username:");
    let mut buff = String::new();
    std::io::stdin()
        .read_line(&mut buff)
        .expect("reading from stdin failed");

    // FIX: keys are being sent even if another client hasn't connected
    let shared_key = key_exchange(&mut stream, &mut sink).await;
    let cipher1 = Aes256Gcm::new_from_slice(&shared_key).unwrap();
    let cipher2 = cipher1.clone();

    let send: tokio::task::JoinHandle<Result<(), MyError>> = tokio::spawn(async move {
        let username = buff.trim();
        let mut buff = String::new();
        let cipher = cipher1;

        loop {
            match send(&mut sink, &mut buff, username, &cipher).await {
                Ok(_) => (),
                Err(MyError::Quit) => return Ok(()),
                Err(err) => return Err(err),
            }
        }
    });

    let recieve: tokio::task::JoinHandle<Result<(), MyError>> = tokio::spawn(async move {
        let cipher = cipher2;
        loop {
            match recieve(&mut stream, &cipher).await {
                Ok(_) => (),
                Err(MyError::Quit) => return Ok(()),
                Err(err) => return Err(err),
            }
        }
    });

    match future::join(send, recieve).await {
        (Err(e), _) | (_, Err(e)) => Err(MyError::Io(e.into())),
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

    let mut shared_key: Vec<u8> = Vec::new();
    while let Some(line) = stream.next().await {
        let recieved = match line {
            Ok(recieved) => recieved,
            Err(err) => {
                error!("Recieved invalid data: {:?}", err);
                continue;
            }
        };

        trace!("Recieved ephemeral pub key");
        let sender_ephemeral_public =
            PublicKey::from_sec1_bytes(recieved.as_ref()).expect("public key is invalid!"); // In real usage, don't panic, handle this!
        let shared = ephemeral_secret.diffie_hellman(&sender_ephemeral_public);
        shared_key = shared.raw_secret_bytes().to_vec();
        break;
    }
    shared_key
}

async fn send(
    sink: &mut FramedWrite<tokio::net::tcp::OwnedWriteHalf, BytesCodec>,
    buff: &mut String,
    username: &str,
    cipher: &aes_gcm::AesGcm<aes_gcm::aes::Aes256, aead::consts::U12>,
) -> Result<(), MyError> {
    std::io::stdin()
        .read_line(buff)
        .expect("reading from stdin failed");
    let message = buff.trim();
    if message == ":q" {
        return Err(MyError::Quit);
    }

    let message = Message::new(username, message);
    let serialized = match serde_json::to_string(&message) {
        Ok(serialized) => serialized,
        Err(err) => {
            error!("Failed to convert message to json: {:?}", err);
            return Ok(());
        }
    };

    let nonce = Aes256Gcm::generate_nonce(&mut OsRng); // 96-bits; unique per message
    let mut ciphertext = match cipher.encrypt(&nonce, serialized.as_ref()) {
        Ok(ciphertext) => ciphertext,
        Err(err) => {
            error!("Failed to encrypt message: {:?}", err);
            return Ok(());
        }
    };

    let mut to_send: Vec<u8> = Vec::new();
    to_send.append(&mut nonce.to_vec());
    to_send.append(&mut ciphertext);
    let bytes = Bytes::from(to_send);

    sink.send(bytes).await.map_err(MyError::Io)?;
    trace!("Message sent");
    buff.clear();
    Ok(())
}

async fn recieve(
    stream: &mut FramedRead<tokio::net::tcp::OwnedReadHalf, BytesCodec>,
    cipher2: &aes_gcm::AesGcm<aes_gcm::aes::Aes256, aead::consts::U12>,
) -> Result<(), MyError> {
    let recieved = match stream.next().await {
        Some(Ok(recieved)) => recieved,
        Some(Err(err)) => {
            error!("Recieved invalid data: {:?}", err);
            return Ok(());
        }
        None => {
            debug!("The stream has been closed");
            return Err(MyError::Quit);
        }
    };

    // Why 12? Because first 92 bits is nonce and the rest is ciphertext
    let (nonce, ciphertext) = recieved.split_at(12);
    let nonce = Nonce::from_slice(nonce);

    let plaintext = match cipher2.decrypt(nonce, ciphertext.as_ref()) {
        Ok(ciphertext) => ciphertext,
        Err(err) => {
            error!("Failed to decrypt message: {:?}", err);
            return Ok(());
        }
    };

    let plaintext = std::str::from_utf8(&plaintext).unwrap();
    let deserialized = match serde_json::from_str::<Message>(plaintext) {
        Ok(deserialized) => deserialized,
        Err(err) => {
            debug!("Recieved invalid json: {:?}", err);
            return Ok(());
        }
    };

    println!(
        "{}: {}: {}",
        deserialized.timestamp(),
        deserialized.sender(),
        deserialized.text(),
    );
    Ok(())
}
