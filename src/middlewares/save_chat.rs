use grammers_client::{types::Chat, Client, Update};
use grammers_friendly::prelude::*;
use rbatis::async_trait;

use crate::{
    database::{Group, User},
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

            match chat {
                Chat::User(user) => {
                    if user.is_self() || user.is_bot() {
                        return Ok(());
                    }

                    if User::select_by_id(&db.get_conn(), user.id())
                        .await
                        .ok()
                        .unwrap()
                        .is_none()
                    {
                        let u = User {
                            id: user.id(),
                            username: user.username().map(String::from),
                            first_name: user.first_name().to_string(),
                            last_name: user.last_name().map(String::from),
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
                        User::insert(&db.get_conn(), &u).await?;
                    }
                }
                Chat::Group(group) => {
                    if Group::select_by_id(&db.get_conn(), group.id())
                        .await
                        .ok()
                        .unwrap()
                        .is_none()
                    {
                        let g = Group {
                            id: group.id(),
                            username: group.username().map(String::from),
                            language_code: "en-GB".to_string(),
                        };
                        Group::insert(&db.get_conn(), &g).await?;
                    }
                }
                Chat::Channel(_) => {}
            }
        }

        if let Some(sender) = sender {
            match sender {
                Chat::User(sender) => {
                    if User::select_by_id(&db.get_conn(), sender.id())
                        .await
                        .ok()
                        .unwrap()
                        .is_none()
                    {
                        let u = User {
                            id: sender.id(),
                            username: sender.username().map(String::from),
                            first_name: sender.first_name().to_string(),
                            last_name: sender.last_name().map(String::from),
                            language_code: sender
                                .lang_code()
                                .map(|lang| match lang {
                                    "en" => "en-GB",
                                    "pt" => "pt-BR",
                                    _ => lang,
                                })
                                .unwrap_or("en-GB")
                                .to_string(),
                        };
                        User::insert(&db.get_conn(), &u).await?;
                    }
                }
                Chat::Group(_) => {}
                Chat::Channel(_) => {}
            }
        }

        Ok(())
    }
}
