use grammers_client::{Client, InputMedia, InputMessage, Update};
use grammers_friendly::prelude::*;

use crate::{
    database::models::{Character, User},
    modules::{Database, I18n},
    Result,
};

pub fn router() -> Router {
    Router::default().add_handler(Handler::new_message(list, macros::command!("list")))
}

async fn list(_client: &mut Client, update: &mut Update, data: &mut Data) -> Result<()> {
    let mut db = data.get_module::<Database>().unwrap();
    let i18n = data.get_module::<I18n>().unwrap();

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

            let ani_client = rust_anilist::Client::default();

            for owned_character_id in owned_characters {
                if let Some(character) = Character::select_by_id(conn, owned_character_id).await? {
                    if let Some(char_ani) = ani_client
                        .get_char(serde_json::json!({"id": owned_character_id}))
                        .await
                        .ok()
                    {
                        let caption = String::from("👩‍👦: {name}\n\n{stars}")
                            .replace(
                                "{name}",
                                &format!("<a href=\"{0}\">{1}</a>", char_ani.url, character.name),
                            )
                            .replace("{stars}", &"⭐".repeat(character.stars as usize));
                        medias.push(InputMedia::html(caption).photo_url(char_ani.image.large));
                    }
                }
            }

            message.reply_album(medias).await?;
        }
    }

    return Ok(());
}
