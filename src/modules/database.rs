use std::collections::HashMap;

use grammers_client::{session::PackedChat, types::Chat};
use grammers_friendly::prelude::*;
use rbatis::{intercept_log::LogInterceptor, table_sync::SqliteTableMapper, RBatis};
use rbdc_sqlite::Driver;

use crate::database::models::*;

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

        // Set database log level
        conn.get_intercept::<LogInterceptor>()
            .unwrap()
            .set_level_filter(log::LevelFilter::Trace);
        log::logger().flush();

        let mut db = Self {
            conn,
            chats_hash: HashMap::new(),
        };
        db.sync().await;

        db
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

    async fn sync(&mut self) {
        log::info!("syncing database...");

        let character = Character::default();
        let _ = RBatis::sync(&self.conn, &SqliteTableMapper {}, &character, "characters").await;

        let group = Group::default();
        let _ = RBatis::sync(&self.conn, &SqliteTableMapper {}, &group, "groups").await;

        let group_character = GroupCharacter::default();
        let _ = RBatis::sync(
            &self.conn,
            &SqliteTableMapper {},
            &group_character,
            "groups_characters",
        )
        .await;

        let series = Series::default();
        let _ = RBatis::sync(&self.conn, &SqliteTableMapper {}, &series, "series").await;

        let user = User::default();
        let _ = RBatis::sync(&self.conn, &SqliteTableMapper {}, &user, "users").await;

        let user_characters = UserCharacters::default();
        let _ = RBatis::sync(
            &self.conn,
            &SqliteTableMapper {},
            &user_characters,
            "users_characters",
        )
        .await;

        log::info!("database synced");
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
