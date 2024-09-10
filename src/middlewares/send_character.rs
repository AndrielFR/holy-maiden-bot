use std::{collections::HashMap, ops::Range};

use grammers_client::{types::Chat, Client, InputMessage, Update};
use grammers_friendly::prelude::*;
use rand::{thread_rng, Rng};
use rbatis::async_trait;

use crate::{database::Character, modules::Database, Result};

#[derive(Clone)]
pub struct SendCharacter {
    min_messages: i64,
    max_messages: i64,

    num_messages: HashMap<i64, i64>,

    ani_client: rust_anilist::Client,
}

impl SendCharacter {
    pub fn new(range: Range<i64>) -> Self {
        Self {
            min_messages: range.start,
            max_messages: range.end,

            num_messages: HashMap::new(),

            ani_client: rust_anilist::Client::default(),
        }
    }
}

#[async_trait]
impl MiddlewareImpl for SendCharacter {
    async fn call(&mut self, _client: &mut Client, update: &mut Update, data: &mut Data) -> Result {
        let db = data.get_module::<Database>().unwrap();

        let chat = update.get_chat();
        let message = update.get_message();

        if let Some(message) = message {
            if let Some(chat) = chat {
                match chat {
                    Chat::User(_) => {
                        return Ok(());
                    }
                    Chat::Group(group) => {
                        let chat_id = group.id();

                        let num_messages = self.num_messages.entry(chat_id).or_insert(0);
                        *num_messages += 1;

                        let num_needed =
                            thread_rng().gen_range(self.min_messages..self.max_messages);
                        if *num_messages >= num_needed {
                            *num_messages = 0;
                            if let Some(char) = Character::select_random(&db.get_conn()).await? {
                                if let Ok(char_ani) = self
                                    .ani_client
                                    .get_char(serde_json::json!({"id": char.anilist_id}))
                                    .await
                                {
                                    message
                                        .reply(
                                            InputMessage::html(char_ani.description)
                                                .photo_url(char_ani.image.medium),
                                        )
                                        .await?;
                                }
                            }
                        }
                    }
                    Chat::Channel(_) => {}
                }
            }
        }

        Ok(())
    }
}
