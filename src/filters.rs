use grammers_client::{types::Chat, Client, Update};
use grammers_friendly::prelude::*;
use rbatis::async_trait;

#[derive(Clone)]
pub struct SudoUser {
    pub ids: Vec<i64>,
}

#[async_trait]
impl Filter for SudoUser {
    async fn is_ok(&mut self, _client: &Client, update: &Update) -> bool {
        let chat = update.get_chat().unwrap();

        match chat {
            Chat::User(user) => self.ids.contains(&user.id()),
            Chat::Group(_) => match update.get_message() {
                Some(message) => match message.sender() {
                    Some(user) => self.ids.contains(&user.id()),
                    None => false,
                },
                None => false,
            },
            Chat::Channel(_) => false,
        }
    }
}

pub fn sudoers() -> SudoUser {
    SudoUser {
        ids: vec![
            1155717290, // @AndrielFR
            1588846160, // @mad_scientistt
            996752722,  // @banzinhoo
            1300992799, // @deggeh
            254533953,  // @aaronkrs
            1363034147, // @vNzera
            1412056311, // @rodrigarroo
            1459361261, // @gnsfujiwara
            1517093012, // @FengCelestial
            1269726556, // @supernovamongol
        ],
    }
}
