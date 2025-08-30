// WebSocket connection
let ws = null;
let currentRoom = null;
let username = null;
let users = [];
let reconnectAttempts = 0;
const maxReconnectAttempts = 5;

// DOM elements
const elements = {
    status: document.getElementById('status'),
    joinForm: document.getElementById('joinForm'),
    chatContainer: document.getElementById('chatContainer'),
    usernameInput: document.getElementById('usernameInput'),
    roomInput: document.getElementById('roomInput'),
    messages: document.getElementById('messages'),
    messageInput: document.getElementById('messageInput'),
    usersList: document.getElementById('usersList'),
    roomTitle: document.getElementById('roomTitle'),
    userCount: document.getElementById('userCount'),
    toast: document.getElementById('toast')
};

// Initialize the app
function init() {
    connect();
    setupEventListeners();
    
    // Try to get stored username
    const storedUsername = localStorage.getItem('chatUsername');
    if (storedUsername) {
        elements.usernameInput.value = storedUsername;
    }
}

// Setup event listeners
function setupEventListeners() {
    elements.messageInput.addEventListener('keypress', (e) => {
        if (e.key === 'Enter') {
            sendMessage();
        }
    });

    elements.usernameInput.addEventListener('keypress', (e) => {
        if (e.key === 'Enter') {
            elements.roomInput.focus();
        }
    });

    elements.roomInput.addEventListener('keypress', (e) => {
        if (e.key === 'Enter') {
            joinRoom();
        }
    });

    // Focus on username input when page loads
    elements.usernameInput.focus();
}

// Connect to WebSocket server
function connect() {
    const wsUrl = `ws://localhost:8080/ws`;
    console.log('Connecting to:', wsUrl);
    
    ws = new WebSocket(wsUrl);
    
    ws.onopen = () => {
        console.log('Connected to server');
        updateStatus('🟢 Connected', 'connected');
        reconnectAttempts = 0;
        hideToast();
    };
    
    ws.onmessage = (event) => {
        try {
            const message = JSON.parse(event.data);
            handleMessage(message);
        } catch (error) {
            console.error('Failed to parse message:', error, event.data);
        }
    };
    
    ws.onclose = (event) => {
        console.log('Disconnected from server', event);
        updateStatus('🔴 Disconnected', 'disconnected');
        
        if (currentRoom) {
            showToast('Connection lost. Attempting to reconnect...', 'error');
        }
        
        // Auto-reconnect
        if (reconnectAttempts < maxReconnectAttempts) {
            setTimeout(() => {
                reconnectAttempts++;
                console.log(`Reconnection attempt ${reconnectAttempts}/${maxReconnectAttempts}`);
                connect();
            }, 2000 * reconnectAttempts);
        } else {
            showToast('Unable to reconnect. Please refresh the page.', 'error', 0);
        }
    };
    
    ws.onerror = (error) => {
        console.error('WebSocket error:', error);
        updateStatus('🔴 Connection Error', 'error');
    };
}

// Handle incoming messages
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
            addSystemMessage(`🎉 Joined room: ${message.data.room}`);
            elements.roomTitle.textContent = `Room: ${message.data.room}`;
            showToast(`Joined room: ${message.data.room}`, 'success');
            break;
            
        case 'RoomLeft':
            showJoinForm();
            addSystemMessage(`👋 Left room: ${message.data.room}`);
            showToast(`Left room: ${message.data.room}`, 'success');
            currentRoom = null;
            users = [];
            break;
            
        case 'NewMessage':
            addMessage(
                message.data.user, 
                message.data.message, 
                new Date(message.data.timestamp)
            );
            break;
            
        case 'UserJoined':
            if (!users.includes(message.data.user)) {
                users.push(message.data.user);
                updateUsersList();
            }
            addSystemMessage(`✨ ${message.data.user} joined the room`);
            break;
            
        case 'UserLeft':
            users = users.filter(u => u !== message.data.user);
            updateUsersList();
            addSystemMessage(`💨 ${message.data.user} left the room`);
            break;
            
        case 'Error':
            console.error('Server error:', message.data.message);
            showToast(`Error: ${message.data.message}`, 'error');
            break;
            
        case 'Pong':
            console.log('Received pong');
            break;
            
        default:
            console.warn('Unknown message type:', message.type);
    }
}

// Join a chat room
function joinRoom() {
    const usernameValue = elements.usernameInput.value.trim();
    const roomValue = elements.roomInput.value.trim();
    
    if (!usernameValue || !roomValue) {
        showToast('Please enter both username and room name', 'error');
        return;
    }
    
    if (!ws || ws.readyState !== WebSocket.OPEN) {
        showToast('Not connected to server. Please wait...', 'error');
        return;
    }
    
    username = usernameValue;
    localStorage.setItem('chatUsername', username);
    
    const message = {
        type: 'Join',
        data: { room: roomValue, username: usernameValue }
    };
    
    console.log('Sending:', message);
    ws.send(JSON.stringify(message));
}

// Leave current room
function leaveRoom() {
    if (!currentRoom) return;
    
    if (ws && ws.readyState === WebSocket.OPEN) {
        const message = {
            type: 'Leave',
            data: { room: currentRoom }
        };
        
        console.log('Sending:', message);
        ws.send(JSON.stringify(message));
    }
}

// Send a message
function sendMessage() {
    const messageText = elements.messageInput.value.trim();
    
    if (!messageText || !currentRoom) return;
    
    if (ws && ws.readyState === WebSocket.OPEN) {
        const message = {
            type: 'SendMessage',
            data: { room: currentRoom, message: messageText }
        };
        
        console.log('Sending:', message);
        ws.send(JSON.stringify(message));
        elements.messageInput.value = '';
    } else {
        showToast('Not connected to server', 'error');
    }
}

// Add a chat message to the display
function addMessage(user, message, timestamp) {
    const messagesContainer = elements.messages;
    const messageDiv = document.createElement('div');
    messageDiv.className = 'message';
    
    const time = timestamp.toLocaleTimeString([], {hour: '2-digit', minute:'2-digit'});
    const isCurrentUser = user === username;
    
    messageDiv.innerHTML = `
        <div class="message-header">
            <span class="timestamp">${time}</span>
            <span class="username ${isCurrentUser ? 'current-user' : ''}">${escapeHtml(user)}:</span>
        </div>
        <div class="message-content">${escapeHtml(message)}</div>
    `;
    
    messagesContainer.appendChild(messageDiv);
    messagesContainer.scrollTop = messagesContainer.scrollHeight;
}

// Add a system message
function addSystemMessage(message) {
    const messagesContainer = elements.messages;
    const messageDiv = document.createElement('div');
    messageDiv.className = 'system-message';
    messageDiv.textContent = message;
    
    messagesContainer.appendChild(messageDiv);
    messagesContainer.scrollTop = messagesContainer.scrollHeight;
}

// Update the users list display
function updateUsersList() {
    const usersList = elements.usersList;
    const userCount = elements.userCount;
    
    usersList.innerHTML = '';
    userCount.textContent = `${users.length} user${users.length !== 1 ? 's' : ''}`;
    
    users.forEach(user => {
        const userBadge = document.createElement('div');
        userBadge.className = 'user-badge';
        userBadge.textContent = user;
        
        if (user === username) {
            userBadge.classList.add('current-user');
        }
        
        usersList.appendChild(userBadge);
    });
}

// Show chat container
function showChatContainer() {
    elements.joinForm.style.display = 'none';
    elements.chatContainer.style.display = 'flex';
    elements.messageInput.focus();
}

// Show join form
function showJoinForm() {
    elements.joinForm.style.display = 'flex';
    elements.chatContainer.style.display = 'none';
    elements.messages.innerHTML = '';
    elements.usernameInput.focus();
}

// Update connection status
function updateStatus(text, className = '') {
    elements.status.textContent = text;
    elements.status.className = `connection-status ${className}`;
}

// Show toast notification
function showToast(message, type = 'info', duration = 3000) {
    const toast = elements.toast;
    toast.textContent = message;
    toast.className = `toast ${type}`;
    
    // Force reflow
    toast.offsetHeight;
    
    toast.classList.add('show');
    
    if (duration > 0) {
        setTimeout(hideToast, duration);
    }
}

// Hide toast notification
function hideToast() {
    elements.toast.classList.remove('show');
}

// Escape HTML to prevent XSS
function escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
}

// Send ping to keep connection alive
function sendPing() {
    if (ws && ws.readyState === WebSocket.OPEN) {
        const message = {
            type: 'Ping',
            data: null
        };
        ws.send(JSON.stringify(message));
    }
}

// Initialize the application
document.addEventListener('DOMContentLoaded', init);

// Send ping every 30 seconds to keep connection alive
setInterval(sendPing, 30000);

// Handle page visibility change
document.addEventListener('visibilitychange', () => {
    if (!document.hidden && ws && ws.readyState === WebSocket.CLOSED) {
        console.log('Page became visible, attempting to reconnect...');
        connect();
    }
});