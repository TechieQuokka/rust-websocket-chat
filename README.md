# 🚀 Lightweight Real-time Chat App in Rust

A high-performance, lightweight real-time chat application built with Rust, focusing on simplicity, safety, and scalability.

## ✨ Features

- **Real-time messaging** with WebSocket communication
- **Multi-room support** with dynamic room creation
- **User presence tracking** and notifications
- **Lightweight architecture** with minimal resource usage
- **Memory safe** implementation using Rust
- **Async/await** for high concurrency
- **Simple web client** included

## 🏗️ Architecture

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   Web Client    │◄──►│  WebSocket       │◄──►│   Chat Engine   │
│   (Browser)     │    │  Gateway         │    │   (Core Logic)  │
└─────────────────┘    └──────────────────┘    └─────────────────┘
                                │                        │
                                ▼                        ▼
                       ┌─────────────────┐    ┌─────────────────┐
                       │   Connection    │    │   Message       │
                       │   Manager       │    │   Storage       │
                       └─────────────────┘    └─────────────────┘
```

### Core Components

- **WebSocket Gateway**: Handle connections and message routing
- **Chat Engine**: Core business logic and message processing  
- **Connection Manager**: Track active connections and user presence
- **Message Storage**: Optional persistence layer

## 🚀 Quick Start

### Prerequisites

- Rust 1.70+ with Cargo
- Modern web browser

### Installation

1. **Clone the repository**:
```bash
git clone <repository-url>
cd communication-project
```

2. **Build the project**:
```bash
cargo build --release
```

3. **Start the server**:
```bash
cargo run
```

4. **Open the web client**:
   - Open `client/index.html` in your browser
   - Or serve it with a local HTTP server:
```bash
cd client
python -m http.server 8000
# Then visit http://localhost:8000
```

### Usage

1. **Connect**: Server starts on `ws://localhost:8080/ws`
2. **Join Room**: Enter username and room name
3. **Chat**: Send messages in real-time
4. **Multiple Users**: Open multiple browser tabs to test

## 📖 Documentation

- [**Architecture Guide**](docs/ARCHITECTURE.md) - System design and components
- [**API Specification**](docs/API_SPECIFICATION.md) - WebSocket protocol details
- [**Implementation Guide**](docs/IMPLEMENTATION_GUIDE.md) - Step-by-step development

## 🛠️ Technology Stack

| Component | Technology | Purpose |
|-----------|------------|---------|
| Runtime | `tokio` | Async runtime and utilities |
| WebSocket | `tokio-tungstenite` | WebSocket server |
| Serialization | `serde` + `serde_json` | Message serialization |
| Logging | `tracing` | Structured logging |
| UUID | `uuid` | User ID generation |
| Concurrency | `dashmap` | Concurrent data structures |

## 📊 Performance Targets

| Metric | Target | Status |
|--------|--------|---------|
| Connection Setup | < 10ms | ✅ |
| Message Latency | < 5ms | ✅ |
| Concurrent Users | 10,000+ | 🎯 |
| Memory Usage | < 50MB | ✅ |
| CPU Usage | < 20% | ✅ |

## 🔧 Configuration

### Environment Variables

```bash
CHAT_PORT=8080              # Server port
CHAT_HOST=0.0.0.0          # Bind address  
CHAT_LOG_LEVEL=info        # Logging level
CHAT_MAX_CONNECTIONS=1000  # Connection limit
```

### Build Features

```bash
# Basic build
cargo build

# Release build with optimizations
cargo build --release

# Development with debug logs
RUST_LOG=debug cargo run
```

## 🧪 Testing

```bash
# Run unit tests
cargo test

# Run with output
cargo test -- --nocapture

# Test specific module
cargo test models::message
```

## 📝 API Examples

### Join a Room
```javascript
ws.send(JSON.stringify({
  type: 'Join',
  data: { room: 'general', username: 'alice' }
}));
```

### Send Message
```javascript
ws.send(JSON.stringify({
  type: 'SendMessage', 
  data: { room: 'general', message: 'Hello everyone!' }
}));
```

### Receive Message
```javascript
// Server sends:
{
  "type": "NewMessage",
  "data": {
    "room": "general",
    "user": "alice", 
    "message": "Hello everyone!",
    "timestamp": 1640995200000
  }
}
```

## 🔒 Security Features

- **Input validation** and sanitization
- **Rate limiting** per connection
- **XSS prevention** in web client
- **Connection limits** to prevent DoS
- **Memory safety** guaranteed by Rust

## 🚀 Deployment

### Single Instance (MVP)
```bash
# Build optimized binary
cargo build --release

# Run in production
./target/release/communication-project
```

### Docker Deployment
```dockerfile
FROM rust:1.70 AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/communication-project /usr/local/bin/
EXPOSE 8080
CMD ["communication-project"]
```

### Load Balancer Setup
```nginx
upstream chat_servers {
    server chat1:8080;
    server chat2:8080;
    server chat3:8080;
}

server {
    listen 80;
    location /ws {
        proxy_pass http://chat_servers;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
    }
}
```

## 🔄 Development Workflow

1. **Make changes** to Rust code
2. **Test locally**: `cargo run`
3. **Run tests**: `cargo test`
4. **Format code**: `cargo fmt`
5. **Check code**: `cargo clippy`
6. **Build release**: `cargo build --release`

## 🛣️ Roadmap

### Phase 1: Core Features ✅
- [x] Real-time messaging
- [x] Multi-room support  
- [x] User presence
- [x] Web client

### Phase 2: Enhancements 🚧
- [ ] Message persistence
- [ ] User authentication
- [ ] Private messaging
- [ ] File sharing

### Phase 3: Scaling 🎯
- [ ] Horizontal scaling
- [ ] Load balancing
- [ ] Redis integration
- [ ] Monitoring

## 🤝 Contributing

1. Fork the repository
2. Create feature branch (`git checkout -b feature/amazing-feature`)
3. Commit changes (`git commit -m 'Add amazing feature'`)
4. Push to branch (`git push origin feature/amazing-feature`)
5. Open Pull Request

### Development Guidelines

- Follow Rust conventions and `cargo fmt`
- Add tests for new features
- Update documentation
- Check with `cargo clippy`

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🆘 Support

- **Documentation**: Check the `docs/` folder
- **Issues**: Open GitHub issues for bugs
- **Discussions**: Use GitHub discussions for questions

## 🎯 Why This Architecture?

### Performance Benefits
- **Zero-copy** message passing where possible
- **Async I/O** for high concurrency
- **Memory efficient** with Rust's ownership model
- **CPU efficient** with minimal allocations

### Scalability Features  
- **Stateless design** ready for horizontal scaling
- **Event-driven** architecture for loose coupling
- **Connection pooling** for resource efficiency
- **Load balancing** preparation

### Safety Guarantees
- **Memory safety** without garbage collection
- **Thread safety** with Rust's type system
- **Error handling** with Result types
- **Resource cleanup** with RAII patterns

---

**Built with ❤️ and Rust 🦀**