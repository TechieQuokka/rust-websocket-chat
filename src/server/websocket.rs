use crate::models::message::{ClientMessage, ServerMessage};
use crate::server::chat_engine::ChatEngine;
use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use hyper::{
    header::{CONNECTION, UPGRADE, SEC_WEBSOCKET_ACCEPT, SEC_WEBSOCKET_KEY},
    server::conn::AddrStream,
    service::{make_service_fn, service_fn},
    upgrade::Upgraded,
    Body, Method, Request, Response, Server, StatusCode, Version,
};
use hyper_tungstenite::{tungstenite::Message, WebSocketStream};
use std::sync::Arc;
use tokio::sync::mpsc;
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
        let chat_engine = Arc::clone(&self.chat_engine);
        
        let make_svc = make_service_fn(move |conn: &AddrStream| {
            let chat_engine = Arc::clone(&chat_engine);
            let remote_addr = conn.remote_addr();
            
            async move {
                Ok::<_, hyper::Error>(service_fn(move |req| {
                    let chat_engine = Arc::clone(&chat_engine);
                    handle_request(req, chat_engine, remote_addr)
                }))
            }
        });

        let addr = addr.parse()?;
        let server = Server::bind(&addr).serve(make_svc);

        info!("WebSocket server listening on {}", addr);
        
        if let Err(e) = server.await {
            error!("Server error: {}", e);
            return Err(e.into());
        }

        Ok(())
    }
}

async fn handle_request(
    req: Request<Body>,
    chat_engine: Arc<ChatEngine>,
    remote_addr: std::net::SocketAddr,
) -> Result<Response<Body>, hyper::Error> {
    info!("New connection from {}", remote_addr);

    // Check if this is a WebSocket upgrade request
    if req.method() == Method::GET 
        && req.uri().path() == "/ws"
        && req.headers().get(CONNECTION).map_or(false, |v| v.to_str().unwrap_or("").to_lowercase().contains("upgrade"))
        && req.headers().get(UPGRADE).map_or(false, |v| v.to_str().unwrap_or("").to_lowercase() == "websocket")
        && req.version() >= Version::HTTP_11
    {
        let ws_key = req.headers().get(SEC_WEBSOCKET_KEY).cloned();
        if let Some(key) = ws_key {
            // Perform WebSocket handshake
            tokio::spawn(async move {
                match hyper::upgrade::on(req).await {
                    Ok(upgraded) => {
                        if let Err(e) = handle_websocket(upgraded, chat_engine, remote_addr).await {
                            error!("WebSocket error for {}: {}", remote_addr, e);
                        }
                    }
                    Err(e) => {
                        error!("Upgrade error for {}: {}", remote_addr, e);
                    }
                }
            });

            // Return WebSocket handshake response
            let accept_key = hyper_tungstenite::tungstenite::handshake::derive_accept_key(key.as_bytes());
            
            return Ok(Response::builder()
                .status(StatusCode::SWITCHING_PROTOCOLS)
                .header(CONNECTION, "upgrade")
                .header(UPGRADE, "websocket")
                .header(SEC_WEBSOCKET_ACCEPT, accept_key)
                .body(Body::empty())
                .unwrap());
        }
    }

    // Handle regular HTTP requests (serve static files or return 404)
    match req.uri().path() {
        "/" => {
            let body = "WebSocket Chat Server\nConnect to ws://localhost:8080/ws";
            Ok(Response::new(Body::from(body)))
        }
        _ => {
            Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::from("Not Found"))
                .unwrap())
        }
    }
}

async fn handle_websocket(
    upgraded: Upgraded,
    chat_engine: Arc<ChatEngine>,
    remote_addr: std::net::SocketAddr,
) -> Result<()> {
    let ws_stream = WebSocketStream::from_raw_socket(upgraded, hyper_tungstenite::tungstenite::protocol::Role::Server, None).await;
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();
    
    let user_id = Uuid::new_v4().to_string();
    let (message_sender, mut message_receiver) = mpsc::unbounded_channel::<ServerMessage>();

    info!("WebSocket connection established for user: {} from {}", user_id, remote_addr);

    // Add connection to chat engine
    chat_engine.add_connection(user_id.clone(), message_sender);

    // Send welcome message
    let welcome_msg = ServerMessage::Welcome {
        user_id: user_id.clone(),
    };
    
    if let Err(e) = ws_sender.send(Message::Text(serde_json::to_string(&welcome_msg)?)).await {
        warn!("Failed to send welcome message: {}", e);
    }

    let user_id_for_sender = user_id.clone();
    let user_id_for_receiver = user_id.clone();
    let chat_engine_for_cleanup = Arc::clone(&chat_engine);

    // Spawn task to handle outgoing messages
    let sender_task = tokio::spawn(async move {
        while let Some(message) = message_receiver.recv().await {
            match serde_json::to_string(&message) {
                Ok(json_str) => {
                    if let Err(e) = ws_sender.send(Message::Text(json_str)).await {
                        error!("Failed to send message to {}: {}", user_id_for_sender, e);
                        break;
                    }
                }
                Err(e) => {
                    error!("Failed to serialize message: {}", e);
                }
            }
        }
    });

    // Handle incoming messages
    let receiver_task = tokio::spawn(async move {
        while let Some(msg_result) = ws_receiver.next().await {
            match msg_result {
                Ok(Message::Text(text)) => {
                    match serde_json::from_str::<ClientMessage>(&text) {
                        Ok(client_msg) => {
                            if let Err(e) = chat_engine.handle_message(&user_id_for_receiver, client_msg).await {
                                error!("Error handling message from {}: {}", user_id_for_receiver, e);
                            }
                        }
                        Err(e) => {
                            warn!("Invalid message from {}: {} - Error: {}", user_id_for_receiver, text, e);
                        }
                    }
                }
                Ok(Message::Close(_)) => {
                    info!("Client {} disconnected", user_id_for_receiver);
                    break;
                }
                Ok(Message::Ping(_data)) => {
                    // Note: We need to handle this in the sender task, but for now we'll skip it
                    // as we don't have access to ws_sender in this scope
                    info!("Received ping from {}", user_id_for_receiver);
                }
                Ok(Message::Pong(_)) => {
                    // Pong received, connection is alive
                    info!("Received pong from {}", user_id_for_receiver);
                }
                Err(e) => {
                    error!("WebSocket error for {}: {}", user_id_for_receiver, e);
                    break;
                }
                _ => {
                    // Handle other message types if needed
                }
            }
        }
    });

    // Wait for either task to complete
    tokio::select! {
        _ = sender_task => {
            info!("Sender task completed for {}", user_id);
        }
        _ = receiver_task => {
            info!("Receiver task completed for {}", user_id);
        }
    }

    // Cleanup: remove connection from chat engine
    chat_engine_for_cleanup.remove_connection(&user_id).await;
    info!("Connection cleanup completed for user {}", user_id);

    Ok(())
}