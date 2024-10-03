use std::{collections::HashMap, io::Cursor, ops::Range};

use async_trait::async_trait;
use grammers_client::{types::Chat, Client, InputMessage, Update};
use grammers_friendly::prelude::*;
use rand::{thread_rng, Rng};

use crate::{
    database::models::{Character, GroupCharacter},
    modules::{Anilist, Database, I18n},
    Result,
};

#[derive(Clone, Default)]
pub struct SendCharacter {
    min_messages: i64,
    max_messages: i64,

    chats: HashMap<i64, (i64, i64)>,
}

impl SendCharacter {
    pub fn new(range: Range<i64>) -> Self {
        Self {
            min_messages: range.start,
            max_messages: range.end,

            ..Default::default()
        }
    }
}

#[async_trait]
impl MiddlewareImpl for SendCharacter {
    async fn call(
        &mut self,
        client: &mut Client,
        update: &mut Update,
        data: &mut Data,
    ) -> Result<()> {
        let mut db = data.get_module::<Database>().unwrap();
        let i18n = data.get_module::<I18n>().unwrap();
        let mut ani = data.get_module::<Anilist>().unwrap();

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

                    let last_group_character =
                        GroupCharacter::select_last_by_id(conn, group_id).await?;

                    if let Some(ref group_character) = last_group_character {
                        if group_character.available {
                            // Check if the character is left behind without anyone collecting it
                            if (message.id() - group_character.last_message_id) >= 35 {
                                if let Some(character) =
                                    Character::select_by_id(conn, group_character.character_id)
                                        .await?
                                {
                                    // Reset message count
                                    *num_messages = 0;

                                    // Delete group last character
                                    GroupCharacter::delete_by_id(
                                        conn,
                                        group_character.group_id,
                                        group_character.character_id,
                                    )
                                    .await?;

                                    // Send the reply message
                                    message
                                        .respond(
                                            InputMessage::html(
                                                t("character_escaped")
                                                    .replace("{name}", &character.name),
                                            )
                                            .reply_to(Some(group_character.last_message_id)),
                                        )
                                        .await?;

                                    return Ok(());
                                }
                            }
                        }
                    }

                    if let Some(mut random_character) = Character::select_random(conn).await? {
                        if let Some(character) = ani.get_char(random_character.id).await {
                            // If the character is the last one, skip it
                            if let Some(group_character) = last_group_character {
                                if character.id == group_character.character_id {
                                    return Ok(());
                                }
                            }

                            let bytes = random_character.image.unwrap_or({
                                let bytes = ani.get_image(random_character.id).await?.to_vec();

                                // Update character's image bytes
                                random_character.image = Some(bytes.clone());
                                Character::update_by_id(
                                    conn,
                                    &random_character,
                                    random_character.id,
                                )
                                .await?;

                                bytes
                            });
                            let mut stream = Cursor::new(&bytes);
                            let file = client
                                .upload_stream(
                                    &mut stream,
                                    bytes.len(),
                                    format!(
                                        "char_{}-{}.jpg",
                                        random_character.id, random_character.name
                                    ),
                                )
                                .await?;

                            // Send the character
                            let response = message
                                .respond(
                                    InputMessage::html(t("new_character"))
                                        .media_ttl(200)
                                        .photo(file),
                                )
                                .await?;

                            // Update group last character
                            let group_character = GroupCharacter {
                                group_id,
                                character_id: character.id,
                                last_message_id: response.id(),

                                available: true,
                            };
                            GroupCharacter::insert(conn, &group_character).await?;
                        }
                    }
                }
            }
        }

        Ok(())
    }
}
