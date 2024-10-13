use grammers_client::{button, reply_markup, types::Chat, Client, InputMessage, Update};
use grammers_friendly::prelude::*;

use crate::{modules::I18n, Result};

pub fn router() -> Router {
    Router::default().add_handler(Handler::new_message(start, macros::command!("start")))
}

async fn start(client: &mut Client, update: &mut Update, data: &mut Data) -> Result<()> {
    let i18n = data.get_module::<I18n>().unwrap();
    let t = |key| i18n.get(key);

    let me = client.get_me().await?;
    let chat = update.get_chat().unwrap();
    let message = update.get_message().unwrap();

    let username = me.username().unwrap();

    match chat {
        Chat::User(_) => {
            message
                .reply(
                    InputMessage::html(t("start")).reply_markup(&reply_markup::inline(vec![vec![
                        button::url(
                            t("add_to_a_group_button"),
                            format!("t.me/{}?startgroup=new", username),
                        ),
                    ]])),
                )
                .await?;
        }
        Chat::Group(_) => {
            message
                .reply(
                    InputMessage::html(t("not_private")).reply_markup(&reply_markup::inline(vec![
                        vec![button::url(
                            t("private_button"),
                            format!("t.me/{}?start", username),
                        )],
                    ])),
                )
                .await?;
        }
        Chat::Channel(_) => {}
    }

    Ok(())
}
