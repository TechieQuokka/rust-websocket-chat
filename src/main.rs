mod models;
mod server;
mod utils;

use crate::server::{chat_engine::ChatEngine, websocket::WebSocketServer};
use crate::utils::config::Config;
use std::sync::Arc;
use tracing::{info, Level};
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Load configuration
    let config = Config::from_env();

    // Initialize logging
    let log_level = match config.log_level.to_lowercase().as_str() {
        "trace" => Level::TRACE,
        "debug" => Level::DEBUG,
        "info" => Level::INFO,
        "warn" => Level::WARN,
        "error" => Level::ERROR,
        _ => Level::INFO,
    };

    tracing_subscriber::fmt()
        .with_max_level(log_level)
        .with_target(false)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .init();

    info!("🚀 Starting Rust Chat Server");
    info!("Configuration: {:?}", config);

    // Create chat engine
    let chat_engine = Arc::new(ChatEngine::new());
    info!("✅ Chat engine initialized");

    // Create and start WebSocket server
    let ws_server = WebSocketServer::new(chat_engine);
    let addr = config.addr();

    info!("🌐 Server starting on {}", addr);
    info!("📱 Web client available at: client/index.html");
    info!("🔗 WebSocket endpoint: ws://{}/ws", addr);
    
    // Start the server
    if let Err(e) = ws_server.start(&addr).await {
        tracing::error!("❌ Server failed to start: {}", e);
        return Err(e);
    }

    Ok(())
}
