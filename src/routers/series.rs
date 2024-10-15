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
                .or(macros::command!("series"))
                .or(macros::command!("/!.", "s")),
        ))
        .add_handler(Handler::callback_query(
            see_serie,
            filters::query("series id:int sender:int index:int"),
        ))
        .add_handler(Handler::new_message(
            see_serie_characters,
            macros::command!("/!.", "si").or(macros::command!("/!.", "oi")),
        ))
        .add_handler(Handler::callback_query(
            see_serie_characters,
            filters::query("series i id:int sender:int index:int"),
        ))
        .add_handler(Handler::callback_query(
            like_series,
            filters::query("slike id:int"),
        ))
        .add_handler(Handler::new_message(
            search_series,
            macros::command!("/!.", "ss"),
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

    let mut splitted = if let Some(ref query) = query {
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
                    &crate::utils::escape_html(format!("{} <title|id>", splitted[0])),
                )))
                .await?;
        }
    } else {
        let conn = db.get_conn();
        let is_like = splitted[0].contains("like");
        let sender_id = sender.id();

        if ["i", "c", "p"].iter().any(|letter| splitted[1] == *letter) {
            return see_serie_characters(client, update, data).await;
        } else if splitted[1].contains("s") {
            return search_series(client, update, data).await;
        } else if let Some(series) = match splitted[1].parse::<i64>() {
            Ok(id) => Series::select_by_id(conn, id).await?,
            Err(_) => {
                splitted[1] = splitted[1..].join(" ");
                splitted.truncate(2);

                Series::select_by_title(conn, &splitted[1]).await?
            }
        } {
            let char_per_page = 15;

            let mut index = 1;
            let total_characters = Character::count_by_series(conn, series.id).await?;
            let total = ((total_characters as f64) / (char_per_page as f64)).ceil() as usize;
            let mut buttons = Vec::new();

            if splitted.len() > 2 {
                if let Ok(user_id) = splitted[2].parse::<i64>() {
                    if user_id != sender_id {
                        return Ok(());
                    }
                }
            }

            if splitted.len() > 3 {
                if let Ok(i) = splitted[3].parse::<i64>() {
                    index = i as usize;
                }
            }

            let mut caption = String::new();

            let characters =
                Character::select_page_by_series(conn, series.id, index as u16, char_per_page)
                    .await?;
            let space_count = characters
                .iter()
                .map(|character| character.id.to_string().len())
                .max()
                .unwrap_or(0);

            for (num, character) in characters.iter().enumerate() {
                if num == 0 {
                    caption = crate::utils::construct_series_info(&series, total_characters);
                }

                caption +=
                    &crate::utils::construct_character_partial_info(&character, false, space_count);
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

            if query.is_some() {
                message.edit(input_message).await?;
            } else {
                if let Some(file) = crate::utils::upload_banner(client, series.clone(), conn)
                    .await
                    .unwrap()
                {
                    message.reply(input_message.photo(file)).await?;
                } else {
                    message.reply(input_message).await?;
                }
            }
        } else {
            message
                .reply(InputMessage::html(t("unknown_series")))
                .await?;
        }
    }

    Ok(())
}

async fn see_serie_characters(
    client: &mut Client,
    update: &mut Update,
    data: &mut Data,
) -> Result<()> {
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

    let mut splitted = if let Some(ref query) = query {
        utils::split_query(query.data())
    } else {
        message
            .text()
            .split_whitespace()
            .map(|part| part.to_string())
            .collect::<Vec<String>>()
    };

    if splitted.len() > 1 {
        if splitted[0].contains("si") || splitted[0].contains("oi") {
            splitted.insert(1, "i".to_string());
        }

        let conn = db.get_conn();
        let sender_id = sender.id();

        if splitted.len() <= 2 {
            message
                .reply(InputMessage::html(t("invalid_command").replace(
                    "{cmd}",
                    &crate::utils::escape_html(format!("{} i <title|id>", splitted[0])),
                )))
                .await?;

            return Ok(());
        }

        if let Some(series) = match splitted[2].parse::<i64>() {
            Ok(id) => Series::select_by_id(conn, id).await?,
            Err(_) => {
                splitted[2] = splitted[2..].join(" ");
                splitted.truncate(3);

                Series::select_by_title(conn, &splitted[2]).await?
            }
        } {
            let mut file = None;
            let mut index = 1;
            let total = Character::count_by_series(conn, series.id).await?;
            let mut buttons = Vec::new();

            if splitted.len() > 3 {
                if let Ok(user_id) = splitted[3].parse::<i64>() {
                    if user_id != sender_id {
                        return Ok(());
                    }
                }
            }

            if splitted.len() > 4 {
                if let Ok(i) = splitted[4].parse::<i64>() {
                    index = i as usize;
                }
            }

            let mut caption = String::new();

            let characters =
                Character::select_page_by_series(conn, series.id, index as u16, 1).await?;
            for character in characters.iter() {
                file = crate::utils::upload_photo(client, character.clone(), conn).await?;

                caption += &(crate::utils::construct_character_partial_info(&character, true, 0)
                    + &crate::utils::construct_series_info(&series, 0));
            }

            caption += &format!("🔖 | {}/{}", index, total);

            if index > 1 {
                if index > 2 {
                    buttons.push(button::inline(
                        "⏪",
                        format!("series i {0} {1} {2}", series.id, sender_id, 1),
                    ));
                }

                buttons.push(button::inline(
                    "⬅",
                    format!("series i {0} {1} {2}", series.id, sender_id, index - 1),
                ));
            }
            if index < total {
                buttons.push(button::inline(
                    "➡",
                    format!("series i {0} {1} {2}", series.id, sender_id, index + 1),
                ));

                if index < total - 1 {
                    buttons.push(button::inline(
                        "⏩",
                        format!("series i {0} {1} {2}", series.id, sender_id, total),
                    ));
                }
            }

            let mut input_message = InputMessage::html(caption);

            if !buttons.is_empty() {
                input_message = input_message.reply_markup(&reply_markup::inline(vec![buttons]));
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
    } else {
        message
            .reply(InputMessage::html(t("invalid_command").replace(
                "{cmd}",
                &crate::utils::escape_html(format!("{} <title|id>", splitted[0])),
            )))
            .await?;
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

async fn search_series(_client: &mut Client, update: &mut Update, data: &mut Data) -> Result<()> {
    let mut db = data.get_module::<Database>().unwrap();
    let i18n = data.get_module::<I18n>().unwrap();

    let t = |key| i18n.get(key);
    let conn = db.get_conn();

    let message = update.get_message().unwrap();

    let mut splitted = message.text().split_whitespace().collect::<Vec<&str>>();

    if splitted.len() > 1 {
        if splitted[0].contains("ss") {
            splitted.insert(1, "s");
        }

        if splitted.len() <= 2 {
            message
                .reply(InputMessage::html(t("invalid_command").replace(
                    "{cmd}",
                    &crate::utils::escape_html(format!("{} s <title>", splitted[0])),
                )))
                .await?;

            return Ok(());
        }

        let title = splitted[2..].join(" ");
        let mut text = t("search_results").replace("{search}", &title) + "\n";

        let series = Series::select_page_by_title(conn, &title, 1, 15).await?;
        let space_count = series
            .iter()
            .map(|series| series.id.to_string().len())
            .max()
            .unwrap_or(0);

        for series in series.iter() {
            let character_id_length = series.id.to_string().len();

            text += &format!(
                "\n{0} <code>{1}</code><code>{2}</code>. <b>{3}</b>",
                crate::utils::media_type_symbol(&series.media_type),
                if space_count > character_id_length {
                    " ".repeat(space_count - character_id_length)
                } else {
                    String::new()
                },
                series.id,
                series.title
            );
        }

        message.reply(InputMessage::html(text)).await?;
    } else {
        message
            .reply(InputMessage::html(t("invalid_command").replace(
                "{cmd}",
                &crate::utils::escape_html(format!("{} <title>", splitted[0])),
            )))
            .await?;
    }

    Ok(())
}
