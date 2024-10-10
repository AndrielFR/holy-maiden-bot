use grammers_client::{button, reply_markup, Client, InputMessage, Update};
use grammers_friendly::prelude::*;

use crate::{
    database::models::Series,
    modules::{Database, I18n},
    Result,
};

pub fn router() -> Router {
    Router::default().add_handler(Handler::new_message(
        see_serie,
        macros::command!("obra")
            .or(macros::command!("/!.", "o"))
            .or(macros::command!("serie"))
            .or(macros::command!("/!.", "s")),
    ))
}

async fn see_serie(client: &mut Client, update: &mut Update, data: &mut Data) -> Result<()> {
    let mut db = data.get_module::<Database>().unwrap();
    let i18n = data.get_module::<I18n>().unwrap();

    let t = |key| i18n.get(key);

    let query = update.get_query();
    let message = if let Some(ref query) = query {
        query.load_message().await?
    } else {
        update.get_message().unwrap()
    };

    let splitted = if let Some(ref query) = query {
        utils::split_query(query.data())
    } else {
        message
            .text()
            .split_whitespace()
            .map(|part| part.to_string())
            .collect::<Vec<String>>()
    };

    if splitted.len() == 1 {
        if crate::filters::sudoers().is_ok(client, update).await {
            message
                .reply(
                    InputMessage::html(t("select_button")).reply_markup(&reply_markup::inline(
                        vec![vec![
                            button::inline(t("add_button"), format!("series add")),
                            button::inline(t("list_button"), format!("series list 1")),
                        ]],
                    )),
                )
                .await?;
        } else {
            message
                .reply(InputMessage::html(t("invalid_command").replace(
                    "{cmd}",
                    &crate::utils::escape_html(format!("{} <name|id>", splitted[0])),
                )))
                .await?;
        }
    } else {
        let conn = db.get_conn();

        if let Some(series) = match splitted[1].parse::<i64>() {
            Ok(id) => Series::select_by_id(conn, id).await?,
            Err(_) => {
                if let Some(series) = Series::select_by_name(conn, &splitted[1]).await? {
                    Some(series)
                } else {
                    None
                }
            }
        } {
            let mut buttons = Vec::new();

            if crate::filters::sudoers().is_ok(client, update).await {
                buttons.push(vec![
                    button::inline(t("edit_button"), format!("series edit {}", series.id)),
                    button::inline(t("delete_button"), format!("series delete {}", series.id)),
                ]);
            }

            let mut input_message = InputMessage::html(crate::utils::construct_series_info(
                t("series_info"),
                &series,
            ));

            if !buttons.is_empty() {
                input_message = input_message.reply_markup(&reply_markup::inline(buttons));
            }

            message.reply(input_message).await?;
        } else {
            message
                .reply(InputMessage::html(t("unknown_series")))
                .await?;
        }
    }

    Ok(())
}
