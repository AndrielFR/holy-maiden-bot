use grammers_client::{button, reply_markup, Client, InputMessage, Update};
use grammers_friendly::prelude::*;

use crate::{
    database::models::{Character, Series},
    modules::{Database, I18n},
    Result,
};

pub fn router() -> Router {
    Router::default()
        .add_handler(Handler::new_message(
            see_serie,
            macros::command!("obra")
                .or(macros::command!("/!.", "o"))
                .or(macros::command!("serie"))
                .or(macros::command!("/!.", "s")),
        ))
        .add_handler(Handler::callback_query(
            see_serie,
            filters::query("series id:int sender:int index:int"),
        ))
        .add_handler(Handler::callback_query(
            like_series,
            filters::query("slike id:int"),
        ))
}

async fn see_serie(client: &mut Client, update: &mut Update, data: &mut Data) -> Result<()> {
    let mut db = data.get_module::<Database>().unwrap();
    let i18n = data.get_module::<I18n>().unwrap();

    let t = |key| i18n.get(key);

    let query = update.get_query();
    let sender = update.get_sender().unwrap();
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
                            button::inline(t("add_button"), format!("series add")),
                            button::inline(t("list_button"), format!("series list 1")),
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
        let is_like = splitted[0].contains("like");
        let sender_id = sender.id();

        if let Some(series) = match splitted[1].parse::<i64>() {
            Ok(id) => Series::select_by_id(conn, id).await?,
            Err(_) => {
                if let Some(series) = Series::select_by_name(conn, &splitted[1]).await? {
                    Some(series)
                } else {
                    None
                }
            }
        } {
            let char_per_page = 15;

            let mut file = None;
            let mut index = 1;
            let total = ((Character::select_by_series(conn, series.id).await?.len() as f64)
                / (char_per_page as f64))
                .ceil() as usize;
            let mut buttons = Vec::new();

            if splitted.len() > 2 {
                if let Ok(user_id) = splitted[2].parse::<i64>() {
                    if user_id != sender_id {
                        return Ok(());
                    }
                }

                if let Ok(i) = splitted[3].parse::<i64>() {
                    index = i as usize;
                }
            }

            let mut caption = String::new();

            let characters =
                Character::select_page_by_series(conn, series.id, index as u16, char_per_page)
                    .await?;
            for (num, character) in characters.iter().enumerate() {
                if num == 0 {
                    if !is_like {
                        file = crate::utils::upload_photo(client, character.clone(), conn).await?;
                    }

                    caption = crate::utils::construct_series_info(&series, Some(character));
                } else {
                    caption += &crate::utils::construct_character_partial_info(&character);
                }
            }

            if index > 1 {
                buttons.push(button::inline(
                    "⬅",
                    format!("series {0} {1} {2}", series.id, sender_id, index - 1),
                ));
            }
            if index < total {
                buttons.push(button::inline(
                    "➡",
                    format!("series {0} {1} {2}", series.id, sender_id, index + 1),
                ));
            }
            let mut buttons = vec![
                buttons,
                vec![button::inline(
                    format!("❤ {}", series.liked_by.len()),
                    format!("slike {}", series.id),
                )],
            ];

            if !is_like && crate::filters::sudoers().is_ok(client, update).await {
                buttons.push(vec![
                    button::inline(t("edit_button"), format!("series edit {}", series.id)),
                    button::inline(t("delete_button"), format!("series delete {}", series.id)),
                ]);
            }

            caption += &format!("\n🔖 | {}/{}", index, total);

            let mut input_message = InputMessage::html(caption);

            if !buttons.is_empty() {
                input_message = input_message.reply_markup(&reply_markup::inline(buttons));
            }

            if let Some(file) = file {
                input_message = input_message.photo(file);
            }

            if query.is_some() {
                message.edit(input_message).await?;
            } else {
                message.reply(input_message).await?;
            }
        } else {
            message
                .reply(InputMessage::html(t("unknown_series")))
                .await?;
        }
    }

    Ok(())
}

async fn like_series(client: &mut Client, update: &mut Update, data: &mut Data) -> Result<()> {
    let mut db = data.get_module::<Database>().unwrap();

    let query = update.get_query().unwrap();
    let sender = query.sender();

    let splitted = utils::split_query(query.data());

    match splitted[1].parse::<i64>() {
        Ok(id) => {
            let conn = db.get_conn();
            let sender_id = sender.id();

            if let Some(mut series) = Series::select_by_id(conn, id).await? {
                let mut liked_by = series.liked_by;

                if liked_by.contains(&sender_id) {
                    liked_by.retain(|id| *id != sender_id);
                } else {
                    liked_by.push(sender.id());
                }

                series.liked_by = liked_by;
                match Series::update_by_id(conn, &series, series.id).await {
                    Ok(_) => see_serie(client, update, data).await?,
                    Err(_) => {}
                }
            }
        }
        Err(_) => {}
    }

    return Ok(());
}
