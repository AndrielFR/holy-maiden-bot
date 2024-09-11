use grammers_client::{Client, InputMessage, Update};
use grammers_friendly::prelude::*;

use crate::{modules::I18n, Result};

pub fn router() -> Dispatcher {
    Dispatcher::default().add_handler(Handler::new_message(help, macros::command!("help")))
}

async fn help(_client: Client, update: Update, data: Data) -> Result<()> {
    let i18n = data.get_module::<I18n>().unwrap();
    let t = |key| i18n.get(key);

    let message = update.get_message().unwrap();

    message.reply(InputMessage::html(t("help"))).await?;

    Ok(())
}
