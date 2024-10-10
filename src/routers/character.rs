use grammers_client::{button, reply_markup, Client, InputMessage, Update};
use grammers_friendly::prelude::*;

use crate::{
    database::models::Character,
    modules::{Database, I18n},
    Result,
};

pub fn router() -> Router {
    Router::default()
        .add_handler(Handler::new_message(
            see_character,
            macros::command!("/!.", "char")
                .or(macros::command!("character"))
                .or(macros::command!("/!.", "c"))
                .or(macros::command!("/!.", "perso"))
                .or(macros::command!("personagem"))
                .or(macros::command!("/!.", "p")),
        ))
        .add_handler(Handler::callback_query(
            like_character,
            filters::query("like id:int"),
        ))
}

async fn see_character(client: &mut Client, update: &mut Update, data: &mut Data) -> Result<()> {
    let mut db = data.get_module::<Database>().unwrap();
    let i18n = data.get_module::<I18n>().unwrap();

    let t = |key| i18n.get(key);

    let query = update.get_query();
    let message = if let Some(ref query) = query {
        query.load_message().await?
    } else {
        update.get_message().unwrap()
    };

    let splitted = if let Some(ref query) = query {
        utils::split_query(query.data())
    } else {
        message
            .text()
            .split_whitespace()
            .map(|part| part.to_string())
            .collect::<Vec<String>>()
    };

    if splitted.len() == 1 {
        if crate::filters::sudoers().is_ok(client, update).await {
            message
                .reply(
                    InputMessage::html(t("select_button")).reply_markup(&reply_markup::inline(
                        vec![vec![
                            button::inline(t("add_button"), format!("char add")),
                            button::inline(t("list_button"), format!("char list 1")),
                        ]],
                    )),
                )
                .await?;
        } else {
            message
                .reply(InputMessage::html(t("invalid_command").replace(
                    "{cmd}",
                    &crate::utils::escape_html(format!("{} <name|id>", splitted[0])),
                )))
                .await?;
        }
    } else {
        let conn = db.get_conn();
        let is_like = splitted[0] == "like";

        if let Some(character_id) = match splitted[1].parse::<i64>() {
            Ok(id) => Some(id),
            Err(_) => {
                if let Some(character) = Character::select_by_name(conn, &splitted[1]).await? {
                    Some(character.id)
                } else {
                    None
                }
            }
        } {
            if let Some(mut character) = Character::select_by_id(conn, character_id).await? {
                let mut buttons = vec![vec![button::inline(
                    format!("❤ {}", character.liked_by.len()),
                    format!("like {}", character_id),
                )]];

                if !is_like && crate::filters::sudoers().is_ok(client, update).await {
                    buttons.push(vec![
                        button::inline(t("edit_button"), format!("char edit {}", character_id)),
                        button::inline(t("delete_button"), format!("char delete {}", character_id)),
                    ]);
                }

                let input_message = InputMessage::html(crate::utils::construct_character_info(
                    t("character_info"),
                    &character,
                    false,
                ))
                .reply_markup(&reply_markup::inline(buttons));

                match {
                    let input_message = input_message.clone();

                    if query.is_some() {
                        message.edit(input_message).await.err()
                    } else {
                        let file = crate::utils::upload_photo(client, character.clone(), conn)
                            .await?
                            .unwrap();
                        message.reply(input_message.photo(file)).await.err()
                    }
                } {
                    Some(e) if e.is("FILE_PARTS_MISSING") || e.is("FILE_PARTS_INVALID") => {
                        character.image = None;
                        Character::update_by_id(conn, &character, character_id).await?;

                        if query.is_some() {
                            message.edit(input_message).await?;
                        } else {
                            message.reply(input_message).await?;
                        }
                    }
                    Some(_) | None => {}
                }
            } else {
                message
                    .reply(InputMessage::html(t("unknown_character")))
                    .await?;
            }
        } else {
            message
                .reply(InputMessage::html(t("unknown_character")))
                .await?;
        }
    }

    Ok(())
}

async fn like_character(client: &mut Client, update: &mut Update, data: &mut Data) -> Result<()> {
    let mut db = data.get_module::<Database>().unwrap();

    let query = update.get_query().unwrap();
    let sender = query.sender();

    let splitted = utils::split_query(query.data());

    match splitted[1].parse::<i64>() {
        Ok(id) => {
            let conn = db.get_conn();
            let sender_id = sender.id();

            if let Some(mut character) = Character::select_by_id(conn, id).await? {
                let mut liked_by = character.liked_by;

                if liked_by.contains(&sender_id) {
                    liked_by.retain(|id| *id != sender_id);
                } else {
                    liked_by.push(sender.id());
                }

                character.liked_by = liked_by;
                match Character::update_by_id(conn, &character, character.id).await {
                    Ok(_) => see_character(client, update, data).await?,
                    Err(_) => {}
                }
            }
        }
        Err(_) => {}
    }

    return Ok(());
}
