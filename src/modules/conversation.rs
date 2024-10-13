use std::{pin::pin, time::Duration};

use futures_util::future::{select, Either};
use grammers_client::{
    types::{Chat, Message},
    Client, InputMessage, Update,
};
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
        user: &Chat,
        message: impl Into<InputMessage>,
        filter: F,
        timeout: Duration,
    ) -> Result<(Message, Option<Message>)> {
        let message = message.into();
        let sent = self.client.send_message(&chat, message).await?;

        let mut message = None;
        let filter: Box<dyn Filter> = Box::new(filter);

        loop {
            if let Ok(Some(update)) = self._wait_for_update(user, filter.clone(), timeout).await {
                if let Some(r_chat) = update.get_chat() {
                    if let Some(r_message) = update.get_message() {
                        if !r_message.text().is_empty() && r_message.media().is_none() {
                            if check_message(r_chat, &r_message, sent.id()) {
                                message = Some(r_message);
                                break;
                            }
                        }
                    }
                }
            } else {
                break;
            }
        }

        Ok((sent, message))
    }

    pub async fn ask_photo<F: Filter>(
        &self,
        chat: Chat,
        user: &Chat,
        message: impl Into<InputMessage>,
        filter: F,
        timeout: Duration,
    ) -> Result<(Message, Option<Message>)> {
        let sent = self.client.send_message(&chat, message).await?;

        let mut message = None;
        let filter: Box<dyn Filter> = Box::new(filter);

        loop {
            if let Ok(Some(update)) = self._wait_for_update(user, filter.clone(), timeout).await {
                if let Some(r_chat) = update.get_chat() {
                    if let Some(r_message) = update.get_message() {
                        if r_message.photo().is_some() {
                            if check_message(r_chat, &r_message, sent.id()) {
                                message = Some(r_message);
                                break;
                            }
                        }
                    }
                }
            } else {
                break;
            }
        }

        Ok((sent, message))
    }

    pub async fn wait_for_update<F: Filter>(
        &self,
        user: &Chat,
        filter: F,
        timeout: Duration,
    ) -> Result<Option<Update>> {
        self._wait_for_update(user, Box::new(filter), timeout).await
    }

    async fn _wait_for_update(
        &self,
        user: &Chat,
        mut filter: Box<dyn Filter>,
        timeout: Duration,
    ) -> Result<Option<Update>> {
        let mut r = None;

        loop {
            let sleep = pin!(async { tokio::time::sleep(timeout).await });
            let update = pin!(async { self.client.next_update().await });

            let update = match select(sleep, update).await {
                Either::Left(_) => break,
                Either::Right((u, _)) => u?,
            };

            if let Some(sender) = update.get_sender() {
                if sender.id() != user.id() {
                    continue;
                }
            }

            if filter.is_ok(&self.client, &update).await {
                r = Some(update);
                break;
            }
        }

        Ok(r)
    }
}

impl Module for Conversation {}

fn check_message(chat: Chat, message: &Message, message_id: i32) -> bool {
    match chat {
        Chat::User(ref user) => {
            if user.id() == chat.id() {
                return true;
            }
        }
        Chat::Group(ref group) => {
            if group.id() == chat.id() {
                if message.reply_to_message_id() == Some(message_id) {
                    return true;
                }
            }
        }
        Chat::Channel(_) => {}
    }

    return false;
}
