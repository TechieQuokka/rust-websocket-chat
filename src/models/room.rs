use dashmap::DashMap;
use std::collections::HashSet;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct User {
    #[allow(dead_code)]
    pub id: String,
    pub username: String,
    #[allow(dead_code)]
    pub rooms: HashSet<String>,
}

#[derive(Debug)]
pub struct Room {
    #[allow(dead_code)]
    pub name: String,
    pub users: HashSet<String>,
    #[allow(dead_code)]
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

    pub fn is_empty(&self) -> bool {
        self.users.is_empty()
    }
}

impl User {
    pub fn new(id: String, username: String) -> Self {
        Self {
            id,
            username,
            rooms: HashSet::new(),
        }
    }

    #[allow(dead_code)]
    pub fn join_room(&mut self, room_name: String) {
        self.rooms.insert(room_name);
    }

    #[allow(dead_code)]
    pub fn leave_room(&mut self, room_name: &str) {
        self.rooms.remove(room_name);
    }
}