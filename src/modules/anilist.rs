use std::{
    collections::{hash_map::Entry, HashMap},
    sync::Arc,
};

use grammers_friendly::prelude::*;
use rust_anilist::{models::Character, Client};
use tokio::sync::Mutex;

#[derive(Clone, Default)]
pub struct Anilist {
    client: Client,

    pub characters: Arc<Mutex<HashMap<i64, Character>>>,
}

impl Anilist {
    pub fn new() -> Self {
        Self {
            client: Client::default().timeout(80),
            ..Default::default()
        }
    }

    pub fn char_count(&self) -> usize {
        self.characters.try_lock().unwrap().len()
    }

    pub async fn get_char(&mut self, id: i64) -> Option<Character> {
        let mut characters = self.characters.lock().await;

        match characters.entry(id) {
            Entry::Occupied(e) => Some(e.get().clone()),
            Entry::Vacant(e) => {
                if let Ok(char) = self.client.get_char(serde_json::json!({"id": id})).await {
                    e.insert(char.clone());
                    Some(char)
                } else {
                    None
                }
            }
        }
    }
}

impl Module for Anilist {}
