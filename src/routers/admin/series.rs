use std::{io::Cursor, time::Duration};

use grammers_client::{button, reply_markup, Client, InputMessage, Update};

use grammers_friendly::prelude::*;

use crate::{
    database::models::{Character, Media, Series},
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
                    "aliases" => {
                        let buttons = series
                            .aliases
                            .iter()
                            .enumerate()
                            .map(|(index, alias)| {
                                button::inline(
                                    alias,
                                    format!("series edit {0} aliases edit {1}", series_id, index),
                                )
                            })
                            .collect::<Vec<_>>();
                        let mut buttons = utils::split_kb_to_columns(buttons, 1);

                        if splitted.len() >= 5 {
                            match splitted[4].as_str() {
                                "add" => {
                                    if series.aliases.len() >= 5 {
                                        let sent = message
                                            .reply(InputMessage::html(t("max_aliases")))
                                            .await?;
                                        tokio::time::sleep(Duration::from_secs(2)).await;
                                        sent.delete().await?;

                                        return Ok(());
                                    }

                                    let field = t("alias");
                                    let timeout = 10;

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
                                            let alias = response.text();

                                            if alias.len() < 3 {
                                                sent.edit(InputMessage::html(
                                                    t("alias_too_short").replace("{min}", "3"),
                                                ))
                                                .await?;
                                            } else if alias.len() > 15 {
                                                sent.edit(InputMessage::html(
                                                    t("alias_too_long").replace("{max}", "15"),
                                                ))
                                                .await?;
                                            } else {
                                                if series.aliases.iter().any(|a| alias == a) {
                                                    sent.edit(InputMessage::html(t(
                                                        "alias_already_exists",
                                                    )))
                                                    .await?;
                                                } else {
                                                    series.aliases.push(alias.trim().to_string());
                                                    buttons.push(vec![button::inline(
                                                        alias,
                                                        format!(
                                                            "series edit {0} aliases edit {1}",
                                                            series_id,
                                                            series.aliases.len() - 1
                                                        ),
                                                    )]);

                                                    match Series::update_by_id(
                                                        conn, &series, series_id,
                                                    )
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
                                "edit" => {
                                    if let Ok(index) = splitted[5].parse::<usize>() {
                                        if let Some(buttons) = buttons.get_mut(index) {
                                            *buttons = vec![
                                                button::inline(
                                                    t("rename_button"),
                                                    format!(
                                                        "series edit {0} aliases rename {1}",
                                                        series_id, index
                                                    ),
                                                ),
                                                button::inline(
                                                    t("cancel_button"),
                                                    format!("series edit {0} aliases", series_id),
                                                ),
                                                button::inline(
                                                    t("delete_button"),
                                                    format!(
                                                        "series edit {0} aliases delete {1}",
                                                        series_id, index
                                                    ),
                                                ),
                                            ];
                                        };
                                    }
                                }
                                "delete" => {
                                    if let Ok(index) = splitted[5].parse::<usize>() {
                                        if let Some(alias) = series.aliases.get(index) {
                                            if splitted.len() == 7
                                                && splitted[6].as_str() == "confirm"
                                            {
                                                buttons.remove(index);
                                                series.aliases.remove(index);

                                                match Series::update_by_id(conn, &series, series_id)
                                                    .await
                                                {
                                                    Ok(_) => {}
                                                    Err(_) => {
                                                        let sent = message
                                                            .reply(InputMessage::html(
                                                                t("error_occurred").replace(
                                                                    "{field}",
                                                                    &t("aliases").to_lowercase(),
                                                                ),
                                                            ))
                                                            .await?;

                                                        tokio::time::sleep(Duration::from_secs(2))
                                                            .await;
                                                        sent.delete().await?;
                                                    }
                                                }
                                            } else {
                                                message
                                                .edit(
                                                    InputMessage::html(
                                                        t("confirm_delete")
                                                            .replace(
                                                                "{object}",
                                                                &t("alias").to_lowercase(),
                                                            )
                                                            .replace("{id}", alias),
                                                    )
                                                    .reply_markup(&reply_markup::inline(vec![
                                                        vec![
                                                            button::inline(
                                                                t("cancel_button"),
                                                                format!("series edit {0} aliases", series_id),
                                                            ),
                                                            button::inline(
                                                                t("confirm_button"),
                                                                format!("series edit {0} aliases delete {1} confirm", series_id, index),
                                                            ),
                                                        ],
                                                    ])),
                                                )
                                                .await?;

                                                return Ok(());
                                            }
                                        }
                                    }
                                }
                                "rename" => {
                                    if let Ok(index) = splitted[5].parse::<usize>() {
                                        let field = t("alias");
                                        let timeout = 10;

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
                                                let alias = response.text();

                                                if alias.len() < 3 {
                                                    sent.edit(InputMessage::html(
                                                        t("alias_too_short").replace("{min}", "3"),
                                                    ))
                                                    .await?;
                                                } else if alias.len() > 15 {
                                                    sent.edit(InputMessage::html(
                                                        t("alias_too_long").replace("{max}", "15"),
                                                    ))
                                                    .await?;
                                                } else {
                                                    if series.aliases.iter().any(|a| alias == a) {
                                                        sent.edit(InputMessage::html(t(
                                                            "alias_already_exists",
                                                        )))
                                                        .await?;
                                                    } else {
                                                        if let Some(current_alias) =
                                                            series.aliases.get_mut(index)
                                                        {
                                                            *current_alias =
                                                                alias.trim().to_string();
                                                            if let Some(buttons) =
                                                                buttons.get_mut(index)
                                                            {
                                                                *buttons = vec![button::inline(
                                                                alias,
                                                                format!(
                                                                    "series edit {0} aliases edit {1}",
                                                                    series_id, index
                                                                ),
                                                            )];
                                                            }

                                                            match Series::update_by_id(
                                                                conn, &series, series_id,
                                                            )
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
                                                                        t("error_occurred")
                                                                            .replace(
                                                                                "{field}",
                                                                                &field
                                                                                    .to_lowercase(),
                                                                            ),
                                                                    ))
                                                                    .await?;
                                                                }
                                                            }
                                                        }
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
                                }
                                _ => {}
                            }
                        }

                        buttons.extend(vec![
                            vec![button::inline(
                                t("add_button"),
                                format!("series edit {} aliases add", series_id),
                            )],
                            vec![button::inline(
                                t("back_button"),
                                format!("series edit {}", series_id),
                            )],
                        ]);

                        message
                            .edit(
                                InputMessage::html(t("select_button"))
                                    .reply_markup(&reply_markup::inline(buttons)),
                            )
                            .await?;

                        return Ok(());
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
                    "characters" => {
                        let char_per_page = 15;

                        let mut page = 1;
                        let characters_count = Character::count_by_series(conn, series_id).await?;
                        let total_pages =
                            ((characters_count as f64) / (char_per_page as f64)).ceil() as usize;

                        let mut characters;

                        if splitted.len() >= 5 {
                            if let Ok(p) = splitted[4].parse::<usize>() {
                                page = p;
                            } else {
                                if splitted[4].as_str() == "add" {
                                    let timeout = 20;

                                    match conv
                                        .ask_message(
                                            chat,
                                            sender,
                                            InputMessage::html(
                                                t("send_characters_to_add")
                                                    .replace("{title}", &series.title),
                                            ),
                                            crate::filters::sudoers(),
                                            Duration::from_secs(timeout),
                                        )
                                        .await
                                        .unwrap()
                                    {
                                        (sent, Some(response)) => {
                                            let text = response.text().trim();
                                            let mut characters_id = Vec::new();

                                            if text.contains(',') {
                                                text.split(',').for_each(|part| {
                                                    if let Ok(id) = part.trim().parse::<i64>() {
                                                        if !characters_id.contains(&id) {
                                                            characters_id.push(id);
                                                        }
                                                    }
                                                });
                                            } else if text.contains('\n') {
                                                text.split('\n').for_each(|part| {
                                                    if let Ok(id) = part.trim().parse::<i64>() {
                                                        if !characters_id.contains(&id) {
                                                            characters_id.push(id);
                                                        }
                                                    }
                                                });
                                            } else {
                                                text.split_whitespace().for_each(|part| {
                                                    if let Ok(id) = part.trim().parse::<i64>() {
                                                        if !characters_id.contains(&id) {
                                                            characters_id.push(id);
                                                        }
                                                    }
                                                });
                                            }

                                            let mut characters_name = Vec::new();
                                            for character_id in characters_id.iter() {
                                                if let Some(mut character) =
                                                    Character::select_by_id(conn, *character_id)
                                                        .await?
                                                {
                                                    if character.series_id != series_id {
                                                        character.series_id = series_id;
                                                        Character::update_by_id(
                                                            conn,
                                                            &character,
                                                            *character_id,
                                                        )
                                                        .await?;

                                                        characters_name.push(character.name);
                                                    }
                                                }
                                            }

                                            if characters_id.is_empty()
                                                || characters_name.is_empty()
                                            {
                                                message
                                                    .edit(
                                                        InputMessage::html(
                                                            t("no_character_added")
                                                                .replace("{title}", &series.title),
                                                        )
                                                        .reply_markup(&reply_markup::inline(vec![
                                                            vec![button::inline(
                                                                t("continue_button"),
                                                                format!(
                                                                "series edit {0} characters {1}",
                                                                series_id, page
                                                            ),
                                                            )],
                                                        ])),
                                                    )
                                                    .await?;
                                            } else {
                                                message
                                                    .edit(
                                                        InputMessage::html(
                                                            t("characters_added_to_series")
                                                                .replace(
                                                                    "{names}",
                                                                    &characters_name.join(", "),
                                                                )
                                                                .replace("{title}", &series.title),
                                                        )
                                                        .reply_markup(&reply_markup::inline(vec![
                                                            vec![button::inline(
                                                                t("continue_button"),
                                                                format!(
                                                                "series edit {0} characters {1}",
                                                                series_id, page
                                                            ),
                                                            )],
                                                        ])),
                                                    )
                                                    .await?;
                                            }

                                            tokio::time::sleep(Duration::from_secs(2)).await;
                                            sent.delete().await?;
                                            let _ = response.delete().await;

                                            return Ok(());
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
                            }

                            characters = Character::select_page_by_series(
                                conn,
                                series_id,
                                page as u16,
                                char_per_page,
                            )
                            .await?;

                            if splitted.len() >= 6 {
                                match splitted[5].as_str() {
                                    "remove" => {
                                        if splitted.len() >= 7 {
                                            if let Ok(character_id) = splitted[6].parse::<i64>() {
                                                if let Some(mut character) =
                                                    Character::select_by_id(conn, character_id)
                                                        .await?
                                                {
                                                    if splitted.len() >= 8
                                                        && splitted[7].as_str() == "confirm"
                                                    {
                                                        if let Some(index) = characters
                                                            .iter()
                                                            .position(|character| {
                                                                character.id == character_id
                                                            })
                                                        {
                                                            characters.remove(index);
                                                        }

                                                        character.series_id = 0;
                                                        Character::update_by_id(
                                                            conn,
                                                            &character,
                                                            character_id,
                                                        )
                                                        .await?;

                                                        characters =
                                                            Character::select_page_by_series(
                                                                conn,
                                                                series_id,
                                                                page as u16,
                                                                char_per_page,
                                                            )
                                                            .await?;
                                                    } else {
                                                        message
                                                        .edit(InputMessage::html(t(
                                                            "confirm_remove_character_from_series",
                                                        ).replace("{name}", &character.name).replace("{title}", &series.title)).reply_markup(&reply_markup::inline(vec![vec![button::inline(t("cancel_button"), format!("series edit {0} characters {1} delete", series_id, page)), button::inline(t("confirm_button"), format!("series edit {0} characters {1} remove {2} confirm", series_id, page, character.id))]])))
                                                        .await?;

                                                        return Ok(());
                                                    }
                                                }
                                            }
                                        }

                                        let buttons = characters
                                            .iter()
                                            .map(|character| {
                                                button::inline(
                                                    format!(
                                                        "{0}. {1}",
                                                        character.id, character.name
                                                    ),
                                                    format!(
                                                        "series edit {0} characters {1} remove {2}",
                                                        series_id, page, character.id
                                                    ),
                                                )
                                            })
                                            .collect::<Vec<_>>();
                                        let mut buttons = utils::split_kb_to_columns(buttons, 3);
                                        buttons.push(vec![button::inline(
                                            t("cancel_button"),
                                            format!(
                                                "series edit {0} characters {1}",
                                                series_id, page
                                            ),
                                        )]);

                                        message
                                            .edit(
                                                InputMessage::html(t("select_character_to_remove"))
                                                    .reply_markup(&reply_markup::inline(buttons)),
                                            )
                                            .await?;

                                        return Ok(());
                                    }
                                    _ => {}
                                }
                            }
                        } else {
                            characters = Character::select_page_by_series(
                                conn,
                                series_id,
                                page as u16,
                                char_per_page,
                            )
                            .await?;
                        }

                        let mut text =
                            t("series_characters").replace("{title}", &series.title) + "\n\n";

                        let space_count = characters
                            .iter()
                            .map(|character| character.id.to_string().len())
                            .max()
                            .unwrap_or(0);
                        for character in characters.iter() {
                            text += &crate::utils::construct_character_partial_info(
                                character,
                                false,
                                space_count,
                            )
                        }

                        text += &format!("\n🔖 | {}/{}", page, total_pages);

                        let mut buttons = Vec::new();
                        if page > 1 {
                            buttons.push(button::inline(
                                "⬅",
                                format!("series edit {0} characters {1}", series_id, page - 1),
                            ));
                        }
                        if page < total_pages {
                            buttons.push(button::inline(
                                "➡",
                                format!("series edit {0} characters {1}", series_id, page + 1),
                            ));
                        }
                        let mut buttons = utils::split_kb_to_columns(buttons, 2);

                        buttons.extend(vec![
                            vec![
                                button::inline(
                                    t("add_button"),
                                    format!("series edit {} characters add", series_id),
                                ),
                                button::inline(
                                    t("remove_button"),
                                    format!(
                                        "series edit {0} characters {1} remove",
                                        series_id, page
                                    ),
                                ),
                            ],
                            vec![button::inline(
                                t("back_button"),
                                format!("series edit {}", series_id),
                            )],
                        ]);

                        message
                            .edit(
                                InputMessage::html(text)
                                    .reply_markup(&reply_markup::inline(buttons)),
                            )
                            .await?;
                        return Ok(());
                    }
                    _ => {}
                }
            }

            let fields = vec![
                "title",
                "artist",
                "aliases",
                "banner",
                "media_type",
                "characters",
            ];
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
