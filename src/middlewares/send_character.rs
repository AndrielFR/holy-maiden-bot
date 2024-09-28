use std::{
    collections::{hash_map::Entry, HashMap},
    ops::Range,
};

use async_trait::async_trait;
use grammers_client::{types::Chat, Client, InputMessage, Update};
use grammers_friendly::prelude::*;
use rand::{thread_rng, Rng};

use crate::{
    database::models::{Character, Group},
    modules::{Database, I18n},
    Result,
};

#[derive(Clone)]
pub struct SendCharacter {
    min_messages: i64,
    max_messages: i64,

    chats: HashMap<i64, (i64, i64)>,
    characters: HashMap<i64, rust_anilist::models::Character>,

    ani_client: rust_anilist::Client,
}

impl SendCharacter {
    pub fn new(range: Range<i64>) -> Self {
        Self {
            min_messages: range.start,
            max_messages: range.end,

            chats: HashMap::new(),
            characters: HashMap::new(),

            ani_client: rust_anilist::Client::default(),
        }
    }
}

#[async_trait]
impl MiddlewareImpl for SendCharacter {
    async fn call(
        &mut self,
        _client: &mut Client,
        update: &mut Update,
        data: &mut Data,
    ) -> Result<()> {
        let mut db = data.get_module::<Database>().unwrap();
        let i18n = data.get_module::<I18n>().unwrap();

        let t = |key| i18n.get(key);

        let chat = update.get_chat();
        let message = update.get_message();

        if let Some(message) = message {
            if let Some(Chat::Group(group)) = chat {
                let group_id = group.id();

                let (num_messages, num_needed) = self.chats.entry(group_id).or_insert((
                    0,
                    thread_rng().gen_range(self.min_messages..self.max_messages),
                ));
                *num_messages += 1;

                if num_messages >= num_needed {
                    *num_messages = 0;
                    *num_needed = thread_rng().gen_range(self.min_messages..self.max_messages);

                    let conn = db.get_conn();

                    if let Some(mut group) = Group::select_by_id(conn, group_id).await? {
                        if let Some(last_message_id) = group.last_character_message_id {
                            // Check if the character is left behind without anyone collecting it
                            if (message.id() - last_message_id) >= 35 {
                                if let Some(character) =
                                    Character::select_by_id(conn, group.last_character_id.unwrap())
                                        .await?
                                {
                                    // Reset message count
                                    *num_messages = 0;

                                    // Update group last character
                                    group.last_character_id = Some(0);
                                    group.last_character_message_id = Some(0);
                                    Group::update_by_id(conn, &group, group.id).await?;

                                    // Send the reply message
                                    message
                                        .respond(
                                            InputMessage::html(
                                                t("character_escaped")
                                                    .replace("{name}", &character.name),
                                            )
                                            .reply_to(Some(last_message_id)),
                                        )
                                        .await?;

                                    return Ok(());
                                }
                            }
                        }

                        if let Some(random_character) = Character::random(conn).await? {
                            if let Some(character) = match self
                                .characters
                                .entry(random_character.id)
                            {
                                Entry::Occupied(entry) => Some(entry.into_mut()),
                                Entry::Vacant(entry) => {
                                    if let Ok(character) = self
                                        .ani_client
                                        .get_char(serde_json::json!({"id": random_character.id}))
                                        .await
                                    {
                                        Some(entry.insert(character))
                                    } else {
                                        None
                                    }
                                }
                            } {
                                // If the character is the last one, skip it
                                if group.last_character_id == Some(character.id) {
                                    return Ok(());
                                }

                                // Send the character
                                let response = message
                                    .respond(
                                        InputMessage::html(t("new_character"))
                                            .media_ttl(200)
                                            .photo_url(&character.image.large),
                                    )
                                    .await?;

                                // Update group last character
                                group.last_character_id = Some(character.id);
                                group.last_character_message_id = Some(response.id());
                                Group::update_by_id(conn, &group, group.id).await?;
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }
}
