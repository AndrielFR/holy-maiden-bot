use std::{
    collections::{hash_map::Entry, HashMap},
    sync::Arc,
};

use bytes::Bytes;
use grammers_friendly::prelude::*;
use rust_anilist::models::Character;
use tokio::{io::AsyncWriteExt, sync::Mutex};

use crate::Result;

#[derive(Clone, Default)]
pub struct Anilist {
    client: rust_anilist::Client,

    images: Arc<Mutex<HashMap<i64, Bytes>>>,
    characters: Arc<Mutex<HashMap<i64, Character>>>,
}

impl Anilist {
    pub fn new() -> Self {
        Self {
            client: rust_anilist::Client::default().timeout(80),
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
                if let Ok(character) = self.client.get_char(serde_json::json!({"id": id})).await {
                    e.insert(character.clone());

                    Some(character)
                } else {
                    None
                }
            }
        }
    }

    pub async fn get_image(&mut self, id: i64) -> Result<Bytes> {
        let characters = self.characters.lock().await;
        let mut images = self.images.lock().await;

        match images.entry(id) {
            Entry::Occupied(e) => Ok(e.get().clone()),
            Entry::Vacant(e) => {
                if let Some(character) = characters.get(&id) {
                    let file_path = format!(
                        "{}/assets/char_{}.jpg",
                        std::env::current_dir()?.to_str().unwrap(),
                        character.id
                    );

                    let response = reqwest::get(&character.image.large).await?;
                    let mut file = tokio::fs::File::create(&file_path).await?;
                    let content = response.bytes().await?;
                    file.write_all(&content).await?;

                    e.insert(content.clone());
                    tokio::fs::remove_file(file_path).await?;

                    Ok(content)
                } else {
                    Err("Character not found".into())
                }
            }
        }
    }
}

impl Module for Anilist {}
