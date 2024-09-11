use std::collections::HashMap;

use grammers_client::{types::Chat, Client, Update};
use grammers_friendly::prelude::*;
use rbatis::async_trait;

use crate::{
    database::{Group, User},
    modules::{Database, I18n},
    Result,
};

#[derive(Clone, Default)]
pub struct SetLocale {
    chats_locale: HashMap<i64, String>,
}

#[async_trait]
impl MiddlewareImpl for SetLocale {
    async fn call(
        &mut self,
        _client: &mut Client,
        update: &mut Update,
        data: &mut Data,
    ) -> Result<()> {
        let chat = update.get_chat();

        if let Some(chat) = chat {
            let db = data.get_module::<Database>().unwrap();
            let i18n = data.get_module::<I18n>().unwrap();

            if let Some(locale) = self.chats_locale.get(&chat.id()) {
                i18n.set_locale(locale);
                return Ok(());
            }

            match chat {
                Chat::User(user) => {
                    if let Ok(Some(u)) = User::select_by_id(&db.get_conn(), user.id()).await {
                        i18n.set_locale(&u.language_code);
                    }
                }
                Chat::Group(group) => {
                    if let Ok(Some(g)) = Group::select_by_id(&db.get_conn(), group.id()).await {
                        i18n.set_locale(&g.language_code);
                    }
                }
                Chat::Channel(_) => {}
            }
        }

        Ok(())
    }
}
