use std::io::Cursor;

use grammers_client::{types::media::Uploaded, Client};
use rbatis::RBatis;

use crate::{database::models::Character, modules::Anilist, Result};

pub fn shorten_text(text: impl Into<String>, size: usize) -> String {
    let mut text = text.into();
    if text.len() > size {
        text.truncate(size);
        text.push_str("...");
    }

    text
}

pub fn escape_html(text: impl Into<String>) -> String {
    text.into()
        .replace("&", "&amp;")
        .replace("<", "&lt;")
        .replace(">", "&gt;")
        .replace(r"\", "&quot;")
        .replace("'", "&#x27;")
        .replace("/", "&#x2F;")
}

pub async fn upload_photo(
    client: &mut Client,
    mut character: Character,
    ani: &mut Anilist,
    conn: &mut RBatis,
) -> Result<Uploaded> {
    let bytes = character.image.unwrap_or({
        let bytes = ani.get_image(character.id).await?.to_vec();

        // Update character's image bytes
        character.image = Some(bytes.clone());
        Character::update_by_id(conn, &character, character.id).await?;

        bytes
    });
    let mut stream = Cursor::new(&bytes);

    Ok(client
        .upload_stream(
            &mut stream,
            bytes.len(),
            format!("char_{}-{}.jpg", character.id, character.name),
        )
        .await?)
}
