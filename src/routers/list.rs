use grammers_client::{Client, InputMedia, InputMessage, Update};
use grammers_friendly::prelude::*;
use rust_anilist::models::Gender;

use crate::{
    database::models::{Character, User},
    modules::{Anilist, Database, I18n},
    Result,
};

pub fn router() -> Router {
    Router::default().add_handler(Handler::new_message(list, macros::command!("list")))
}

async fn list(_client: &mut Client, update: &mut Update, data: &mut Data) -> Result<()> {
    let mut db = data.get_module::<Database>().unwrap();
    let i18n = data.get_module::<I18n>().unwrap();
    let mut ani = data.get_module::<Anilist>().unwrap();

    let t = |key| i18n.get(key);

    let message = update.get_message().unwrap();

    let conn = db.get_conn();

    let sender = update.get_sender().unwrap();
    if let Some(user) = User::select_by_id(conn, sender.id()).await? {
        let owned_characters = user.owned_characters.unwrap_or_else(Vec::new);
        if owned_characters.is_empty() {
            message
                .reply(InputMessage::html(t("no_characters")))
                .await?;
        } else {
            let mut medias = Vec::new();

            for owned_character_id in owned_characters {
                if let Some(character) = ani.get_char(owned_character_id).await {
                    if let Some(owned_character) =
                        Character::select_by_id(conn, owned_character_id).await?
                    {
                        let caption = String::from("{gender_emoji} <b>{name}</b>\n\n⭐: {stars}")
                            .replace(
                                "{gender_emoji}",
                                match character.gender.unwrap_or(Gender::NonBinary) {
                                    Gender::Male => "💥",
                                    Gender::Female => "🌸",
                                    Gender::NonBinary | Gender::Other(_) => "🍃",
                                },
                            )
                            .replace(
                                "{name}",
                                &format!(
                                    "<a href=\"{0}\">{1}</a>",
                                    character.url, owned_character.name
                                ),
                            )
                            .replace(
                                "{stars}",
                                match owned_character.stars {
                                    1 => "⚪",
                                    2 => "🟢",
                                    3 => "🔵",
                                    4 => "🟣",
                                    5 => "🔴",
                                    _ => "🟡",
                                },
                            );

                        medias.push(InputMedia::html(caption).photo_url(character.image.large));
                    }
                }
            }

            message.reply_album(medias).await?;
        }
    }

    return Ok(());
}
