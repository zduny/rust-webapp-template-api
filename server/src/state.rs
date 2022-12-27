use std::{
    collections::HashMap,
    sync::atomic::{AtomicUsize, Ordering},
};

use tokio::sync::broadcast::{self, Sender};

static NEXT_USER_ID: AtomicUsize = AtomicUsize::new(1);

#[derive(Debug)]
pub struct User {
    pub id: usize,
    pub name: String,
}

impl User {
    pub fn new() -> Self {
        let id = NEXT_USER_ID.fetch_add(1, Ordering::Relaxed);
        User {
            id,
            name: format!("User {id}"),
        }
    }
}

#[derive(Debug)]
pub struct State {
    pub users: HashMap<usize, User>,
    pub message_sender: Sender<(String, String)>,
    pub connected_sender: Sender<String>,
    pub disconnected_sender: Sender<String>,
}

impl State {
    pub fn new() -> Self {
        State {
            users: HashMap::new(),
            message_sender: broadcast::channel(10).0,
            connected_sender: broadcast::channel(10).0,
            disconnected_sender: broadcast::channel(10).0,
        }
    }

    /// Add new user and return reference to it.
    pub fn add_user(&mut self) -> &User {
        let user = User::new();
        let id = user.id;
        self.users.insert(user.id, user);
        &self.users[&id]
    }
}
