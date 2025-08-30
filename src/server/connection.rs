use crate::models::message::ServerMessage;
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{error, info};

pub type MessageSender = mpsc::UnboundedSender<ServerMessage>;
pub type MessageSenders = Arc<DashMap<String, MessageSender>>;

#[derive(Debug)]
pub struct ConnectionInfo {
    #[allow(dead_code)]
    pub user_id: String,
    pub username: Option<String>,
    pub current_room: Option<String>,
}

pub type Connections = Arc<DashMap<String, ConnectionInfo>>;

pub struct ConnectionManager {
    connections: Connections,
    message_senders: MessageSenders,
}

impl ConnectionManager {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(DashMap::new()),
            message_senders: Arc::new(DashMap::new()),
        }
    }

    pub fn add_connection(&self, user_id: String, sender: MessageSender) {
        let connection_info = ConnectionInfo {
            user_id: user_id.clone(),
            username: None,
            current_room: None,
        };
        
        self.connections.insert(user_id.clone(), connection_info);
        self.message_senders.insert(user_id.clone(), sender);
        
        info!("Added connection for user: {}", user_id);
    }

    pub fn remove_connection(&self, user_id: &str) {
        self.connections.remove(user_id);
        self.message_senders.remove(user_id);
        info!("Removed connection for user: {}", user_id);
    }

    pub fn update_connection_info(&self, user_id: &str, username: Option<String>, current_room: Option<String>) {
        if let Some(mut conn_info) = self.connections.get_mut(user_id) {
            if let Some(username) = username {
                conn_info.username = Some(username);
            }
            if let Some(room) = current_room {
                conn_info.current_room = Some(room);
            }
        }
    }

    pub fn clear_user_room(&self, user_id: &str) {
        if let Some(mut conn_info) = self.connections.get_mut(user_id) {
            conn_info.current_room = None;
        }
    }

    pub fn get_connections(&self) -> &Connections {
        &self.connections
    }

    #[allow(dead_code)]
    pub fn get_message_senders(&self) -> &MessageSenders {
        &self.message_senders
    }

    pub async fn send_to_user(&self, user_id: &str, message: ServerMessage) -> Result<(), anyhow::Error> {
        if let Some(sender) = self.message_senders.get(user_id) {
            sender.send(message).map_err(|_| anyhow::anyhow!("Failed to send message to user {}", user_id))?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("User {} not found", user_id))
        }
    }

    pub async fn broadcast_to_room(
        &self,
        room: &str,
        message: ServerMessage,
        exclude_user: Option<&str>,
    ) -> Result<(), anyhow::Error> {
        let mut sent_count = 0;
        let mut error_count = 0;

        for entry in self.connections.iter() {
            let user_id = entry.key();
            let conn_info = entry.value();
            
            if Some(room) == conn_info.current_room.as_deref() 
                && exclude_user.map_or(true, |ex| ex != user_id) {
                
                if let Some(sender) = self.message_senders.get(user_id) {
                    match sender.send(message.clone()) {
                        Ok(_) => sent_count += 1,
                        Err(_) => {
                            error!("Failed to send message to user {}", user_id);
                            error_count += 1;
                        }
                    }
                }
            }
        }

        info!("Broadcasted message to room '{}': {} sent, {} errors", room, sent_count, error_count);
        Ok(())
    }

    #[allow(dead_code)]
    pub fn get_room_users(&self, room: &str) -> Vec<String> {
        self.connections
            .iter()
            .filter_map(|entry| {
                let conn_info = entry.value();
                if conn_info.current_room.as_deref() == Some(room) {
                    conn_info.username.clone()
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn connection_count(&self) -> usize {
        self.connections.len()
    }
}