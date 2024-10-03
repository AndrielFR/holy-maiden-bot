use grammers_client::{Client, InputMessage, Update};
use grammers_friendly::prelude::*;
use rust_anilist::models::Gender;

use crate::{
    database::models::Character,
    modules::{Anilist, Database, I18n},
    utils, Result,
};

pub fn router() -> Router {
    Router::default().add_handler(Handler::new_message(
        see_character,
        macros::command!("char")
            .or(macros::command!("character"))
            .and(crate::filters::sudoers()),
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
                &utils::escape_html(format!("{} <id>", splitted.first().unwrap())),
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
                        message.reply(InputMessage::html(text).photo(file)).await?;
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
