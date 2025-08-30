# Implementation Guide

## Getting Started

This guide provides step-by-step instructions for implementing the lightweight Rust chat application.

## Prerequisites

### Required Tools
- **Rust** 1.70+ with Cargo
- **Node.js** 18+ (for client development)
- **Git** for version control

### Development Environment
```bash
# Install Rust if not already installed
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Verify installation
rustc --version
cargo --version
```

## Project Structure

```
communication-project/
├── Cargo.toml                 # Rust dependencies and metadata
├── src/
│   ├── main.rs               # Application entry point
│   ├── lib.rs                # Library root
│   ├── server/               # Server implementation
│   │   ├── mod.rs
│   │   ├── websocket.rs      # WebSocket handling
│   │   ├── connection.rs     # Connection management
│   │   └── chat_engine.rs    # Core chat logic
│   ├── models/               # Data models
│   │   ├── mod.rs
│   │   ├── message.rs        # Message types
│   │   └── room.rs           # Room management
│   └── utils/                # Utilities
│       ├── mod.rs
│       └── config.rs         # Configuration management
├── client/                   # Web client
│   ├── index.html            # Main HTML page
│   ├── app.js                # JavaScript client
│   └── style.css             # Styling
├── docs/                     # Documentation
│   ├── ARCHITECTURE.md
│   ├── API_SPECIFICATION.md
│   └── IMPLEMENTATION_GUIDE.md
└── tests/                    # Integration tests
    ├── integration_tests.rs
    └── websocket_tests.rs
```

## Step 1: Project Setup

### Initialize Rust Project
```bash
# Create project (already done in your case)
cargo new communication-project
cd communication-project
```

### Configure Dependencies

Update `Cargo.toml`:
```toml
[package]
name = "communication-project"
version = "0.1.0"
edition = "2021"

[dependencies]
tokio = { version = "1.35", features = ["full"] }
tokio-tungstenite = "0.21"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tracing = "0.1"
tracing-subscriber = "0.3"
uuid = { version = "1.6", features = ["v4"] }
dashmap = "5.5"
config = "0.14"
anyhow = "1.0"
futures-util = "0.3"

[dev-dependencies]
tokio-test = "0.4"
```

## Step 2: Core Data Models

### Message Types (`src/models/message.rs`)
```rust
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
            }
            ClientMessage::SendMessage { message, .. } => {
                if message.is_empty() || message.len() > 1000 {
                    return Err("Message must be 1-1000 characters".to_string());
                }
            }
            _ => {}
        }
        Ok(())
    }
}
```

### Room Management (`src/models/room.rs`)
```rust
use dashmap::DashMap;
use std::collections::HashSet;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct User {
    pub id: String,
    pub username: String,
    pub rooms: HashSet<String>,
}

#[derive(Debug)]
pub struct Room {
    pub name: String,
    pub users: HashSet<String>,
    pub created_at: u64,
}

pub type Rooms = Arc<DashMap<String, Room>>;
pub type Users = Arc<DashMap<String, User>>;

impl Room {
    pub fn new(name: String) -> Self {
        Self {
            name,
            users: HashSet::new(),
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }

    pub fn add_user(&mut self, user_id: String) {
        self.users.insert(user_id);
    }

    pub fn remove_user(&mut self, user_id: &str) {
        self.users.remove(user_id);
    }

    pub fn get_usernames(&self, users: &Users) -> Vec<String> {
        self.users
            .iter()
            .filter_map(|id| users.get(id).map(|u| u.username.clone()))
            .collect()
    }
}
```

## Step 3: Connection Management

### Connection Manager (`src/server/connection.rs`)
```rust
use crate::models::message::ServerMessage;
use dashmap::DashMap;
use futures_util::SinkExt;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio_tungstenite::{WebSocketStream, tungstenite::Message};

pub type WebSocket = WebSocketStream<TcpStream>;
pub type Connections = Arc<DashMap<String, ConnectionInfo>>;

#[derive(Debug)]
pub struct ConnectionInfo {
    pub user_id: String,
    pub username: Option<String>,
    pub current_room: Option<String>,
}

pub struct ConnectionManager {
    connections: Connections,
}

impl ConnectionManager {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(DashMap::new()),
        }
    }

    pub fn add_connection(&self, user_id: String, connection_info: ConnectionInfo) {
        self.connections.insert(user_id, connection_info);
    }

    pub fn remove_connection(&self, user_id: &str) {
        self.connections.remove(user_id);
    }

    pub fn get_connections(&self) -> &Connections {
        &self.connections
    }

    pub async fn broadcast_to_room(
        &self,
        room: &str,
        message: ServerMessage,
        sender_connections: &Arc<DashMap<String, tokio::sync::mpsc::UnboundedSender<ServerMessage>>>,
        exclude_user: Option<&str>,
    ) -> Result<(), anyhow::Error> {
        let message_json = serde_json::to_string(&message)?;
        
        for entry in self.connections.iter() {
            let user_id = entry.key();
            let conn_info = entry.value();
            
            if Some(room) == conn_info.current_room.as_deref() 
                && exclude_user.map_or(true, |ex| ex != user_id) {
                if let Some(sender) = sender_connections.get(user_id) {
                    let _ = sender.send(message.clone());
                }
            }
        }
        
        Ok(())
    }
}
```

## Step 4: Chat Engine

### Core Logic (`src/server/chat_engine.rs`)
```rust
use crate::models::{message::*, room::*};
use crate::server::connection::*;
use std::sync::Arc;
use tokio::sync::mpsc;
use uuid::Uuid;

pub struct ChatEngine {
    rooms: Rooms,
    users: Users,
    connection_manager: ConnectionManager,
    message_senders: Arc<dashmap::DashMap<String, mpsc::UnboundedSender<ServerMessage>>>,
}

impl ChatEngine {
    pub fn new() -> Self {
        Self {
            rooms: Arc::new(dashmap::DashMap::new()),
            users: Arc::new(dashmap::DashMap::new()),
            connection_manager: ConnectionManager::new(),
            message_senders: Arc::new(dashmap::DashMap::new()),
        }
    }

    pub fn add_connection(&self, user_id: String, sender: mpsc::UnboundedSender<ServerMessage>) {
        self.message_senders.insert(user_id.clone(), sender);
        self.connection_manager.add_connection(
            user_id.clone(),
            ConnectionInfo {
                user_id: user_id.clone(),
                username: None,
                current_room: None,
            }
        );
    }

    pub fn remove_connection(&self, user_id: &str) {
        // Leave current room if any
        if let Some(conn_info) = self.connection_manager.get_connections().get(user_id) {
            if let Some(room_name) = &conn_info.current_room {
                let _ = self.leave_room(user_id, room_name);
            }
        }
        
        self.message_senders.remove(user_id);
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
                self.send_to_user(user_id, ServerMessage::Pong).await?;
            }
        }
        
        Ok(())
    }

    async fn join_room(&self, user_id: &str, room_name: &str, username: &str) -> Result<(), anyhow::Error> {
        // Update user info
        let user = crate::models::room::User {
            id: user_id.to_string(),
            username: username.to_string(),
            rooms: std::collections::HashSet::new(),
        };
        self.users.insert(user_id.to_string(), user);

        // Create room if doesn't exist
        self.rooms.entry(room_name.to_string())
            .or_insert_with(|| Room::new(room_name.to_string()));

        // Add user to room
        let mut room = self.rooms.get_mut(room_name).unwrap();
        room.add_user(user_id.to_string());
        let users_in_room = room.get_usernames(&self.users);
        drop(room);

        // Update connection info
        if let Some(mut conn_info) = self.connection_manager.get_connections().get_mut(user_id) {
            conn_info.username = Some(username.to_string());
            conn_info.current_room = Some(room_name.to_string());
        }

        // Send room joined confirmation
        self.send_to_user(
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
            &self.message_senders,
            Some(user_id),
        ).await?;

        Ok(())
    }

    async fn leave_room(&self, user_id: &str, room_name: &str) -> Result<(), anyhow::Error> {
        let username = self.users.get(user_id)
            .map(|u| u.username.clone())
            .unwrap_or_else(|| "Unknown".to_string());

        // Remove user from room
        if let Some(mut room) = self.rooms.get_mut(room_name) {
            room.remove_user(user_id);
            
            // Remove empty rooms
            if room.users.is_empty() {
                drop(room);
                self.rooms.remove(room_name);
            }
        }

        // Update connection info
        if let Some(mut conn_info) = self.connection_manager.get_connections().get_mut(user_id) {
            conn_info.current_room = None;
        }

        // Send confirmation
        self.send_to_user(
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
                user: username,
            },
            &self.message_senders,
            Some(user_id),
        ).await?;

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
                user: username,
                message: message_content.to_string(),
                timestamp,
            },
            &self.message_senders,
            None,
        ).await?;

        Ok(())
    }

    async fn send_to_user(&self, user_id: &str, message: ServerMessage) -> Result<(), anyhow::Error> {
        if let Some(sender) = self.message_senders.get(user_id) {
            sender.send(message).map_err(|_| anyhow::anyhow!("Failed to send message"))?;
        }
        Ok(())
    }

    pub async fn send_error(&self, user_id: &str, error_message: &str) -> Result<(), anyhow::Error> {
        self.send_to_user(
            user_id,
            ServerMessage::Error {
                message: error_message.to_string(),
            },
        ).await
    }
}
```

## Step 5: WebSocket Handler

### WebSocket Server (`src/server/websocket.rs`)
```rust
use crate::models::message::*;
use crate::server::chat_engine::ChatEngine;
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio_tungstenite::{accept_async, tungstenite::Message};
use tracing::{error, info, warn};
use uuid::Uuid;

pub struct WebSocketServer {
    chat_engine: Arc<ChatEngine>,
}

impl WebSocketServer {
    pub fn new(chat_engine: Arc<ChatEngine>) -> Self {
        Self { chat_engine }
    }

    pub async fn start(&self, addr: &str) -> Result<(), anyhow::Error> {
        let listener = TcpListener::bind(addr).await?;
        info!("WebSocket server listening on {}", addr);

        while let Ok((stream, peer_addr)) = listener.accept().await {
            info!("New connection from {}", peer_addr);
            let chat_engine = Arc::clone(&self.chat_engine);
            
            tokio::spawn(async move {
                if let Err(e) = handle_connection(stream, chat_engine).await {
                    error!("Error handling connection from {}: {}", peer_addr, e);
                }
            });
        }

        Ok(())
    }
}

async fn handle_connection(
    stream: TcpStream,
    chat_engine: Arc<ChatEngine>,
) -> Result<(), anyhow::Error> {
    let ws_stream = accept_async(stream).await?;
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();
    
    let user_id = Uuid::new_v4().to_string();
    let (message_sender, mut message_receiver) = mpsc::unbounded_channel::<ServerMessage>();

    // Add connection to chat engine
    chat_engine.add_connection(user_id.clone(), message_sender);

    // Send welcome message
    let welcome_msg = ServerMessage::Welcome {
        user_id: user_id.clone(),
    };
    let welcome_json = serde_json::to_string(&welcome_msg)?;
    ws_sender.send(Message::Text(welcome_json)).await?;

    // Spawn task to send messages to client
    let user_id_for_sender = user_id.clone();
    let sender_task = tokio::spawn(async move {
        while let Some(message) = message_receiver.recv().await {
            match serde_json::to_string(&message) {
                Ok(json) => {
                    if let Err(e) = ws_sender.send(Message::Text(json)).await {
                        error!("Failed to send message to {}: {}", user_id_for_sender, e);
                        break;
                    }
                }
                Err(e) => {
                    error!("Failed to serialize message for {}: {}", user_id_for_sender, e);
                }
            }
        }
    });

    // Handle incoming messages from client
    while let Some(msg) = ws_receiver.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                match serde_json::from_str::<ClientMessage>(&text) {
                    Ok(client_message) => {
                        if let Err(e) = chat_engine.handle_message(&user_id, client_message).await {
                            warn!("Error handling message from {}: {}", user_id, e);
                            if let Err(send_error) = chat_engine.send_error(&user_id, &e.to_string()).await {
                                error!("Failed to send error message to {}: {}", user_id, send_error);
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Invalid JSON from {}: {}", user_id, e);
                        if let Err(send_error) = chat_engine.send_error(&user_id, "Invalid message format").await {
                            error!("Failed to send error message to {}: {}", user_id, send_error);
                        }
                    }
                }
            }
            Ok(Message::Close(_)) => {
                info!("Client {} disconnected", user_id);
                break;
            }
            Ok(Message::Ping(data)) => {
                if let Err(e) = ws_sender.send(Message::Pong(data)).await {
                    error!("Failed to send pong to {}: {}", user_id, e);
                    break;
                }
            }
            Err(e) => {
                error!("WebSocket error for {}: {}", user_id, e);
                break;
            }
            _ => {}
        }
    }

    // Cleanup
    sender_task.abort();
    chat_engine.remove_connection(&user_id);
    info!("Connection {} cleaned up", user_id);

    Ok(())
}
```

## Step 6: Main Application

### Entry Point (`src/main.rs`)
```rust
mod models;
mod server;
mod utils;

use crate::server::{chat_engine::ChatEngine, websocket::WebSocketServer};
use std::sync::Arc;
use tracing::{info, Level};
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    info!("Starting chat server...");

    // Create chat engine
    let chat_engine = Arc::new(ChatEngine::new());

    // Create and start WebSocket server
    let ws_server = WebSocketServer::new(chat_engine);
    let addr = "127.0.0.1:8080";

    info!("Server will listen on {}", addr);
    ws_server.start(addr).await?;

    Ok(())
}
```

### Library Root (`src/lib.rs`)
```rust
pub mod models;
pub mod server;
pub mod utils;

pub use server::{chat_engine::ChatEngine, websocket::WebSocketServer};
```

### Module Declarations (`src/models/mod.rs`, etc.)
```rust
// src/models/mod.rs
pub mod message;
pub mod room;

// src/server/mod.rs
pub mod chat_engine;
pub mod connection;
pub mod websocket;

// src/utils/mod.rs
pub mod config;
```

## Step 7: Basic Web Client

### HTML Client (`client/index.html`)
```html
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Rust Chat App</title>
    <link rel="stylesheet" href="style.css">
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>Chat Room</h1>
            <div class="connection-status" id="status">Disconnected</div>
        </div>
        
        <div class="join-form" id="joinForm">
            <input type="text" id="usernameInput" placeholder="Enter username" maxlength="50">
            <input type="text" id="roomInput" placeholder="Room name" value="general" maxlength="50">
            <button onclick="joinRoom()">Join Chat</button>
        </div>

        <div class="chat-container" id="chatContainer" style="display: none;">
            <div class="chat-messages" id="messages"></div>
            <div class="users-list">
                <h3>Users</h3>
                <ul id="usersList"></ul>
            </div>
            <div class="message-input">
                <input type="text" id="messageInput" placeholder="Type a message..." maxlength="1000">
                <button onclick="sendMessage()">Send</button>
                <button onclick="leaveRoom()">Leave</button>
            </div>
        </div>
    </div>

    <script src="app.js"></script>
</body>
</html>
```

### JavaScript Client (`client/app.js`)
```javascript
let ws = null;
let currentRoom = null;
let username = null;
let users = [];

function connect() {
    ws = new WebSocket('ws://localhost:8080/ws');
    
    ws.onopen = () => {
        updateStatus('Connected');
        console.log('Connected to server');
    };
    
    ws.onmessage = (event) => {
        const message = JSON.parse(event.data);
        handleMessage(message);
    };
    
    ws.onclose = () => {
        updateStatus('Disconnected');
        console.log('Disconnected from server');
    };
    
    ws.onerror = (error) => {
        updateStatus('Error');
        console.error('WebSocket error:', error);
    };
}

function handleMessage(message) {
    console.log('Received:', message);
    
    switch (message.type) {
        case 'Welcome':
            console.log('Welcome! User ID:', message.data.user_id);
            break;
            
        case 'RoomJoined':
            currentRoom = message.data.room;
            users = message.data.users;
            updateUsersList();
            showChatContainer();
            addSystemMessage(`Joined room: ${message.data.room}`);
            break;
            
        case 'RoomLeft':
            hideJoinForm();
            addSystemMessage(`Left room: ${message.data.room}`);
            break;
            
        case 'NewMessage':
            addMessage(message.data.user, message.data.message, new Date(message.data.timestamp));
            break;
            
        case 'UserJoined':
            if (!users.includes(message.data.user)) {
                users.push(message.data.user);
                updateUsersList();
            }
            addSystemMessage(`${message.data.user} joined`);
            break;
            
        case 'UserLeft':
            users = users.filter(u => u !== message.data.user);
            updateUsersList();
            addSystemMessage(`${message.data.user} left`);
            break;
            
        case 'Error':
            alert('Error: ' + message.data.message);
            break;
            
        case 'Pong':
            console.log('Received pong');
            break;
    }
}

function joinRoom() {
    const usernameInput = document.getElementById('usernameInput');
    const roomInput = document.getElementById('roomInput');
    
    username = usernameInput.value.trim();
    const room = roomInput.value.trim();
    
    if (!username || !room) {
        alert('Please enter both username and room name');
        return;
    }
    
    if (!ws || ws.readyState !== WebSocket.OPEN) {
        alert('Not connected to server');
        return;
    }
    
    ws.send(JSON.stringify({
        type: 'Join',
        data: { room, username }
    }));
}

function leaveRoom() {
    if (currentRoom && ws && ws.readyState === WebSocket.OPEN) {
        ws.send(JSON.stringify({
            type: 'Leave',
            data: { room: currentRoom }
        }));
    }
    hideJoinForm();
}

function sendMessage() {
    const messageInput = document.getElementById('messageInput');
    const message = messageInput.value.trim();
    
    if (!message || !currentRoom) return;
    
    if (ws && ws.readyState === WebSocket.OPEN) {
        ws.send(JSON.stringify({
            type: 'SendMessage',
            data: { room: currentRoom, message }
        }));
        messageInput.value = '';
    }
}

function addMessage(user, message, timestamp) {
    const messages = document.getElementById('messages');
    const messageDiv = document.createElement('div');
    messageDiv.className = 'message';
    
    const time = timestamp.toLocaleTimeString();
    messageDiv.innerHTML = `
        <span class="timestamp">${time}</span>
        <span class="username">${user}:</span>
        <span class="content">${escapeHtml(message)}</span>
    `;
    
    messages.appendChild(messageDiv);
    messages.scrollTop = messages.scrollHeight;
}

function addSystemMessage(message) {
    const messages = document.getElementById('messages');
    const messageDiv = document.createElement('div');
    messageDiv.className = 'system-message';
    messageDiv.textContent = message;
    
    messages.appendChild(messageDiv);
    messages.scrollTop = messages.scrollHeight;
}

function updateUsersList() {
    const usersList = document.getElementById('usersList');
    usersList.innerHTML = '';
    
    users.forEach(user => {
        const li = document.createElement('li');
        li.textContent = user;
        if (user === username) {
            li.className = 'current-user';
        }
        usersList.appendChild(li);
    });
}

function showChatContainer() {
    document.getElementById('joinForm').style.display = 'none';
    document.getElementById('chatContainer').style.display = 'block';
}

function hideJoinForm() {
    document.getElementById('joinForm').style.display = 'block';
    document.getElementById('chatContainer').style.display = 'none';
    currentRoom = null;
    users = [];
}

function updateStatus(status) {
    document.getElementById('status').textContent = status;
}

function escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
}

// Event listeners
document.getElementById('messageInput').addEventListener('keypress', (e) => {
    if (e.key === 'Enter') {
        sendMessage();
    }
});

document.getElementById('usernameInput').addEventListener('keypress', (e) => {
    if (e.key === 'Enter') {
        document.getElementById('roomInput').focus();
    }
});

document.getElementById('roomInput').addEventListener('keypress', (e) => {
    if (e.key === 'Enter') {
        joinRoom();
    }
});

// Connect on page load
connect();

// Reconnection logic
setInterval(() => {
    if (!ws || ws.readyState === WebSocket.CLOSED) {
        console.log('Attempting to reconnect...');
        connect();
    }
}, 5000);

// Heartbeat
setInterval(() => {
    if (ws && ws.readyState === WebSocket.OPEN) {
        ws.send(JSON.stringify({ type: 'Ping', data: null }));
    }
}, 30000);
```

### Basic Styling (`client/style.css`)
```css
* {
    margin: 0;
    padding: 0;
    box-sizing: border-box;
}

body {
    font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
    background: #f5f5f5;
    height: 100vh;
}

.container {
    max-width: 800px;
    margin: 0 auto;
    height: 100vh;
    display: flex;
    flex-direction: column;
    background: white;
    box-shadow: 0 0 20px rgba(0,0,0,0.1);
}

.header {
    background: #2c3e50;
    color: white;
    padding: 1rem;
    display: flex;
    justify-content: space-between;
    align-items: center;
}

.connection-status {
    padding: 0.5rem 1rem;
    border-radius: 20px;
    background: #e74c3c;
    font-size: 0.9rem;
}

.join-form {
    padding: 2rem;
    text-align: center;
}

.join-form input {
    margin: 0.5rem;
    padding: 0.75rem;
    font-size: 1rem;
    border: 2px solid #ddd;
    border-radius: 5px;
    width: 200px;
}

.join-form button {
    margin: 0.5rem;
    padding: 0.75rem 2rem;
    font-size: 1rem;
    background: #3498db;
    color: white;
    border: none;
    border-radius: 5px;
    cursor: pointer;
}

.join-form button:hover {
    background: #2980b9;
}

.chat-container {
    flex: 1;
    display: flex;
    flex-direction: column;
}

.chat-messages {
    flex: 1;
    padding: 1rem;
    overflow-y: auto;
    border-bottom: 1px solid #ddd;
}

.message {
    margin-bottom: 0.5rem;
    padding: 0.5rem;
    border-radius: 5px;
    background: #f8f9fa;
}

.timestamp {
    color: #6c757d;
    font-size: 0.8rem;
    margin-right: 0.5rem;
}

.username {
    font-weight: bold;
    color: #2c3e50;
    margin-right: 0.5rem;
}

.system-message {
    color: #6c757d;
    font-style: italic;
    text-align: center;
    margin: 0.5rem 0;
    padding: 0.25rem;
}

.users-list {
    background: #f8f9fa;
    padding: 1rem;
    border-bottom: 1px solid #ddd;
}

.users-list h3 {
    margin-bottom: 0.5rem;
    color: #2c3e50;
}

.users-list ul {
    list-style: none;
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
}

.users-list li {
    background: #e9ecef;
    padding: 0.25rem 0.5rem;
    border-radius: 15px;
    font-size: 0.9rem;
}

.current-user {
    background: #3498db !important;
    color: white;
}

.message-input {
    padding: 1rem;
    display: flex;
    gap: 0.5rem;
    background: #f8f9fa;
}

.message-input input {
    flex: 1;
    padding: 0.75rem;
    font-size: 1rem;
    border: 2px solid #ddd;
    border-radius: 25px;
}

.message-input button {
    padding: 0.75rem 1.5rem;
    font-size: 1rem;
    background: #27ae60;
    color: white;
    border: none;
    border-radius: 25px;
    cursor: pointer;
}

.message-input button:last-child {
    background: #e74c3c;
}

.message-input button:hover {
    opacity: 0.8;
}

/* Connected status */
.connection-status:has-text("Connected") {
    background: #27ae60;
}
```

## Step 8: Building and Testing

### Build the Application
```bash
# Build in debug mode
cargo build

# Build optimized release
cargo build --release

# Run the server
cargo run

# Or run with logging
RUST_LOG=debug cargo run
```

### Testing
```bash
# Run unit tests
cargo test

# Run with output
cargo test -- --nocapture

# Test specific module
cargo test models::message
```

### Development Workflow
1. **Start server**: `cargo run`
2. **Open client**: Navigate to `client/index.html` in browser
3. **Test functionality**: Join rooms, send messages, multiple users
4. **Monitor logs**: Check console for server logs
5. **Debug issues**: Use browser dev tools for client debugging

## Next Steps

### Production Deployment
- [ ] Add TLS/SSL support (WSS)
- [ ] Implement proper logging
- [ ] Add configuration file support
- [ ] Set up reverse proxy (nginx)
- [ ] Add monitoring and metrics

### Feature Enhancements
- [ ] Message persistence with database
- [ ] User authentication
- [ ] Private messaging
- [ ] File sharing
- [ ] Rate limiting
- [ ] Admin features

### Performance Optimization  
- [ ] Connection pooling
- [ ] Message batching
- [ ] Horizontal scaling
- [ ] Load balancing
- [ ] Caching layer

This implementation guide provides a complete foundation for building a lightweight, real-time chat application in Rust. The architecture is designed to be simple yet scalable, making it easy to extend with additional features as needed.