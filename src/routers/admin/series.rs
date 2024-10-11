use std::{io::Cursor, time::Duration};

use grammers_client::{button, reply_markup, Client, InputMessage, Update};

use grammers_friendly::prelude::*;

use crate::{
    database::models::{Media, Series},
    modules::{Conversation, Database, I18n},
    Result,
};

pub fn router() -> Router {
    Router::default()
        .add_handler(Handler::callback_query(
            add_series,
            filters::query("series add").and(crate::filters::sudoers()),
        ))
        .add_handler(Handler::callback_query(
            delete_series,
            filters::query("series delete id:int").and(crate::filters::sudoers()),
        ))
        .add_handler(Handler::callback_query(
            edit_series,
            filters::query("series edit id:int").and(crate::filters::sudoers()),
        ))
        .add_handler(Handler::callback_query(
            list_series,
            filters::query("series list page:int").and(crate::filters::sudoers()),
        ))
}

async fn add_series(_client: &mut Client, update: &mut Update, data: &mut Data) -> Result<()> {
    let mut db = data.get_module::<Database>().unwrap();
    let i18n = data.get_module::<I18n>().unwrap();
    let conv = data.get_module::<Conversation>().unwrap();

    let t = |key| i18n.get(key);

    let chat = update.get_chat().unwrap();
    let query = update.get_query().unwrap();
    let sender = query.sender();
    let message = query.load_message().await?;
    let timeout = 15;

    match conv
        .ask_message(
            chat.clone(),
            sender,
            InputMessage::html(
                t("ask_field")
                    .replace("{field}", &t("title"))
                    .replace("{timeout}", &timeout.to_string()),
            ),
            crate::filters::sudoers(),
            Duration::from_secs(timeout),
        )
        .await
        .unwrap()
    {
        (sent, Some(response)) => {
            let conn = db.get_conn();

            let last_id = Series::select_last(conn).await?.map_or(0, |serie| serie.id);

            let title = response.text();
            let series = Series {
                id: last_id + 1,
                title: title.to_string(),
                ..Default::default()
            };
            Series::insert(conn, &series).await?;

            sent.edit(InputMessage::html(
                t("object_created").replace("{object}", &t("series")),
            ))
            .await?;

            tokio::time::sleep(Duration::from_secs(2)).await;
            sent.delete().await?;
            let _ = response.delete().await;

            message
                .edit(
                    InputMessage::html(crate::utils::construct_series_info(&series, 0))
                        .reply_markup(&reply_markup::inline(vec![vec![button::inline(
                            t("continue_button"),
                            format!("series edit {}", series.id),
                        )]])),
                )
                .await?;
        }
        (sent, None) => {
            sent.edit(InputMessage::html(
                t("operation_cancelled").replace("{reason}", &t("timeout")),
            ))
            .await?;

            tokio::time::sleep(Duration::from_secs(2)).await;
            sent.delete().await?;
        }
    }

    Ok(())
}

async fn delete_series(_client: &mut Client, update: &mut Update, data: &mut Data) -> Result<()> {
    let mut db = data.get_module::<Database>().unwrap();
    let i18n = data.get_module::<I18n>().unwrap();

    let t = |key| i18n.get(key);

    let query = update.get_query().unwrap();
    let sender = update.get_sender().unwrap();
    let message = query.load_message().await?;

    let splitted = utils::split_query(query.data());

    if splitted.len() >= 3 {
        let conn = db.get_conn();

        let series_id = splitted[2].parse::<i64>().unwrap();

        if splitted.len() == 4 && splitted[3].as_str() == "confirm" {
            if let Some(series) = Series::select_by_id(conn, series_id).await? {
                Series::delete_by_id(conn, series_id).await?;
                message
                    .edit(InputMessage::html(
                        t("object_deleted")
                            .replace("{object}", &t("series"))
                            .replace("{id}", &series.id.to_string()),
                    ))
                    .await?;

                tokio::time::sleep(Duration::from_secs(2)).await;
                let _ = message.delete().await;

                return Ok(());
            } else {
                message
                    .edit(InputMessage::html(t("unknown_character")))
                    .await?;
            }
        }

        message
            .edit(
                InputMessage::html(
                    t("confirm_delete")
                        .replace("{object}", &t("series").to_lowercase())
                        .replace("{id}", &series_id.to_string()),
                )
                .reply_markup(&reply_markup::inline(vec![vec![
                    button::inline(
                        t("cancel_button"),
                        format!("series {0} {1} {2}", series_id, sender.id(), 1),
                    ),
                    button::inline(
                        t("confirm_button"),
                        format!("series delete {} confirm", series_id),
                    ),
                ]])),
            )
            .await?;
    }

    Ok(())
}

async fn edit_series(_client: &mut Client, update: &mut Update, data: &mut Data) -> Result<()> {
    let mut db = data.get_module::<Database>().unwrap();
    let i18n = data.get_module::<I18n>().unwrap();
    let conv = data.get_module::<Conversation>().unwrap();

    let t = |key| i18n.get(key);

    let chat = update.get_chat().unwrap();
    let query = update.get_query().unwrap();
    let sender = query.sender();
    let message = query.load_message().await?;

    let splitted = utils::split_query(query.data());

    if splitted.len() >= 3 {
        let conn = db.get_conn();

        let series_id = splitted[2].parse::<i64>().unwrap();
        if let Some(mut series) = Series::select_by_id(conn, series_id).await? {
            if splitted.len() >= 4 {
                match splitted[3].as_str() {
                    "title" => {
                        let field = t("title");
                        let timeout = 15;

                        match conv
                            .ask_message(
                                chat,
                                sender,
                                InputMessage::html(
                                    t("ask_field")
                                        .replace("{field}", &field)
                                        .replace("{timeout}", &timeout.to_string()),
                                ),
                                crate::filters::sudoers(),
                                Duration::from_secs(timeout),
                            )
                            .await
                            .unwrap()
                        {
                            (sent, Some(response)) => {
                                let new_title = response.text();
                                series.title = new_title.trim().to_string();

                                match Series::update_by_id(conn, &series, series_id).await {
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

                                tokio::time::sleep(Duration::from_secs(2)).await;
                                sent.delete().await?;
                                let _ = response.delete().await;
                            }
                            (sent, None) => {
                                sent.edit(InputMessage::html(
                                    t("operation_cancelled").replace("{reason}", &t("timeout")),
                                ))
                                .await?;

                                tokio::time::sleep(Duration::from_secs(2)).await;
                                sent.delete().await?;

                                return Ok(());
                            }
                        }
                    }
                    "media_type" => {
                        let timeout = 10;

                        let types = vec![
                            "anime",
                            "game",
                            "manga",
                            "manhua",
                            "manhwa",
                            "light_novel",
                            "visual_novel",
                            "unknown",
                        ];
                        let buttons = types
                            .iter()
                            .map(|r#type| (format!("{}_button", r#type), r#type.to_string()))
                            .map(|(text, data)| button::inline(i18n.get(text), data))
                            .collect::<Vec<_>>();
                        let buttons = utils::split_kb_to_columns(buttons, 3);

                        message
                            .edit(
                                InputMessage::html(t("select_button"))
                                    .reply_markup(&reply_markup::inline(buttons)),
                            )
                            .await?;

                        let mut query = types.join("|");
                        query.insert(0, '[');
                        query.push(']');

                        match conv
                            .wait_for_update(
                                sender,
                                filters::query(&query).and(crate::filters::sudoers()),
                                Duration::from_secs(timeout),
                            )
                            .await
                            .unwrap()
                        {
                            Some(update) => {
                                if let Some(query) = update.get_query() {
                                    let splitted = utils::split_query(query.data());

                                    series.media_type = match splitted[0].as_str() {
                                        "anime" => Media::Anime,
                                        "game" => Media::Game,
                                        "manga" => Media::Manga,
                                        "manhua" => Media::Manhua,
                                        "manhwa" => Media::Manhwa,
                                        "light_novel" => Media::LightNovel,
                                        "visual_novel" => Media::VisualNovel,
                                        _ => Media::Unknown,
                                    };

                                    Series::update_by_id(conn, &series, series_id).await?;
                                }
                            }
                            None => {}
                        }
                    }
                    _ => {}
                }
            }

            let fields = vec!["title", "media_type"];
            let buttons = fields
                .into_iter()
                .map(|field| {
                    button::inline(
                        t(field) + " ✏",
                        format!("series edit {} {}", series_id, field),
                    )
                })
                .collect::<Vec<_>>();
            let mut buttons = utils::split_kb_to_columns(buttons, 2);

            buttons.push(vec![button::inline(
                t("back_button"),
                format!("series {0} {1} {2}", series_id, sender.id(), 1),
            )]);

            message
                .edit(
                    InputMessage::html(crate::utils::construct_series_info(&series, 0))
                        .reply_markup(&reply_markup::inline(buttons)),
                )
                .await?;
        }
    }

    Ok(())
}

async fn list_series(client: &mut Client, update: &mut Update, data: &mut Data) -> Result<()> {
    let mut db = data.get_module::<Database>().unwrap();

    let query = update.get_query().unwrap();
    let message = query.load_message().await?;

    let conn = db.get_conn();

    let mut text = String::from("id | title | media type\n");

    let all_series = Series::select_all(conn).await?;
    for series in all_series.iter() {
        text += &format!(
            "\n{0} | {1} | {2}",
            series.id, series.title, series.media_type
        );
    }

    if let Err(_) = message.edit(InputMessage::html(&text)).await {
        let bytes = text.as_bytes();
        let mut stream = Cursor::new(&bytes);

        let file = client
            .upload_stream(&mut stream, bytes.len(), "characters.txt".to_string())
            .await?;

        message
            .reply(InputMessage::html("Lista de personagens").file(file))
            .await?;
    }

    Ok(())
}
