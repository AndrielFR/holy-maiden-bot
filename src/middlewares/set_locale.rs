use async_trait::async_trait;
use grammers_client::{types::Chat, Client, Update};
use grammers_friendly::prelude::*;

use crate::{
    database::models::{Group, User},
    modules::{Database, I18n},
    Result,
};

#[derive(Clone)]
pub struct SetLocale;

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
            let mut db = data.get_module::<Database>().unwrap();
            let i18n = data.get_module::<I18n>().unwrap();

            let conn = db.get_conn();

            match chat {
                Chat::User(ref user) => {
                    if let Some(u) = User::select_by_id(conn, user.id()).await? {
                        i18n.set_locale(&u.language_code);
                    }
                }
                Chat::Group(ref group) => {
                    if let Some(g) = Group::select_by_id(conn, group.id()).await? {
                        i18n.set_locale(&g.language_code);
                    }
                }
                Chat::Channel(_) => {}
            }
        }

        Ok(())
    }
}
