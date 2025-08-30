use crate::models::{message::*, room::*};
use crate::server::connection::ConnectionManager;
use std::sync::Arc;
use tracing::{error, info, warn};

pub struct ChatEngine {
    rooms: Rooms,
    users: Users,
    connection_manager: Arc<ConnectionManager>,
}

impl ChatEngine {
    pub fn new() -> Self {
        Self {
            rooms: Arc::new(dashmap::DashMap::new()),
            users: Arc::new(dashmap::DashMap::new()),
            connection_manager: Arc::new(ConnectionManager::new()),
        }
    }

    pub fn add_connection(&self, user_id: String, sender: tokio::sync::mpsc::UnboundedSender<ServerMessage>) {
        self.connection_manager.add_connection(user_id, sender);
    }

    pub async fn remove_connection(&self, user_id: &str) {
        // Leave current room if any
        if let Some(conn_info) = self.connection_manager.get_connections().get(user_id) {
            if let Some(room_name) = &conn_info.current_room {
                let room_name = room_name.clone();
                if let Err(e) = self.leave_room(user_id, &room_name).await {
                    error!("Error leaving room during disconnect: {}", e);
                }
            }
        }
        
        self.connection_manager.remove_connection(user_id);
        self.users.remove(user_id);
    }

    pub async fn handle_message(&self, user_id: &str, message: ClientMessage) -> Result<(), anyhow::Error> {
        // Validate message
        message.validate().map_err(|e| anyhow::anyhow!(e))?;

        match message {
            ClientMessage::Join { room, username } => {
                self.join_room(user_id, &room, &username).await?;
            }
            ClientMessage::Leave { room } => {
                self.leave_room(user_id, &room).await?;
            }
            ClientMessage::SendMessage { room, message } => {
                self.send_message(user_id, &room, &message).await?;
            }
            ClientMessage::Ping => {
                self.connection_manager.send_to_user(user_id, ServerMessage::Pong).await?;
            }
        }
        
        Ok(())
    }

    async fn join_room(&self, user_id: &str, room_name: &str, username: &str) -> Result<(), anyhow::Error> {
        info!("User {} ({}) joining room '{}'", user_id, username, room_name);

        // Update or create user
        let user = User::new(user_id.to_string(), username.to_string());
        self.users.insert(user_id.to_string(), user);

        // Create room if doesn't exist
        self.rooms.entry(room_name.to_string())
            .or_insert_with(|| Room::new(room_name.to_string()));

        // Add user to room
        let users_in_room = {
            let mut room = self.rooms.get_mut(room_name)
                .ok_or_else(|| anyhow::anyhow!("Room disappeared during join"))?;
            room.add_user(user_id.to_string());
            room.get_usernames(&self.users)
        };

        // Update connection info
        self.connection_manager.update_connection_info(
            user_id, 
            Some(username.to_string()), 
            Some(room_name.to_string())
        );

        // Send room joined confirmation
        self.connection_manager.send_to_user(
            user_id,
            ServerMessage::RoomJoined {
                room: room_name.to_string(),
                users: users_in_room,
            },
        ).await?;

        // Broadcast user joined to others in room
        self.connection_manager.broadcast_to_room(
            room_name,
            ServerMessage::UserJoined {
                room: room_name.to_string(),
                user: username.to_string(),
            },
            Some(user_id),
        ).await?;

        info!("User {} successfully joined room '{}'", username, room_name);
        Ok(())
    }

    async fn leave_room(&self, user_id: &str, room_name: &str) -> Result<(), anyhow::Error> {
        let username = self.users.get(user_id)
            .map(|u| u.username.clone())
            .unwrap_or_else(|| "Unknown".to_string());

        info!("User {} ({}) leaving room '{}'", user_id, username, room_name);

        // Remove user from room
        let should_remove_room = {
            if let Some(mut room) = self.rooms.get_mut(room_name) {
                room.remove_user(user_id);
                room.is_empty()
            } else {
                false
            }
        };

        // Remove empty rooms
        if should_remove_room {
            self.rooms.remove(room_name);
            info!("Removed empty room '{}'", room_name);
        }

        // Update connection info
        self.connection_manager.clear_user_room(user_id);

        // Send confirmation
        self.connection_manager.send_to_user(
            user_id,
            ServerMessage::RoomLeft {
                room: room_name.to_string(),
            },
        ).await?;

        // Broadcast user left to others
        self.connection_manager.broadcast_to_room(
            room_name,
            ServerMessage::UserLeft {
                room: room_name.to_string(),
                user: username.clone(),
            },
            Some(user_id),
        ).await?;

        info!("User {} successfully left room '{}'", username, room_name);
        Ok(())
    }

    async fn send_message(&self, user_id: &str, room_name: &str, message_content: &str) -> Result<(), anyhow::Error> {
        let username = self.users.get(user_id)
            .ok_or_else(|| anyhow::anyhow!("User not found"))?
            .username.clone();

        // Verify user is in room
        let conn_info = self.connection_manager.get_connections().get(user_id)
            .ok_or_else(|| anyhow::anyhow!("Connection not found"))?;
            
        if conn_info.current_room.as_deref() != Some(room_name) {
            return Err(anyhow::anyhow!("User not in room"));
        }

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_millis() as u64;

        // Broadcast message to all users in room
        self.connection_manager.broadcast_to_room(
            room_name,
            ServerMessage::NewMessage {
                room: room_name.to_string(),
                user: username.clone(),
                message: message_content.to_string(),
                timestamp,
            },
            None,
        ).await?;

        info!("Message sent by {} in room '{}': {}", username, room_name, message_content);
        Ok(())
    }

    pub async fn send_error(&self, user_id: &str, error_message: &str) -> Result<(), anyhow::Error> {
        warn!("Sending error to user {}: {}", user_id, error_message);
        self.connection_manager.send_to_user(
            user_id,
            ServerMessage::Error {
                message: error_message.to_string(),
            },
        ).await
    }

    pub fn get_stats(&self) -> (usize, usize, usize) {
        let connection_count = self.connection_manager.connection_count();
        let room_count = self.rooms.len();
        let user_count = self.users.len();
        (connection_count, room_count, user_count)
    }

}