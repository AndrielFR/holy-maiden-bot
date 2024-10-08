use grammers_client::{reply_markup, types::Chat, Client, InputMedia, InputMessage, Update};
use grammers_friendly::prelude::*;

use crate::{
    database::models::{Character, UserCharacters},
    modules::{Database, I18n},
    Result,
};

pub fn router() -> Router {
    Router::default()
        .add_handler(Handler::new_message(
            list_characters_individually,
            macros::command!("/!.", "l"),
        ))
        .add_handler(Handler::callback_query(
            list_characters_individually,
            filters::query("list index:int"),
        ))
        .add_handler(Handler::new_message(
            list_characters,
            macros::command!("list"),
        ))
}

async fn list_characters_individually(
    client: &mut Client,
    update: &mut Update,
    data: &mut Data,
) -> Result<()> {
    let mut db = data.get_module::<Database>().unwrap();
    let i18n = data.get_module::<I18n>().unwrap();

    let t = |key| i18n.get(key);

    let query = update.get_query();
    let sender = update.get_sender().unwrap();
    let message = if let Some(ref q) = query {
        q.load_message().await?
    } else {
        update.get_message().unwrap()
    };

    if let Some(chat) = update.get_chat() {
        match chat {
            Chat::User(_) => {
                message.reply(InputMessage::html(t("not_a_group"))).await?;
            }
            Chat::Group(group) => {
                let conn = db.get_conn();

                if let Some(user_characters) =
                    UserCharacters::select_by_id(conn, sender.id(), group.id()).await?
                {
                    let index = {
                        if let Some(ref query) = query {
                            let splitted = utils::split_query(query.data());

                            splitted[1].parse::<usize>().unwrap_or(1)
                        } else {
                            1
                        }
                    };

                    if let Some(character_id) = user_characters.characters_id.get(index - 1) {
                        if let Some(character) =
                            Character::select_by_id(conn, *character_id).await?
                        {
                            let caption = crate::utils::construct_character_info(
                                t("character_info"),
                                &character,
                            );
                            let buttons = utils::gen_page_buttons(
                                index as i64,
                                user_characters.characters_id.len() as i64,
                                "list :page:",
                                5,
                            );

                            let mut input_message = InputMessage::html(caption)
                                .reply_markup(&reply_markup::inline(vec![buttons]));

                            if let Some(file) =
                                crate::utils::upload_photo(client, character, conn).await?
                            {
                                input_message = input_message.photo(file);
                            }

                            if query.is_some() {
                                message.edit(input_message).await?;
                            } else {
                                message.reply(input_message).await?;
                            }
                        }
                    } else {
                        message
                            .reply(InputMessage::html(t("no_characters")))
                            .await?;
                    }
                } else {
                    message
                        .reply(InputMessage::html(t("no_characters")))
                        .await?;
                }
            }
            Chat::Channel(_) => {}
        }
    }

    return Ok(());
}

async fn list_characters(client: &mut Client, update: &mut Update, data: &mut Data) -> Result<()> {
    let mut db = data.get_module::<Database>().unwrap();
    let i18n = data.get_module::<I18n>().unwrap();

    let t = |key| i18n.get(key);

    let chat = update.get_chat().unwrap();
    let sender = update.get_sender().unwrap();
    let message = update.get_message().unwrap();

    if message.text().split_whitespace().last().unwrap() == "i" {
        return list_characters_individually(client, update, data).await;
    }

    if let Chat::Group(group) = chat {
        let conn = db.get_conn();

        if let Some(user_characters) =
            UserCharacters::select_by_id(conn, sender.id(), group.id()).await?
        {
            let mut medias = Vec::new();

            for character_id in user_characters.characters_id {
                if let Some(character) = Character::select_by_id(conn, character_id).await? {
                    let caption =
                        crate::utils::construct_character_info(t("character_info"), &character);
                    if let Some(file) = crate::utils::upload_photo(client, character, conn).await? {
                        medias.push(InputMedia::html(caption).photo(file));
                    }
                }
            }

            message.reply_album(medias).await?;
        } else {
            message
                .reply(InputMessage::html(t("no_characters")))
                .await?;
        }
    } else {
        message.reply(InputMessage::html(t("not_a_group"))).await?;
    }

    return Ok(());
}
