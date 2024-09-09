use grammers_friendly::prelude::*;
use rbatis::RBatis;
use rbdc_sqlite::Driver;

const PATH: &str = "./assets/db.sqlite";

#[derive(Clone)]
pub struct Database {
    conn: RBatis,
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
}

impl Default for Database {
    fn default() -> Self {
        let rb = RBatis::new();

        Self { conn: rb }
    }
}

impl Module for Database {}
