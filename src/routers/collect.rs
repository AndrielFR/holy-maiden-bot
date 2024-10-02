use grammers_client::{
    types::{Chat, InputMessage},
    Client, Update,
};
use grammers_friendly::prelude::*;

use crate::{
    database::models::{Character, GroupCharacter, UserCharacters},
    modules::{Database, I18n},
    Result,
};

pub fn router() -> Router {
    Router::default().add_handler(Handler::new_message(
        collect,
        filters::private().not().and(filters::reply()),
    ))
}

async fn collect(_client: &mut Client, update: &mut Update, data: &mut Data) -> Result<()> {
    let mut db = data.get_module::<Database>().unwrap();
    let i18n = data.get_module::<I18n>().unwrap();

    let t = |key| i18n.get(key);

    let chat = update.get_chat().unwrap();
    let message = update.get_message().unwrap();

    if let Chat::Group(group) = chat {
        let group_id = group.id();

        if let Ok(Some(reply_message)) = message.get_reply().await {
            let conn = db.get_conn();

            if let Some(mut group_character) =
                GroupCharacter::select_last_by_id(conn, group_id).await?
            {
                // Check if the character match is the same as the user's reply
                if group_character.last_message_id == reply_message.id() {
                    let mut text = t("not_a_character");

                    if let Some(character) =
                        Character::select_by_id(conn, group_character.character_id).await?
                    {
                        if message
                            .text()
                            .to_lowercase()
                            .split_whitespace()
                            .find_map(|guess| {
                                if guess.len() > 2 {
                                    for part in character.name.to_lowercase().split_whitespace() {
                                        if guess == part {
                                            return Some(true);
                                        }
                                    }
                                }

                                None
                            })
                            .is_some()
                        {
                            // Check if character is available
                            if group_character.available {
                                let sender = message.sender().unwrap();
                                let user_id = sender.id();

                                if let Some(mut user_characters) =
                                    UserCharacters::select_or_insert_by_id(conn, user_id, group_id)
                                        .await?
                                {
                                    let mut characters = user_characters.characters_id;

                                    if characters.len() == 9 {
                                        text = t("max_characters");
                                    } else if characters.contains(&character.id) {
                                        text = t("has_character");
                                    } else {
                                        if !characters.contains(&character.id) {
                                            text = t("character_collected")
                                                .replace("{name}", &character.name);

                                            // Add character to user's collection
                                            characters.push(character.id);
                                            user_characters.characters_id = characters;
                                            UserCharacters::update_by_id(
                                                conn,
                                                &user_characters,
                                                user_id,
                                                group_id,
                                            )
                                            .await?;

                                            // Update character availability
                                            group_character.available = false;
                                            GroupCharacter::update_by_id(
                                                conn,
                                                &group_character,
                                                group_id,
                                                group_character.character_id,
                                            )
                                            .await?;
                                        }
                                    }
                                }
                            } else {
                                text = t("expired_character");
                            }
                        } else {
                            text = t("wrong_character");
                        }
                    }

                    // Send the reply message
                    message.reply(InputMessage::html(text)).await?;
                }
            }
        }
    }

    Ok(())
}
