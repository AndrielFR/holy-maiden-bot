use grammers_client::{
    types::{Chat, InputMessage},
    Client, Update,
};
use grammers_friendly::prelude::*;

use crate::{
    database::{GroupCharacter, UserCharacter},
    modules::{Database, I18n},
    Result,
};

pub fn router() -> Dispatcher {
    Dispatcher::default().add_handler(Handler::new_message(
        collect,
        filters::private()
            .not()
            .and(filters::reply())
            .and(macros::command!("collect")),
    ))
}

async fn collect(_client: Client, update: Update, data: Data) -> Result<()> {
    let db = data.get_module::<Database>().unwrap();
    let i18n = data.get_module::<I18n>().unwrap();

    let t = |key| i18n.get(key);

    let chat = update.get_chat().unwrap();
    let message = update.get_message().unwrap();

    if let Chat::Group(group) = chat {
        let group_id = group.id();

        if let Ok(Some(reply_message)) = message.get_reply().await {
            let message_id = reply_message.id();

            if let Ok(Some(mut group_character)) =
                GroupCharacter::select_latest_by_group_id(&db.get_conn(), group_id).await
            {
                let sender = message.sender().unwrap();
                let sender_id = sender.id();

                if group_character.collected_by.is_some()
                    && group_character.collected_by.unwrap() != sender_id
                {
                    return Ok(());
                }

                if group_character.message_id == message_id {
                    let char_ani = rust_anilist::Client::default()
                        .get_char(serde_json::json!({"id": group_character.anilist_id}))
                        .await;

                    if let Ok(char_ani) = char_ani {
                        if let Ok(Some(_user_character)) = UserCharacter::select_by_ids(
                            &db.get_conn(),
                            sender_id,
                            group_id,
                            group_character.character_id,
                        )
                        .await
                        {
                            message
                                .reply(InputMessage::html(
                                    t("already_collected")
                                        .replace("{0}", &char_ani.url)
                                        .replace("{1}", &char_ani.name.full),
                                ))
                                .await?;
                        } else {
                            let user_character = UserCharacter {
                                user_id: sender_id,
                                group_id,
                                anilist_id: group_character.anilist_id,
                                character_id: group_character.character_id,
                            };
                            UserCharacter::insert(&db.get_conn(), &user_character).await?;

                            group_character.collected_by = Some(sender_id);
                            GroupCharacter::update_by_ids(
                                &db.get_conn(),
                                &group_character,
                                group_id,
                                group_character.character_id,
                            )
                            .await?;

                            message
                                .reply(InputMessage::html(
                                    t("collected")
                                        .replace("{0}", &char_ani.url)
                                        .replace("{1}", &char_ani.name.full),
                                ))
                                .await?;
                        }
                    }
                }
            }

            // message.reply(InputMessage::html(t("collect"))).await?;
        }
    }

    Ok(())
}
