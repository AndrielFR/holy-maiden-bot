use grammers_client::{types::Chat, Client, InputMedia, InputMessage, Update};
use grammers_friendly::prelude::*;
use rust_anilist::models::Gender;

use crate::{
    database::models::{Character, UserCharacters},
    modules::{Anilist, Database, I18n},
    Result,
};

pub fn router() -> Router {
    Router::default().add_handler(Handler::new_message(
        list_characters,
        macros::command!("list"),
    ))
}

async fn list_characters(client: &mut Client, update: &mut Update, data: &mut Data) -> Result<()> {
    let mut db = data.get_module::<Database>().unwrap();
    let i18n = data.get_module::<I18n>().unwrap();
    let mut ani = data.get_module::<Anilist>().unwrap();

    let t = |key| i18n.get(key);

    let chat = update.get_chat().unwrap();
    let message = update.get_message().unwrap();
    let sender = update.get_sender().unwrap();

    if let Chat::Group(group) = chat {
        let conn = db.get_conn();

        if let Some(user_characters) =
            UserCharacters::select_by_id(conn, sender.id(), group.id()).await?
        {
            let mut medias = Vec::new();

            for character_id in user_characters.characters_id {
                if let Some(ani_character) = ani.get_char(character_id).await {
                    if let Some(character) = Character::select_by_id(conn, character_id).await? {
                        let caption = t("character_info")
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
