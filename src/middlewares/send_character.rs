use std::{
    collections::{hash_map::Entry, HashMap},
    ops::Range,
};

use grammers_client::{types::Chat, Client, InputMessage, Update};
use grammers_friendly::prelude::*;
use rand::{thread_rng, Rng};
use rbatis::async_trait;

use crate::{
    database::{Character, GroupCharacter},
    modules::{Database, I18n},
    Result,
};

#[derive(Clone)]
pub struct SendCharacter {
    min_messages: i64,
    max_messages: i64,

    characters: HashMap<i64, rust_anilist::models::Character>,
    num_messages: HashMap<i64, i64>,

    ani_client: rust_anilist::Client,
}

impl SendCharacter {
    pub fn new(range: Range<i64>) -> Self {
        Self {
            min_messages: range.start,
            max_messages: range.end,

            characters: HashMap::new(),
            num_messages: HashMap::new(),

            ani_client: rust_anilist::Client::default(),
        }
    }
}

#[async_trait]
impl MiddlewareImpl for SendCharacter {
    async fn call(&mut self, _client: &mut Client, update: &mut Update, data: &mut Data) -> Result {
        let db = data.get_module::<Database>().unwrap();
        let i18n = data.get_module::<I18n>().unwrap();

        let t = |key| i18n.get(key);

        let chat = update.get_chat();
        let message = update.get_message();

        if let Some(message) = message {
            if let Some(Chat::Group(group)) = chat {
                let chat_id = group.id();

                let num_messages = self.num_messages.entry(chat_id).or_insert(0);
                *num_messages += 1;

                let num_needed = thread_rng().gen_range(self.min_messages..self.max_messages);
                if *num_messages >= num_needed {
                    *num_messages = 0;
                    if let Some(char) = Character::select_random(&db.get_conn()).await? {
                        if let Some(char_ani) = {
                            if let Entry::Vacant(e) = self.characters.entry(char.anilist_id) {
                                if let Ok(char_ani) = self
                                    .ani_client
                                    .get_char(serde_json::json!({"id": char.anilist_id}))
                                    .await
                                {
                                    e.insert(char_ani.clone());
                                    Some(char_ani)
                                } else {
                                    None
                                }
                            } else {
                                Some(self.characters.get(&char.anilist_id).unwrap().clone())
                            }
                        } {
                            let mut text = crate::utils::shorten_text(char_ani.description, 380);
                            if text.contains("...") {
                                text += &format!(
                                    "\n\n<a href=\"{0}\">{1}</a>",
                                    char_ani.url,
                                    t("read_more")
                                );
                            }

                            let response = message
                                .respond(
                                    InputMessage::html(text)
                                        .photo_url(char_ani.image.medium)
                                        .invert_media(true),
                                )
                                .await?;

                            let g = GroupCharacter {
                                group_id: chat_id,
                                anilist_id: char_ani.id,
                                message_id: response.id(),
                                character_id: char.id,
                                ..Default::default()
                            };
                            GroupCharacter::insert(&db.get_conn(), &g).await?;
                        }
                    }
                }
            }
        }

        Ok(())
    }
}
