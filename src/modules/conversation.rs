use std::{pin::pin, time::Duration};

use futures_util::future::{select, Either};
use grammers_client::{types::Chat, types::Message, Client, InputMessage};
use grammers_friendly::prelude::*;

use crate::Result;

#[derive(Clone)]
pub struct Conversation {
    client: Client,
}

impl Conversation {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    pub async fn ask_message<F: Filter>(
        &self,
        chat: Chat,
        message: impl Into<InputMessage>,
        mut filter: F,
    ) -> Result<(Message, Option<Message>)> {
        let message = message.into();
        let sent = self.client.send_message(&chat, message).await?;

        loop {
            let sleep = pin!(async { tokio::time::sleep(Duration::from_secs(10)).await });
            let update = pin!(async { self.client.next_update().await });

            let update = match select(sleep, update).await {
                Either::Left(_) => break,
                Either::Right((u, _)) => u?,
            };

            if filter.is_ok(&self.client, &update).await {
                let r_chat = update.get_chat().unwrap();
                let r_message = update.get_message().unwrap();

                match r_chat {
                    Chat::User(user) => {
                        if user.id() == chat.id() {
                            return Ok((sent, Some(r_message)));
                        }
                    }
                    Chat::Group(group) => {
                        if group.id() == chat.id() {
                            if r_message.reply_to_message_id() == Some(sent.id()) {
                                return Ok((sent, Some(r_message)));
                            }
                        }
                    }
                    Chat::Channel(_) => {}
                }
            }
        }

        Ok((sent, None))
    }

    pub async fn ask_photo<F: Filter>(
        &self,
        chat: Chat,
        message: impl Into<InputMessage>,
        mut filter: F,
    ) -> Result<(Message, Option<Message>)> {
        let sent = self.client.send_message(&chat, message).await?;

        loop {
            let sleep = pin!(async { tokio::time::sleep(Duration::from_secs(10)).await });
            let update = pin!(async { self.client.next_update().await });

            let update = match select(sleep, update).await {
                Either::Left(_) => break,
                Either::Right((u, _)) => u?,
            };

            if filter.is_ok(&self.client, &update).await {
                let r_chat = update.get_chat().unwrap();
                let r_message = update.get_message().unwrap();

                if r_message.photo().is_some() {
                    match r_chat {
                        Chat::User(user) => {
                            if user.id() == chat.id() {
                                return Ok((sent, Some(r_message)));
                            }
                        }
                        Chat::Group(group) => {
                            if group.id() == chat.id() {
                                if r_message.reply_to_message_id() == Some(sent.id()) {
                                    return Ok((sent, Some(r_message)));
                                }
                            }
                        }
                        Chat::Channel(_) => {}
                    }
                }
            }
        }

        Ok((sent, None))
    }
}

impl Module for Conversation {}
