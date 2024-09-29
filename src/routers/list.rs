use grammers_client::{
    grammers_tl_types::{self as tl, Deserializable, Serializable},
    types::media::Uploaded,
    Client, InputMedia, InputMessage, Update,
};
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

async fn list(client: &mut Client, update: &mut Update, data: &mut Data) -> Result<()> {
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
                    if let Some(mut owned_character) =
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

                        let file = match owned_character.image {
                            Some(bytes) => {
                                Uploaded::from_raw(tl::enums::InputFile::from_bytes(&bytes)?)
                            }
                            None => {
                                let file = ani.get_image(client, owned_character_id).await?;
                                // Update character image's bytes
                                owned_character.image = Some(file.raw.to_bytes());
                                Character::update_by_id(conn, &owned_character, owned_character_id)
                                    .await?;

                                file
                            }
                        };

                        medias.push(InputMedia::html(caption).photo(file));
                    }
                }
            }

            message.reply_album(medias).await?;
        }
    }

    return Ok(());
}
