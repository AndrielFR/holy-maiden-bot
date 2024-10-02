use async_trait::async_trait;
use grammers_client::{types::Chat, Client, Update};
use grammers_friendly::prelude::*;

use crate::{
    database::models::{Group, User},
    modules::Database,
    Result,
};

#[derive(Clone)]
pub struct SaveChat;

#[async_trait]
impl MiddlewareImpl for SaveChat {
    async fn call(
        &mut self,
        _client: &mut Client,
        update: &mut Update,
        data: &mut Data,
    ) -> Result<()> {
        let chat = update.get_chat();
        let sender = update.get_sender();

        let mut db = data.get_module::<Database>().unwrap();

        if let Some(chat) = chat {
            db.save_chat(chat.clone());

            let conn = db.get_conn();

            if let Chat::Group(group) = chat {
                if Group::select_by_id(conn, group.id()).await?.is_none() {
                    let g = Group {
                        id: group.id(),
                        title: group.title().to_string(),
                        username: group.username().map(String::from),
                        language_code: "en-GB".to_string(),
                    };
                    Group::insert(conn, &g).await?;
                }
            }
        }

        if let Some(sender) = sender {
            let conn = db.get_conn();

            if let Chat::User(user) = sender {
                if User::select_by_id(conn, user.id()).await?.is_none() {
                    let u = User {
                        id: user.id(),
                        username: user.username().map(String::from),
                        full_name: user.full_name(),
                        language_code: user
                            .lang_code()
                            .map(|lang| match lang {
                                "en" => "en-GB",
                                "pt" => "pt-BR",
                                _ => lang,
                            })
                            .unwrap_or("en-GB")
                            .to_string(),
                    };
                    User::insert(conn, &u).await?;
                }
            }
        }

        Ok(())
    }
}
