use grammers_client::{reply_markup, types::Chat, Client, InputMessage, Update};
use grammers_friendly::prelude::*;

use crate::{
    database::{Character, UserCharacter},
    modules::{Database, I18n},
    Result,
};

pub fn router() -> Dispatcher {
    Dispatcher::default()
        .add_handler(Handler::new_message(
            list,
            filters::private().not().and(macros::command!("list")),
        ))
        .add_handler(Handler::callback_query(
            list,
            filters::private()
                .not()
                .and(filters::query("list user_id:int page:int")),
        ))
}

async fn list(_client: Client, update: Update, data: Data) -> Result<()> {
    let db = data.get_module::<Database>().unwrap();
    let i18n = data.get_module::<I18n>().unwrap();

    let chat = update.get_chat().unwrap();
    let query = update.get_query();

    let t = |key| i18n.get(key);
    let mut text = t("list");
    text += "\n";

    let mut page = 0;
    // let page_limit = 15;
    let page_limit = 15;
    let mut total_pages = 0;

    if let Chat::Group(group) = chat {
        let group_id = group.id();

        if let Some(message) = update.get_message() {
            let user = message.sender().unwrap();
            let user_id = user.id();

            get_page_string(&db, &mut text, user_id, group_id, page, page_limit).await?;

            if let Ok((page_text, total_pgs)) =
                get_page_info(&db, user_id, group_id, page, page_limit, t("page")).await
            {
                text += &page_text;
                total_pages = total_pgs;
            }

            let buttons = utils::gen_page_buttons(
                page + 1,
                total_pages,
                format!("list {} :page:", user_id),
                5,
            );
            message
                .reply(InputMessage::html(&text).reply_markup(&reply_markup::inline([buttons])))
                .await?;
        } else if let Some(query) = query {
            let message = query.load_message().await?;

            let splitted = utils::split_query(query.data());

            let user_id = splitted.get(1).unwrap().parse::<i64>().unwrap();
            page = splitted.get(2).unwrap().parse::<i64>().unwrap() - 1;

            get_page_string(&db, &mut text, user_id, group_id, page, page_limit).await?;

            if let Ok((page_text, total_pgs)) =
                get_page_info(&db, user_id, group_id, page, page_limit, t("page")).await
            {
                text += &page_text;
                total_pages = total_pgs;
            }

            let buttons = utils::gen_page_buttons(
                page + 1,
                total_pages,
                format!("list {} :page:", user_id),
                5,
            );
            message
                .edit(InputMessage::html(&text).reply_markup(&reply_markup::inline([buttons])))
                .await?;
        }
    }

    Ok(())
}

async fn get_page_string(
    db: &Database,
    text: &mut String,
    user_id: i64,
    group_id: i64,
    page: i64,
    page_limit: i64,
) -> Result<()> {
    if let Ok(user_characters) =
        UserCharacter::select_page_by_ids(&db.get_conn(), user_id, group_id, page, page_limit).await
    {
        for user_character in user_characters {
            if let Some(character) =
                Character::select_by_id(&db.get_conn(), user_character.character_id).await?
            {
                if let Ok(char_ani) = rust_anilist::Client::default()
                    .get_char(serde_json::json!({"id": character.anilist_id}))
                    .await
                {
                    text.push_str(&format!(
                        "\n- <a href=\"{0}\">{1}</a> - ",
                        char_ani.url, char_ani.name.full
                    ));
                    // text.push_str(&format!(" <b>V{0}</b> ", character.value));
                    text.push_str('🟊'.to_string().repeat(character.stars as usize).as_str());
                }
            }
        }
    }

    Ok(())
}

async fn get_page_info(
    db: &Database,
    user_id: i64,
    group_id: i64,
    page: i64,
    page_limit: i64,
    page_text: String,
) -> Result<(String, i64)> {
    let mut text = String::new();
    let mut total_pages = 0;

    if let Ok(user_characters) =
        UserCharacter::select_all_by_ids(&db.get_conn(), user_id, group_id).await
    {
        let page = page + 1;
        total_pages = (user_characters.len() as f64 / page_limit as f64).ceil() as i64;

        text += &format!(
            "\n\n{0}",
            page_text
                .replace("{0}", &(page).to_string())
                .replace("{1}", &total_pages.to_string(),)
        );
    }

    Ok((text, total_pages))
}
