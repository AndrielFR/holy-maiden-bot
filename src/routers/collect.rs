use grammers_client::{
    types::{Chat, InputMessage},
    Client, Update,
};
use grammers_friendly::prelude::*;

use crate::{
    database::models::{Character, Group, User},
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
        if let Ok(Some(reply_message)) = message.get_reply().await {
            let conn = db.get_conn();

            if let Some(mut group) = Group::select_by_id(conn, group.id()).await? {
                if let Some(last_message_id) = group.last_character_message_id {
                    // Check if the character match is the same as the user's reply
                    if last_message_id == reply_message.id() {
                        let mut text = t("not_a_character");

                        if let Some(mut character) =
                            Character::select_by_id(conn, group.last_character_id.unwrap()).await?
                        {
                            if character
                                .name
                                .to_lowercase()
                                .contains(&message.text().to_lowercase())
                            {
                                // Check if character is available
                                if character.available == 1 {
                                    let sender = message.sender().unwrap();
                                    if let Some(mut user) =
                                        User::select_by_id(conn, sender.id()).await?
                                    {
                                        let mut owned_characters =
                                            user.owned_characters.unwrap_or_else(Vec::new);

                                        if owned_characters.len() == 9 {
                                            text = t("max_characters");
                                        } else if owned_characters.contains(&character.id) {
                                            text = t("has_character");
                                        } else {
                                            if !owned_characters.contains(&character.id) {
                                                text = t("character_collected")
                                                    .replace("{name}", &character.name);

                                                // Add character to user's collection
                                                owned_characters.push(character.id);
                                                user.owned_characters = Some(owned_characters);
                                                User::update_by_id(conn, &user, user.id).await?;

                                                // Update character availability
                                                character.available = 0;
                                                Character::update_by_id(
                                                    conn,
                                                    &character,
                                                    character.id,
                                                )
                                                .await?;

                                                // Update group last character
                                                group.last_character_id = Some(0);
                                                group.last_character_message_id = Some(0);
                                                Group::update_by_id(conn, &group, group.id).await?;
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
    }

    Ok(())
}
