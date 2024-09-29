use std::collections::HashMap;

use grammers_client::{session::PackedChat, types::Chat};
use grammers_friendly::prelude::*;
use rbatis::RBatis;
use rbdc_sqlite::Driver;

#[derive(Clone)]
pub struct Database {
    conn: RBatis,

    chats_hash: HashMap<i64, PackedChat>,
}

impl Database {
    pub async fn connect() -> Self {
        let conn = RBatis::new();
        conn.init(
            Driver {},
            &std::env::var("DATABASE_URL").expect("DATABASE_URL not set"),
        )
        .unwrap();

        Self {
            conn,
            chats_hash: HashMap::new(),
        }
    }

    pub fn get_conn(&mut self) -> &mut RBatis {
        &mut self.conn
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

impl Module for Database {}

pub trait GetChatById {
    fn get_chat_by_id(&self, id: i64) -> Option<PackedChat>;
}

impl GetChatById for Data {
    fn get_chat_by_id(&self, id: i64) -> Option<PackedChat> {
        self.get_module::<Database>().unwrap().get_chat(id)
    }
}
