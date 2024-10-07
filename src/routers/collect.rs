use std::time::Duration;

use grammers_client::{
    button, reply_markup,
    types::{Chat, InputMessage},
    Client, Update,
};
use grammers_friendly::prelude::*;

use crate::{
    database::models::{Character, GroupCharacter, UserCharacters},
    modules::{Conversation, Database, I18n},
    Result,
};

pub fn router() -> Router {
    Router::default().add_handler(Handler::new_message(
        collect_character,
        filters::private().not().and(filters::reply()),
    ))
}

async fn collect_character(
    _client: &mut Client,
    update: &mut Update,
    data: &mut Data,
) -> Result<()> {
    let mut db = data.get_module::<Database>().unwrap();
    let i18n = data.get_module::<I18n>().unwrap();
    let conv = data.get_module::<Conversation>().unwrap();

    let t = |key| i18n.get(key);

    let chat = update.get_chat().unwrap();
    let user = update.get_sender().unwrap();
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
                        let guess = message.text().trim().to_lowercase();
                        let sender = message.sender().unwrap();

                        // Check if character is available
                        if group_character.available {
                            if message.via_bot_id().is_some()
                                || match sender {
                                    Chat::User(ref user) => user.is_bot(),
                                    _ => false,
                                }
                            {
                                // Delete group last character
                                GroupCharacter::delete_by_id(
                                    conn,
                                    group_id,
                                    group_character.character_id,
                                )
                                .await?;

                                message
                                    .reply(InputMessage::html(
                                        t("guess_cheated").replace("{name}", &character.name),
                                    ))
                                    .await?;

                                return Ok(());
                            }

                            if character.name.to_lowercase().trim().contains(&guess) {
                                let user_id = sender.id();

                                if let Some(mut user_characters) =
                                    UserCharacters::select_or_insert_by_id(conn, user_id, group_id)
                                        .await?
                                {
                                    let characters = &mut user_characters.characters_id;

                                    if characters.contains(&character.id) {
                                        text = t("has_character");
                                    } else if characters.len() == 9 {
                                        let timeout = 5;

                                        let sent = message
                                            .reply(
                                                InputMessage::html(
                                                    t("max_characters")
                                                        .replace("{timeout}", &timeout.to_string()),
                                                )
                                                .reply_markup(&reply_markup::inline(vec![vec![
                                                    button::inline(t("yes_button"), "yes"),
                                                    button::inline(t("no_button"), "no"),
                                                ]])),
                                            )
                                            .await?;

                                        match conv
                                            .wait_for_update(
                                                &user,
                                                filters::query("[yes|no]"),
                                                Duration::from_secs(timeout),
                                            )
                                            .await
                                            .unwrap()
                                        {
                                            Some(update) => {
                                                if let Some(query) = update.get_query() {
                                                    let splitted = utils::split_query(query.data());

                                                    match splitted[0].as_str() {
                                                        "yes" => {
                                                            let timeout = 10;

                                                            let buttons = {
                                                                let mut buttons = Vec::new();

                                                                for id in characters.iter() {
                                                                    if let Some(character) =
                                                                        Character::select_by_id(
                                                                            conn, *id,
                                                                        )
                                                                        .await?
                                                                    {
                                                                        buttons.push(
                                                                            button::inline(
                                                                                format!(
                                                                                    "{0}. {1}",
                                                                                    character.id,
                                                                                    character.name
                                                                                ),
                                                                                character
                                                                                    .id
                                                                                    .to_string(),
                                                                            ),
                                                                        );
                                                                    }
                                                                }

                                                                buttons
                                                            };
                                                            let buttons =
                                                                utils::split_kb_to_columns(
                                                                    buttons, 2,
                                                                );

                                                            sent.edit(
                                                                InputMessage::html(
                                                                    t("select_character").replace(
                                                                        "{timeout}",
                                                                        &timeout.to_string(),
                                                                    ),
                                                                )
                                                                .reply_markup(
                                                                    &reply_markup::inline(buttons),
                                                                ),
                                                            )
                                                            .await?;

                                                            let mut query = characters
                                                                .iter()
                                                                .map(|id| id.to_string())
                                                                .collect::<Vec<String>>()
                                                                .join("|");
                                                            query.insert(0, '[');
                                                            query.push(']');

                                                            match conv
                                                                .wait_for_update(
                                                                    &user,
                                                                    filters::query(&query),
                                                                    Duration::from_secs(timeout),
                                                                )
                                                                .await
                                                                .unwrap()
                                                            {
                                                                Some(update) => {
                                                                    if let Some(query) =
                                                                        update.get_query()
                                                                    {
                                                                        let splitted =
                                                                            utils::split_query(
                                                                                query.data(),
                                                                            );

                                                                        if let Ok(id) = splitted[0]
                                                                            .parse::<i64>(
                                                                        ) {
                                                                            for character_id in
                                                                                characters
                                                                                    .iter_mut()
                                                                            {
                                                                                if *character_id
                                                                                    == id
                                                                                {
                                                                                    *character_id =
                                                                                        character.id;
                                                                                }
                                                                            }

                                                                            UserCharacters::update_by_id(
                                                                                conn,
                                                                                &user_characters,
                                                                                user_id,
                                                                                group_id,
                                                                            ).await?;

                                                                            // Update character availability
                                                                            group_character
                                                                                .available = false;
                                                                            GroupCharacter::update_by_id(
                                                                                conn,
                                                                                &group_character,
                                                                                group_id,
                                                                                group_character.character_id,
                                                                            )
                                                                            .await?;

                                                                            if let Some(old_character) =
                                                                                Character::select_by_id(
                                                                                    conn,
                                                                                    id,
                                                                                ).await? {
                                                                                    sent.edit(InputMessage::html(t(
                                                                                        "character_swapped")
                                                                                            .replace("{old}", &old_character.name)
                                                                                            .replace("{new}", &character.name)
                                                                                    )).await?;
                                                                                    }
                                                                        }
                                                                    }
                                                                }
                                                                None => {
                                                                    sent.edit(InputMessage::html(
                                                                        t("timeouted_operation"),
                                                                    ))
                                                                    .await?;
                                                                }
                                                            }
                                                        }
                                                        "no" => {
                                                            sent.delete().await?;
                                                        }
                                                        _ => {}
                                                    }
                                                }
                                            }
                                            None => {
                                                sent.edit(InputMessage::html(t(
                                                    "timeouted_operation",
                                                )))
                                                .await?;
                                            }
                                        }

                                        return Ok(());
                                    } else {
                                        text = t("character_collected")
                                            .replace("{name}", &character.name);

                                        // Add character to user's collection
                                        characters.push(character.id);
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
                            } else {
                                text = t("wrong_character");
                            }
                        } else {
                            text = t("expired_character");
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
