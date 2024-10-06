use std::{io::Cursor, time::Duration};

use grammers_client::{button, reply_markup, Client, InputMessage, Update};
use grammers_friendly::prelude::*;

use crate::{
    database::models::{Character, Gender},
    modules::{Conversation, Database, I18n},
    Result,
};

pub fn router() -> Router {
    Router::default()
        .add_handler(Handler::callback_query(
            add_character,
            filters::query("char add").and(crate::filters::sudoers()),
        ))
        .add_handler(Handler::callback_query(
            delete_character,
            filters::query("char delete id:int").and(crate::filters::sudoers()),
        ))
        .add_handler(Handler::callback_query(
            edit_character,
            filters::query("char edit id:int").and(crate::filters::sudoers()),
        ))
        .add_handler(Handler::callback_query(
            list_characters,
            filters::query("char list page:int").and(crate::filters::sudoers()),
        ))
        .add_handler(Handler::new_message(
            see_character,
            macros::command!("char")
                .or(macros::command!("character"))
                .and(crate::filters::sudoers()),
        ))
}

async fn add_character(client: &mut Client, update: &mut Update, data: &mut Data) -> Result<()> {
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
                    .replace("{field}", &t("name"))
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

            let last_id = Character::select_last(conn)
                .await?
                .map_or(1, |character| character.id);

            let name = response.text();
            let mut character = Character {
                id: last_id + 1,
                name: name.to_string(),
                stars: 1,
                ..Default::default()
            };
            Character::insert(conn, &character).await?;

            sent.edit(InputMessage::html(
                t("object_created").replace("{object}", &t("character")),
            ))
            .await?;

            tokio::time::sleep(Duration::from_secs(2)).await;
            sent.delete().await?;
            let _ = response.delete().await;

            message
                .edit(InputMessage::html(
                    t("character_info")
                        .replace("{id}", &character.id.to_string())
                        .replace(
                            "{gender}",
                            match character.gender {
                                Gender::Male => "💥",
                                Gender::Female => "🌸",
                                Gender::Other(_) => "🍃",
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
                        ),
                ))
                .await?;

            let field = t("photo");
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

                    character.image = Some(bytes.clone());
                    match Character::update_by_id(conn, &character, character.id).await {
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
                        .upload_stream(
                            &mut stream,
                            bytes.len(),
                            format!("char_{}.jpg", character.id),
                        )
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
                                format!("char edit {}", character.id),
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

                    Character::delete_by_id(conn, character.id).await?;

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

async fn delete_character(
    _client: &mut Client,
    update: &mut Update,
    data: &mut Data,
) -> Result<()> {
    let mut db = data.get_module::<Database>().unwrap();
    let i18n = data.get_module::<I18n>().unwrap();

    let t = |key| i18n.get(key);

    let query = update.get_query().unwrap();
    let message = query.load_message().await?;

    let splitted = utils::split_query(query.data());

    if splitted.len() >= 3 {
        let conn = db.get_conn();

        let character_id = splitted[2].parse::<i64>().unwrap();

        if splitted.len() == 4 && splitted[3].as_str() == "confirm" {
            if let Some(character) = Character::select_by_id(conn, character_id).await? {
                Character::delete_by_id(conn, character_id).await?;
                message
                    .edit(InputMessage::html(
                        t("object_deleted")
                            .replace("{object}", &t("character"))
                            .replace("{id}", &character.id.to_string()),
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
                        .replace("{object}", &t("character").to_lowercase())
                        .replace("{id}", &character_id.to_string()),
                )
                .reply_markup(&reply_markup::inline(vec![vec![
                    button::inline(
                        t("confirm_button"),
                        format!("char delete {} confirm", character_id),
                    ),
                    button::inline(
                        t("cancel_button"),
                        format!("char edit {} back", character_id),
                    ),
                ]])),
            )
            .await?;
    }

    Ok(())
}

async fn edit_character(client: &mut Client, update: &mut Update, data: &mut Data) -> Result<()> {
    let mut db = data.get_module::<Database>().unwrap();
    let i18n = data.get_module::<I18n>().unwrap();
    let conv = data.get_module::<Conversation>().unwrap();

    let t = |key| i18n.get(key);

    let chat = update.get_chat().unwrap();
    let query = update.get_query().unwrap();
    let sender = query.sender();
    let message = query.load_message().await?;

    let mut text = message.html_text();
    let splitted = utils::split_query(query.data());

    if splitted.len() >= 3 {
        let conn = db.get_conn();

        let mut file = None;
        let character_id = splitted[2].parse::<i64>().unwrap();

        if let Some(mut character) = Character::select_by_id(conn, character_id).await? {
            if splitted.len() >= 4 {
                match splitted[3].as_str() {
                    "back" => {
                        message
                            .edit(InputMessage::html(text).reply_markup(&reply_markup::inline(
                                vec![vec![
                                    button::inline(
                                        t("edit_button"),
                                        format!("char edit {}", character_id),
                                    ),
                                    button::inline(
                                        t("delete_button"),
                                        format!("char delete {}", character_id),
                                    ),
                                ]],
                            )))
                            .await?;

                        return Ok(());
                    }
                    "name" => {
                        let field = t("name");
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
                                let new_name = response.text();
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
                                            let name = response.text();

                                            if name == "." || name == "0" {
                                                character.artist = None;
                                            } else {
                                                character.artist = Some(name.to_string());
                                            }

                                            match Character::update_by_id(
                                                conn,
                                                &character,
                                                character_id,
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
                                            let link = response.text();

                                            if link == "." || link == "0" {
                                                character.image_link = None;
                                            } else {
                                                character.image_link = Some(link.to_string());
                                            }

                                            match Character::update_by_id(
                                                conn,
                                                &character,
                                                character_id,
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
                                                format!("char edit {} artist name", character_id),
                                            ),
                                            button::inline(
                                                t("link") + " ✏",
                                                format!("char edit {} artist link", character_id),
                                            ),
                                        ],
                                        vec![button::inline(
                                            t("back_button"),
                                            format!("char edit {}", character_id),
                                        )],
                                    ]),
                                ))
                                .await?;

                            return Ok(());
                        }
                    }
                    "photo" => {
                        let field = t("photo");
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

                                let mut stream = Cursor::new(&bytes);
                                file = Some(
                                    client
                                        .upload_stream(
                                            &mut stream,
                                            bytes.len(),
                                            format!("char_{}.jpg", character_id),
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
                    "gender" => {
                        let field = t("gender");
                        let timeout = 10;

                        message
                            .edit(InputMessage::html(t("select_button")).reply_markup(
                                &reply_markup::inline(vec![
                                    vec![
                                        button::inline(t("male_button"), "male"),
                                        button::inline(t("female_button"), "female"),
                                    ],
                                    vec![button::inline(t("other_button"), "other")],
                                ]),
                            ))
                            .await?;

                        match conv
                            .wait_for_update(
                                sender,
                                filters::query(r"[male|female|other]")
                                    .and(crate::filters::sudoers()),
                                Duration::from_secs(timeout),
                            )
                            .await
                            .unwrap()
                        {
                            Some(update) => {
                                if let Some(query) = update.get_query() {
                                    let sender = query.sender();

                                    let splitted = utils::split_query(query.data());
                                    character.gender = match splitted[0].as_str() {
                                        "male" => Gender::Male,
                                        "female" => Gender::Female,
                                        _ => {
                                            message
                                                .edit(InputMessage::html(
                                                    t("ask_field")
                                                        .replace("{field}", &field)
                                                        .replace("{timeout}", &timeout.to_string()),
                                                ))
                                                .await?;
                                            let gender = match conv
                                                .wait_for_update(
                                                    sender,
                                                    filters::query(r"(\w+)")
                                                        .and(crate::filters::sudoers()),
                                                    Duration::from_secs(timeout),
                                                )
                                                .await
                                                .unwrap()
                                            {
                                                Some(update) => {
                                                    if let Some(message) = update.get_message() {
                                                        let _ = message.delete().await;
                                                        message.text().to_string()
                                                    } else {
                                                        String::from("unknown")
                                                    }
                                                }
                                                None => String::from("unknown"),
                                            };

                                            Gender::Other(gender)
                                        }
                                    };

                                    Character::update_by_id(conn, &character, character_id).await?;
                                }
                            }
                            None => {}
                        }
                    }
                    "stars" => {
                        let timeout = 10;

                        let buttons = (1..=6)
                            .map(|stars| {
                                button::inline(
                                    format!(
                                        "{0} ({1})",
                                        match stars {
                                            1 => "⚪",
                                            2 => "🟢",
                                            3 => "🔵",
                                            4 => "🟣",
                                            5 => "🔴",
                                            _ => "🟡",
                                        },
                                        stars
                                    ),
                                    stars.to_string(),
                                )
                            })
                            .collect::<Vec<_>>();
                        let buttons = utils::split_kb_to_columns(buttons, 3);
                        message
                            .edit(
                                InputMessage::html(t("select_button"))
                                    .reply_markup(&reply_markup::inline(buttons)),
                            )
                            .await?;

                        match conv
                            .wait_for_update(
                                sender,
                                filters::query("[1,2,3,4,5,6]").and(crate::filters::sudoers()),
                                Duration::from_secs(timeout),
                            )
                            .await
                            .unwrap()
                        {
                            Some(update) => {
                                if let Some(query) = update.get_query() {
                                    let splitted = utils::split_query(query.data());

                                    if let Ok(stars) = splitted[0].parse::<u8>() {
                                        character.stars = stars;
                                        Character::update_by_id(conn, &character, character_id)
                                            .await?;
                                    }
                                }
                            }
                            None => {}
                        }
                    }
                    _ => {}
                }
            }

            let fields = ["name", "artist", "photo", "gender", "stars"];
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

            let mut name = character.name;
            if let Some(artist) = character.artist {
                name += &format!(
                    " | 🎨 {}.",
                    if let Some(link) = character.image_link {
                        format!("<a href='{0}'>{1}</a>", link, artist)
                    } else {
                        artist
                    }
                );
            }

            text = t("character_info")
                .replace("{id}", &character.id.to_string())
                .replace(
                    "{gender}",
                    match character.gender {
                        Gender::Male => "💥",
                        Gender::Female => "🌸",
                        Gender::Other(_) => "🍃",
                    },
                )
                .replace("{name}", &name)
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
            let mut input_message = InputMessage::html(text);

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

async fn list_characters(client: &mut Client, update: &mut Update, data: &mut Data) -> Result<()> {
    let mut db = data.get_module::<Database>().unwrap();

    let query = update.get_query().unwrap();
    let message = query.load_message().await?;

    let conn = db.get_conn();

    let mut text = String::from("id | name | stars | gender\n");

    let characters = Character::select_all(conn).await?;
    for character in characters.iter() {
        text += &format!(
            "\n{0} | {1} | {2} | {3}",
            character.id, character.name, character.stars, character.gender
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

    // let current_page = 1;
    //
    // let characters = Character::select_page(conn, current_page, 8).await?;
    // let buttons = characters
    //     .into_iter()
    //     .map(|character| {
    //         button::inline(
    //             format!(
    //                 "{0}. {1}",
    //                 character.id,
    //                 crate::utils::shorten_text(character.name, 12)
    //             ),
    //             format!(""),
    //         )
    //     })
    //     .collect::<Vec<_>>();
    // let buttons = utils::split_kb_to_columns(buttons, 2);
    // buttons.push(utils::gen_page_buttons(
    //     current_page.into(),
    //     total_pages,
    //     "char list ",
    //     5,
    // ));
    // message
    //     .reply(InputMessage::html(
    //         t("page_title").replace("{type}", &t("characters"))
    //             + "\n\n"
    //             + &t("page_info").replace("{current}", &current_page.to_string()),
    //     ))
    //     .await?;

    Ok(())
}

async fn see_character(client: &mut Client, update: &mut Update, data: &mut Data) -> Result<()> {
    let mut db = data.get_module::<Database>().unwrap();
    let i18n = data.get_module::<I18n>().unwrap();

    let t = |key| i18n.get(key);

    let message = update.get_message().unwrap();

    let splitted = message.text().split_whitespace().collect::<Vec<_>>();

    let conn = db.get_conn();

    if splitted.len() <= 1 {
        message
            .reply(
                InputMessage::html(t("select_button")).reply_markup(&reply_markup::inline(vec![
                    vec![
                        button::inline(t("add_button"), format!("char add")),
                        button::inline(t("list_button"), format!("char list 1")),
                    ],
                ])),
            )
            .await?;
    } else {
        match splitted[1].parse::<i64>() {
            Ok(character_id) => {
                if let Some(mut character) = Character::select_by_id(conn, character_id).await? {
                    let mut name = character.name.clone();
                    if let Some(ref artist) = character.artist {
                        name += &format!(
                            " | 🎨 {}.",
                            if let Some(ref link) = character.image_link {
                                format!("<a href='{0}'>{1}</a>", link, artist)
                            } else {
                                artist.to_string()
                            }
                        );
                    }

                    let text = t("character_info")
                        .replace("{id}", &character.id.to_string())
                        .replace(
                            "{gender}",
                            match character.gender {
                                Gender::Male => "💥",
                                Gender::Female => "🌸",
                                Gender::Other(_) => "🍃",
                            },
                        )
                        .replace("{name}", &name)
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

                    let file = crate::utils::upload_photo(client, character.clone(), conn)
                        .await?
                        .unwrap();
                    match message
                        .reply(
                            InputMessage::html(text.clone())
                                .reply_markup(&reply_markup::inline(vec![vec![
                                    button::inline(
                                        t("edit_button"),
                                        format!("char edit {}", character_id),
                                    ),
                                    button::inline(
                                        t("delete_button"),
                                        format!("char delete {}", character_id),
                                    ),
                                ]]))
                                .photo(file),
                        )
                        .await
                    {
                        Err(e) if e.is("FILE_PARTS_MISSING") || e.is("FILE_PARTS_INVALID") => {
                            character.image = None;
                            Character::update_by_id(conn, &character, character_id).await?;

                            message
                                .reply(InputMessage::html(text).reply_markup(
                                    &reply_markup::inline(vec![vec![
                                        button::inline(
                                            t("edit_button"),
                                            format!("char edit {}", character_id),
                                        ),
                                        button::inline(
                                            t("delete_button"),
                                            format!("char delete {}", character_id),
                                        ),
                                    ]]),
                                ))
                                .await?;
                        }
                        Ok(_) | Err(_) => {}
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
