use chrono::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Message {
    sender: String,
    text: String,
    timestamp: String,
}

impl Message {
    pub fn new(sender_id: &str, text: &str) -> Self {
        let timestamp: String = Utc::now().format("%H:%M").to_string();
        Self {
            sender: sender_id.to_owned(),
            text: text.to_owned(),
            timestamp,
        }
    }

    pub fn sender(&self) -> &str {
        self.sender.as_ref()
    }

    pub fn text(&self) -> &str {
        self.text.as_ref()
    }

    pub fn timestamp(&self) -> &str {
        self.timestamp.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize() {
        let sender_id = String::from("asd");
        let text: String = "some text".into();
        let timestamp: String = Utc::now().format("%H:%M").to_string();

        let msg_json = format!(
            r#"{{"sender_id":"{}","text":"{}","timestamp":"{}"}}"#,
            sender_id, text, timestamp
        );
        let msg = Message {
            sender: sender_id,
            text,
            timestamp,
        };

        let serialized = serde_json::to_string(&msg).unwrap();
        assert_eq!(serialized, msg_json);
    }
}
