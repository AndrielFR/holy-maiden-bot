use grammers_client::{button, reply_markup, types::Chat, Client, InputMessage, Update};
use grammers_friendly::prelude::*;

use crate::{
    database::models::{Group, User},
    modules::{Database, I18n},
    Result,
};

pub fn router() -> Router {
    Router::default()
        .add_handler(Handler::new_message(
            language,
            macros::command!("language").and(filters::private().or(filters::admin())),
        ))
        .add_handler(Handler::callback_query(
            set_language,
            filters::query("set_language lang:str").and(filters::private().or(filters::admin())),
        ))
}

async fn language(_client: &mut Client, update: &mut Update, data: &mut Data) -> Result<()> {
    let i18n = data.get_module::<I18n>().unwrap();
    let t = |key| i18n.get(key);

    let message = update.get_message().unwrap();

    let buttons = i18n
        .locales()
        .iter()
        .map(|locale| {
            button::inline(
                format!(
                    "{} {}",
                    i18n.get_from_locale(locale, "language_name"),
                    if *locale == i18n.locale() { "✔" } else { "" }
                ),
                format!("set_language {}", locale),
            )
        })
        .collect::<Vec<_>>();
    let buttons = utils::split_kb_to_columns(buttons, 2);

    message
        .reply(InputMessage::html(t("language")).reply_markup(&reply_markup::inline(buttons)))
        .await?;

    Ok(())
}

async fn set_language(_client: &mut Client, update: &mut Update, data: &mut Data) -> Result<()> {
    let mut db = data.get_module::<Database>().unwrap();
    let i18n = data.get_module::<I18n>().unwrap();

    let chat = update.get_chat().unwrap();
    let query = update.get_query().unwrap();
    let message = query.load_message().await?;

    let splitted = utils::split_query(query.data());
    let locale = splitted.get(1).unwrap();
    i18n.set_locale(locale);

    let t = |key| i18n.get(key);
    let text = t("language_set").replace("{new_lang}", &t("language_name"));
    let input_message = InputMessage::html(&text).reply_markup(&reply_markup::hide());

    let conn = db.get_conn();

    match chat {
        Chat::User(u) => {
            if let Some(mut user) = User::select_by_id(conn, u.id()).await? {
                if &user.language_code != locale {
                    user.language_code = locale.to_string();
                    User::update_by_id(conn, &user, user.id).await?;

                    message.edit(input_message).await?;
                }
            }
        }
        Chat::Group(g) => {
            if let Some(mut group) = Group::select_by_id(conn, g.id()).await? {
                if &group.language_code != locale {
                    group.language_code = locale.to_string();
                    Group::update_by_id(conn, &group, group.id).await?;

                    message.edit(input_message).await?;
                }
            }
        }
        Chat::Channel(_) => {}
    }

    Ok(())
}
