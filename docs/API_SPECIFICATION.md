# API Specification

## WebSocket Protocol

The chat application uses WebSocket for real-time, bidirectional communication between clients and server.

### Connection Endpoint
```
ws://localhost:8080/ws
wss://your-domain.com/ws (production)
```

### Message Format

All messages are JSON-formatted with a common structure:

```json
{
  "type": "message_type",
  "data": { /* payload specific to message type */ }
}
```

## Client → Server Messages

### 1. Join Room
Join a chat room and receive existing room information.

```json
{
  "type": "Join",
  "data": {
    "room": "general",
    "username": "alice"
  }
}
```

**Parameters:**
- `room` (string): Room identifier
- `username` (string): Display name for the user

### 2. Leave Room
Leave a chat room.

```json
{
  "type": "Leave", 
  "data": {
    "room": "general"
  }
}
```

**Parameters:**
- `room` (string): Room identifier to leave

### 3. Send Message
Send a message to a room.

```json
{
  "type": "SendMessage",
  "data": {
    "room": "general",
    "message": "Hello everyone!"
  }
}
```

**Parameters:**
- `room` (string): Target room identifier
- `message` (string): Message content (max 1000 characters)

### 4. Ping
Heartbeat message to keep connection alive.

```json
{
  "type": "Ping",
  "data": null
}
```

## Server → Client Messages

### 1. Welcome
Sent immediately after connection is established.

```json
{
  "type": "Welcome",
  "data": {
    "user_id": "user_123456789"
  }
}
```

**Data:**
- `user_id` (string): Unique identifier for this connection

### 2. Room Joined
Confirmation that user joined a room successfully.

```json
{
  "type": "RoomJoined",
  "data": {
    "room": "general",
    "users": ["alice", "bob", "charlie"]
  }
}
```

**Data:**
- `room` (string): Room identifier
- `users` (string[]): List of current users in room

### 3. Room Left
Confirmation that user left a room.

```json
{
  "type": "RoomLeft",
  "data": {
    "room": "general"
  }
}
```

**Data:**
- `room` (string): Room identifier that was left

### 4. New Message
A new message was posted to a room.

```json
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

**Data:**
- `room` (string): Room where message was sent
- `user` (string): Username of sender
- `message` (string): Message content
- `timestamp` (number): Unix timestamp in milliseconds

### 5. User Joined
Notification that a user joined a room.

```json
{
  "type": "UserJoined",
  "data": {
    "room": "general", 
    "user": "dave"
  }
}
```

**Data:**
- `room` (string): Room identifier
- `user` (string): Username of user who joined

### 6. User Left
Notification that a user left a room.

```json
{
  "type": "UserLeft",
  "data": {
    "room": "general",
    "user": "dave"
  }
}
```

**Data:**
- `room` (string): Room identifier
- `user` (string): Username of user who left

### 7. Error
Error response for invalid requests.

```json
{
  "type": "Error",
  "data": {
    "message": "Room name cannot be empty"
  }
}
```

**Data:**
- `message` (string): Human-readable error description

### 8. Pong
Response to ping message.

```json
{
  "type": "Pong",
  "data": null
}
```

## Connection Lifecycle

### 1. Connection Establishment
```
Client                           Server
  │                                │
  │ ── WebSocket Handshake ──────► │
  │ ◄────── Upgrade Response ────── │
  │ ◄────── Welcome Message ────── │
```

### 2. Room Management
```
Client                           Server
  │                                │
  │ ────── Join Message ─────────► │
  │ ◄──── RoomJoined Response ──── │
  │ ◄─── UserJoined (broadcast) ── │ (to other users)
```

### 3. Message Flow
```
Client A                  Server                  Client B
   │                        │                        │
   │ ── SendMessage ──────► │                        │
   │                        │ ── NewMessage ──────► │
   │ ◄── NewMessage ──────── │                        │
```

### 4. Connection Cleanup
```
Client                           Server
  │                                │
  │ ────── Leave Message ─────────► │
  │ ◄──── RoomLeft Response ────── │
  │ ◄─── UserLeft (broadcast) ──── │ (to other users)
  │ ────── Connection Close ──────► │
```

## Error Handling

### Client Errors
- **Invalid JSON**: Connection closed with error
- **Unknown message type**: Error response sent
- **Missing required fields**: Error response sent
- **Invalid room/username**: Error response sent

### Server Errors  
- **Room not found**: Error response
- **User not in room**: Error response  
- **Message too long**: Error response
- **Rate limit exceeded**: Error response

### Connection Errors
- **Network timeout**: Automatic reconnection recommended
- **Server unavailable**: Retry with exponential backoff
- **Authentication failure**: Re-authenticate and reconnect

## Rate Limiting

### Per-Connection Limits
- **Messages**: 10 per second, 100 per minute
- **Room joins**: 5 per minute
- **Connection attempts**: 3 per minute per IP

### Global Limits
- **Total connections**: 10,000 (configurable)
- **Messages per room**: 1,000 per minute
- **New rooms**: 10 per minute

## Security

### Input Validation
- **Message length**: Max 1000 characters
- **Username length**: Max 50 characters
- **Room name**: Max 50 characters, alphanumeric + underscore
- **HTML sanitization**: All user content escaped

### Authentication (Future)
- **JWT tokens**: Bearer token in WebSocket headers
- **Session management**: Token refresh mechanism
- **Permission model**: Room-based access control

## Examples

### Complete Chat Flow

1. **Connect and join room:**
```javascript
const ws = new WebSocket('ws://localhost:8080/ws');

ws.onopen = () => {
  ws.send(JSON.stringify({
    type: 'Join',
    data: { room: 'general', username: 'alice' }
  }));
};
```

2. **Handle incoming messages:**
```javascript
ws.onmessage = (event) => {
  const message = JSON.parse(event.data);
  
  switch (message.type) {
    case 'Welcome':
      console.log('Connected with ID:', message.data.user_id);
      break;
    case 'RoomJoined':
      console.log('Joined room:', message.data.room);
      console.log('Users:', message.data.users);
      break;
    case 'NewMessage':
      console.log(`${message.data.user}: ${message.data.message}`);
      break;
    case 'Error':
      console.error('Error:', message.data.message);
      break;
  }
};
```

3. **Send messages:**
```javascript
function sendMessage(text) {
  ws.send(JSON.stringify({
    type: 'SendMessage',
    data: { room: 'general', message: text }
  }));
}
```

4. **Leave room and disconnect:**
```javascript
function leave() {
  ws.send(JSON.stringify({
    type: 'Leave',
    data: { room: 'general' }
  }));
  ws.close();
}
```