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

async fn add_series(client: &mut Client, update: &mut Update, data: &mut Data) -> Result<()> {
    let mut db = data.get_module::<Database>().unwrap();
    let i18n = data.get_module::<I18n>().unwrap();
    let conv = data.get_module::<Conversation>().unwrap();

    let t = |key| i18n.get(key);

    let chat = update.get_chat().unwrap();
    let query = update.get_query().unwrap();
    let sender = query.sender();
    let message = query.load_message().await?;
    let mut timeout = 15;

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
            let mut series = Series {
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
                    InputMessage::html(crate::utils::construct_series_info(&series, 0, false))
                        .reply_markup(&reply_markup::inline(vec![vec![button::inline(
                            t("continue_button"),
                            format!("series edit {}", series.id),
                        )]])),
                )
                .await?;

            let field = t("banner");
            timeout = 30;

            match conv
                .ask_photo(
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
                    let photo = response.photo().unwrap();
                    let bytes = crate::utils::download_tele_photo(client, photo).await?;

                    series.banner = Some(bytes.clone());
                    match Series::update_by_id(conn, &series, series.id).await {
                        Ok(_) => {
                            sent.edit(InputMessage::html(
                                t("field_updated").replace("{field}", &field.to_lowercase()),
                            ))
                            .await?;
                        }
                        Err(_) => {
                            sent.edit(InputMessage::html(
                                t("error_occurred").replace("{field}", &field.to_lowercase()),
                            ))
                            .await?;
                        }
                    }

                    let mut stream = Cursor::new(&bytes);
                    let file = client
                        .upload_stream(&mut stream, bytes.len(), format!("char_{}.jpg", series.id))
                        .await?;

                    tokio::time::sleep(Duration::from_secs(2)).await;
                    sent.delete().await?;
                    let _ = response.delete().await;
                    // if message.refetch().await.is_ok() {
                    message.delete().await?;
                    message
                        .reply(
                            InputMessage::html(
                                message.html_text()
                                    + &format!("<a href='tg://user?id={}'>ㅤ</a>", sender.id()),
                            )
                            .photo(file)
                            .reply_markup(&reply_markup::inline(vec![vec![button::inline(
                                t("continue_button"),
                                format!("series edit {}", series.id),
                            )]])),
                        )
                        .await?;
                    // }
                }
                (sent, None) => {
                    sent.edit(InputMessage::html(
                        t("operation_cancelled").replace("{reason}", &t("timeout")),
                    ))
                    .await?;

                    Series::delete_by_id(conn, series.id).await?;

                    tokio::time::sleep(Duration::from_secs(2)).await;
                    sent.delete().await?;
                }
            }
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

async fn edit_series(client: &mut Client, update: &mut Update, data: &mut Data) -> Result<()> {
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

        let mut file = None;
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
                    "artist" => {
                        if splitted.len() == 5 {
                            match splitted[4].as_str() {
                                "name" => {
                                    let field = t("artist_name");
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
                                            let title = response.text();

                                            series.artist = title.trim().to_string();

                                            match Series::update_by_id(conn, &series, series_id)
                                                .await
                                            {
                                                Ok(_) => {
                                                    sent.edit(InputMessage::html(
                                                        t("field_updated").replace(
                                                            "{field}",
                                                            &field.to_lowercase(),
                                                        ),
                                                    ))
                                                    .await?;
                                                }
                                                Err(_) => {
                                                    sent.edit(InputMessage::html(
                                                        t("error_occurred").replace(
                                                            "{field}",
                                                            &field.to_lowercase(),
                                                        ),
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
                                                t("operation_cancelled")
                                                    .replace("{reason}", &t("timeout")),
                                            ))
                                            .await?;

                                            tokio::time::sleep(Duration::from_secs(2)).await;
                                            sent.delete().await?;

                                            return Ok(());
                                        }
                                    }
                                }
                                "link" => {
                                    let field = t("image_link");
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
                                            let link = response.text();

                                            series.image_link = link.trim().to_string();

                                            match Series::update_by_id(conn, &series, series_id)
                                                .await
                                            {
                                                Ok(_) => {
                                                    sent.edit(InputMessage::html(
                                                        t("field_updated").replace(
                                                            "{field}",
                                                            &field.to_lowercase(),
                                                        ),
                                                    ))
                                                    .await?;
                                                }
                                                Err(_) => {
                                                    sent.edit(InputMessage::html(
                                                        t("error_occurred").replace(
                                                            "{field}",
                                                            &field.to_lowercase(),
                                                        ),
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
                                                t("operation_cancelled")
                                                    .replace("{reason}", &t("timeout")),
                                            ))
                                            .await?;

                                            tokio::time::sleep(Duration::from_secs(2)).await;
                                            sent.delete().await?;

                                            return Ok(());
                                        }
                                    }
                                }
                                _ => {}
                            }
                        } else {
                            message
                                .edit(InputMessage::html(t("select_button")).reply_markup(
                                    &reply_markup::inline(vec![
                                        vec![
                                            button::inline(
                                                t("name") + " ✏",
                                                format!("series edit {} artist name", series_id),
                                            ),
                                            button::inline(
                                                t("link") + " ✏",
                                                format!("series edit {} artist link", series_id),
                                            ),
                                        ],
                                        vec![button::inline(
                                            t("back_button"),
                                            format!("series edit {}", series_id),
                                        )],
                                    ]),
                                ))
                                .await?;

                            return Ok(());
                        }
                    }
                    "banner" => {
                        let field = t("banner");
                        let timeout = 30;

                        match conv
                            .ask_photo(
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
                                let photo = response.photo().unwrap();
                                let bytes =
                                    crate::utils::download_tele_photo(client, photo).await?;

                                series.banner = Some(bytes.clone());

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

                                let mut stream = Cursor::new(&bytes);
                                file = Some(
                                    client
                                        .upload_stream(
                                            &mut stream,
                                            bytes.len(),
                                            format!("char_{}.jpg", series_id),
                                        )
                                        .await?,
                                );

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

            let fields = vec!["title", "artist", "media_type", "banner"];
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

            let mut input_message =
                InputMessage::html(crate::utils::construct_series_info(&series, 0, true));

            if let Some(file) = file {
                input_message = input_message.photo(file);
            }

            message
                .edit(input_message.reply_markup(&reply_markup::inline(buttons)))
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
