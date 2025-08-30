use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", content = "data")]
pub enum ClientMessage {
    Join { room: String, username: String },
    Leave { room: String },
    SendMessage { room: String, message: String },
    Ping,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", content = "data")]
pub enum ServerMessage {
    Welcome { user_id: String },
    RoomJoined { room: String, users: Vec<String> },
    RoomLeft { room: String },
    NewMessage { 
        room: String, 
        user: String, 
        message: String, 
        timestamp: u64 
    },
    UserJoined { room: String, user: String },
    UserLeft { room: String, user: String },
    Error { message: String },
    Pong,
}

impl ClientMessage {
    pub fn validate(&self) -> Result<(), String> {
        match self {
            ClientMessage::Join { room, username } => {
                if room.is_empty() || room.len() > 50 {
                    return Err("Room name must be 1-50 characters".to_string());
                }
                if username.is_empty() || username.len() > 50 {
                    return Err("Username must be 1-50 characters".to_string());
                }
                if !room.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
                    return Err("Room name can only contain alphanumeric characters, underscores, and hyphens".to_string());
                }
                if !username.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
                    return Err("Username can only contain alphanumeric characters, underscores, and hyphens".to_string());
                }
            }
            ClientMessage::Leave { room } => {
                if room.is_empty() || room.len() > 50 {
                    return Err("Room name must be 1-50 characters".to_string());
                }
            }
            ClientMessage::SendMessage { room, message } => {
                if room.is_empty() || room.len() > 50 {
                    return Err("Room name must be 1-50 characters".to_string());
                }
                if message.is_empty() || message.len() > 1000 {
                    return Err("Message must be 1-1000 characters".to_string());
                }
            }
            ClientMessage::Ping => {}
        }
        Ok(())
    }
}