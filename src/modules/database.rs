use std::collections::HashMap;

use grammers_client::types::Chat;
use grammers_friendly::prelude::*;
use grammers_session::PackedChat;
use rbatis::RBatis;
use rbdc_sqlite::Driver;

const PATH: &str = "./assets/db.sqlite";

#[derive(Clone)]
pub struct Database {
    conn: RBatis,

    chats_hash: HashMap<i64, PackedChat>,
}

impl Database {
    pub async fn connect(&self) {
        self.conn
            .link(Driver {}, &format!("sqlite://{}?mode=rwc", PATH))
            .await
            .unwrap();
    }

    pub fn get_conn(&self) -> RBatis {
        self.conn.clone()
    }

    pub fn get_chat(&self, id: i64) -> Option<PackedChat> {
        self.chats_hash.get(&id).cloned()
    }

    pub fn save_chat(&mut self, chat: Chat) {
        self.chats_hash
            .entry(chat.id())
            .or_insert_with(|| chat.pack());
    }
}

impl Default for Database {
    fn default() -> Self {
        let rb = RBatis::new();

        Self {
            conn: rb,
            chats_hash: HashMap::new(),
        }
    }
}

impl Module for Database {}

pub trait GetChatById {
    fn get_chat_by_id(&self, id: i64) -> Option<PackedChat>;
}

impl GetChatById for Data {
    fn get_chat_by_id(&self, id: i64) -> Option<PackedChat> {
        self.get_module::<Database>().unwrap().get_chat(id)
    }
}
