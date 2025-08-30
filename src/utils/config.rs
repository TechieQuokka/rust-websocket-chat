use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub host: String,
    pub port: u16,
    #[allow(dead_code)]
    pub max_connections: usize,
    pub log_level: String,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            host: env::var("CHAT_HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
            port: env::var("CHAT_PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()
                .unwrap_or(8080),
            max_connections: env::var("CHAT_MAX_CONNECTIONS")
                .unwrap_or_else(|_| "1000".to_string())
                .parse()
                .unwrap_or(1000),
            log_level: env::var("CHAT_LOG_LEVEL").unwrap_or_else(|_| "info".to_string()),
        }
    }

    pub fn addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}