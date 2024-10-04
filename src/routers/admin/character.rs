use std::{io::Cursor, time::Duration};

use bytes::Bytes;
use grammers_client::{
    button, reply_markup,
    types::{photo_sizes::VecExt, Downloadable},
    Client, InputMessage, Update,
};
use grammers_friendly::prelude::*;
use rust_anilist::models::Gender;

use crate::{
    database::models::Character,
    modules::{Anilist, Conversation, Database, I18n},
    Result,
};

pub fn router() -> Router {
    Router::default()
        .add_handler(Handler::new_message(
            see_character,
            macros::command!("char")
                .or(macros::command!("character"))
                .and(crate::filters::sudoers()),
        ))
        .add_handler(Handler::callback_query(
            edit_character,
            filters::query("char edit id:int").and(crate::filters::sudoers()),
        ))
}

async fn see_character(client: &mut Client, update: &mut Update, data: &mut Data) -> Result<()> {
    let mut db = data.get_module::<Database>().unwrap();
    let i18n = data.get_module::<I18n>().unwrap();
    let mut ani = data.get_module::<Anilist>().unwrap();

    let t = |key| i18n.get(key);

    let message = update.get_message().unwrap();

    let splitted = message.text().split_whitespace().collect::<Vec<_>>();

    if splitted.len() <= 1 {
        message
            .reply(InputMessage::html(t("invalid_command").replace(
                "{cmd}",
                &crate::utils::escape_html(format!("{} <id>", splitted.first().unwrap())),
            )))
            .await?;
    } else {
        match splitted[1].parse::<i64>() {
            Ok(character_id) => {
                let conn = db.get_conn();

                if let Some(character) = Character::select_by_id(conn, character_id).await? {
                    if let Some(ani_character) = ani.get_char(character_id).await {
                        let text = t("character_info")
                            .replace("{id}", &ani_character.id.to_string())
                            .replace(
                                "{gender}",
                                match ani_character.gender.unwrap_or(Gender::NonBinary) {
                                    Gender::Male => "💥",
                                    Gender::Female => "🌸",
                                    Gender::NonBinary | Gender::Other(_) => "🍃",
                                },
                            )
                            .replace("{name}", &character.name)
                            .replace(
                                "{bubble}",
                                match character.stars {
                                    1 => "⚪",
                                    2 => "🟢",
                                    3 => "🔵",
                                    4 => "🟣",
                                    5 => "🔴",
                                    _ => "🟡",
                                },
                            );

                        let file =
                            crate::utils::upload_photo(client, character, &mut ani, conn).await?;
                        message
                            .reply(InputMessage::html(text).photo(file).reply_markup(
                                &reply_markup::inline(vec![vec![button::inline(
                                    t("edit_button"),
                                    format!("char edit {}", character_id),
                                )]]),
                            ))
                            .await?;
                    }
                } else {
                    message
                        .reply(InputMessage::html(t("unknown_character")))
                        .await?;
                }
            }
            Err(_) => {
                message
                    .reply(InputMessage::html(
                        t("invalid_id").replace("{id}", splitted[1]),
                    ))
                    .await?;
            }
        }
    }

    Ok(())
}

async fn edit_character(client: &mut Client, update: &mut Update, data: &mut Data) -> Result<()> {
    let mut db = data.get_module::<Database>().unwrap();
    let i18n = data.get_module::<I18n>().unwrap();
    let mut ani = data.get_module::<Anilist>().unwrap();
    let conv = data.get_module::<Conversation>().unwrap();

    let t = |key| i18n.get(key);

    let chat = update.get_chat().unwrap();
    let query = update.get_query().unwrap();
    let message = query.load_message().await?;

    let mut text = message.html_text();
    let splitted = utils::split_query(query.data());
    let mut input_message = InputMessage::html(text.clone());

    if splitted.len() >= 3 {
        let character_id = splitted[2].parse::<i64>().unwrap();

        if splitted.len() == 4 {
            let conn = db.get_conn();

            match splitted[3].as_str() {
                "back" => {
                    message
                        .edit(input_message.reply_markup(&reply_markup::inline(vec![vec![
                            button::inline(t("edit_button"), format!("char edit {}", character_id)),
                        ]])))
                        .await?;

                    return Ok(());
                }
                "name" => {
                    let field = t("name");

                    match conv
                        .ask_message(
                            chat,
                            InputMessage::html(t("ask_field").replace("{field}", &field)),
                            filters::reply().and(crate::filters::sudoers()),
                        )
                        .await
                        .unwrap()
                    {
                        (sent, Some(response)) => {
                            let new_name = response.text();

                            if let Some(mut character) =
                                Character::select_by_id(conn, character_id).await?
                            {
                                text = text.replace(&character.name, &new_name);
                                input_message = InputMessage::html(text);

                                character.name = new_name.to_string();

                                match Character::update_by_id(conn, &character, character_id).await
                                {
                                    Ok(_) => {
                                        sent.edit(InputMessage::html(
                                            t("field_updated")
                                                .replace("{field}", &field.to_lowercase()),
                                        ))
                                        .await?;
                                    }
                                    Err(_) => {
                                        sent.edit(InputMessage::html(
                                            t("error_occurred")
                                                .replace("{field}", &field.to_lowercase()),
                                        ))
                                        .await?;
                                    }
                                }
                            }

                            tokio::time::sleep(Duration::from_secs(4)).await;
                            sent.delete().await?;
                        }
                        (sent, None) => {
                            sent.edit(InputMessage::html(
                                t("operation_cancelled").replace("{reason}", &t("timeout")),
                            ))
                            .await?;

                            tokio::time::sleep(Duration::from_secs(4)).await;
                            sent.delete().await?;

                            return Ok(());
                        }
                    }
                }
                "photo" => {
                    let field = t("photo");

                    match conv
                        .ask_photo(
                            chat,
                            InputMessage::html(t("ask_field").replace("{field}", &field)),
                            filters::reply().and(crate::filters::sudoers()),
                        )
                        .await
                        .unwrap()
                    {
                        (sent, Some(response)) => {
                            let photo = response.photo().unwrap();
                            let thumbs = photo.thumbs();
                            let downloadable =
                                Downloadable::PhotoSize(thumbs.largest().cloned().unwrap());
                            let mut download = client.iter_download(&downloadable);

                            let mut bytes = Vec::new();
                            while let Some(chunk) = download.next().await? {
                                bytes.extend(chunk);
                            }

                            ani.update_image(character_id, Bytes::from(bytes.clone()))
                                .await;

                            if let Some(mut character) =
                                Character::select_by_id(conn, character_id).await?
                            {
                                character.image = Some(bytes.clone());

                                match Character::update_by_id(conn, &character, character_id).await
                                {
                                    Ok(_) => {
                                        sent.edit(InputMessage::html(
                                            t("field_updated")
                                                .replace("{field}", &field.to_lowercase()),
                                        ))
                                        .await?;
                                    }
                                    Err(_) => {
                                        sent.edit(InputMessage::html(
                                            t("error_occurred")
                                                .replace("{field}", &field.to_lowercase()),
                                        ))
                                        .await?;
                                    }
                                }
                            }

                            let mut stream = Cursor::new(&bytes);
                            let file = client
                                .upload_stream(
                                    &mut stream,
                                    bytes.len(),
                                    format!("char_{}.jpg", character_id),
                                )
                                .await?;
                            input_message = input_message.photo(file);

                            tokio::time::sleep(Duration::from_secs(4)).await;
                            sent.delete().await?;
                        }
                        (sent, None) => {
                            sent.edit(InputMessage::html(
                                t("operation_cancelled").replace("{reason}", &t("timeout")),
                            ))
                            .await?;

                            tokio::time::sleep(Duration::from_secs(4)).await;
                            sent.delete().await?;

                            return Ok(());
                        }
                    }
                }
                _ => {}
            }
        }

        let fields = ["name", "photo"];
        let buttons = fields
            .iter()
            .map(|field| {
                button::inline(
                    t(field) + " ✏",
                    format!("char edit {} {}", character_id, field),
                )
            })
            .collect::<Vec<_>>();
        let mut buttons = utils::split_kb_to_columns(buttons, 2);

        buttons.push(vec![button::inline(
            t("back_button"),
            format!("char edit {} back", character_id),
        )]);

        message
            .edit(input_message.reply_markup(&reply_markup::inline(buttons)))
            .await?;
    }

    Ok(())
}
