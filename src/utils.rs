use std::io::Cursor;

use grammers_client::{
    types::{media::Uploaded, photo_sizes::VecExt, Downloadable, Photo},
    Client,
};
use rbatis::RBatis;

use crate::{
    database::models::{Character, Gender},
    Result,
};

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

pub fn construct_character_info(template: String, character: &Character, liked: bool) -> String {
    let name = character.name.clone()
        + if liked { " ❤" } else { "" }
        + &format!(
            " | 🎨 {}.",
            if !(character.image_link == "." || character.image_link == "0") {
                format!(
                    "<a href='{0}'>{1}</a>",
                    character.image_link, character.artist
                )
            } else {
                character.artist.clone()
            }
        );
    let text = template
        .replace("{id}", &character.id.to_string())
        .replace(
            "{gender}",
            match character.gender {
                Gender::Male => "💥",
                Gender::Female => "🌸",
                Gender::Other => "🍃",
            },
        )
        .replace("{name}", &name)
        .replace(
            "{bubble}",
            match character.stars {
                1 => "⚪",
                2 => "🟢",
                3 => "🔵",
                4 => "🟣",
                5 => "🔴",
                _ => "🟡",
            },
        );

    text
}

pub async fn upload_photo(
    client: &mut Client,
    mut character: Character,
    conn: &mut RBatis,
) -> Result<Option<Uploaded>> {
    let bytes = character.image.unwrap_or({
        let bytes = if let Some(id) = character.anilist_id {
            download_ani_image(id).await?
        } else {
            Vec::new()
        };

        bytes
    });

    // Update character's image bytes
    character.image = Some(bytes.clone());
    Character::update_by_id(conn, &character, character.id).await?;

    let mut stream = Cursor::new(&bytes);

    Ok(Some(
        client
            .upload_stream(
                &mut stream,
                bytes.len(),
                format!("char_{}-{}.jpg", character.id, character.name),
            )
            .await?,
    ))
}

pub async fn download_photo(url: &str) -> Result<Vec<u8>> {
    let response = reqwest::get(url).await?;
    let content = response.bytes().await?;

    Ok(content.to_vec())
}

async fn download_ani_image(id: i64) -> Result<Vec<u8>> {
    let ani_client = rust_anilist::Client::default().timeout(80);
    let ani_char = ani_client
        .get_char(serde_json::json!({"id": id}))
        .await
        .unwrap();

    download_photo(&ani_char.image.large).await
}

pub async fn download_tele_photo(client: &mut Client, photo: Photo) -> Result<Vec<u8>> {
    let thumbs = photo.thumbs();
    let downloadable = Downloadable::PhotoSize(thumbs.largest().cloned().unwrap());
    let mut download = client.iter_download(&downloadable);

    let mut bytes = Vec::new();
    while let Some(chunk) = download.next().await? {
        bytes.extend(chunk);
    }

    Ok(bytes)
}
