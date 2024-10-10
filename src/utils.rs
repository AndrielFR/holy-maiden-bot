use std::io::Cursor;

use grammers_client::{
    types::{media::Uploaded, photo_sizes::VecExt, Downloadable, Photo},
    Client,
};
use rbatis::RBatis;

use crate::{
    database::models::{Character, Gender, Media, Series},
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

pub fn construct_character_info(
    character: &Character,
    is_liked: bool,
    series: Option<Series>,
) -> String {
    let template = String::from(
        "{gender} <code>{id}</code>. <b>{name}</b>\n{series_type} <i>{series_title}</i>\n\n⭐: {bubble}",
    );

    let name = character.name.clone()
        + if is_liked { " ❤" } else { "" }
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

    template
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
            "{series_title}",
            &match series {
                Some(ref series) => series.title.clone(),
                None => String::new(),
            },
        )
        .replace(
            "{series_type}",
            &match series {
                Some(ref series) => media_type_symbol(&series.media_type).to_string(),
                None => media_type_symbol(&Media::Unknown).to_string(),
            },
        )
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
        )
}

pub fn construct_series_info(series: &Series, character: Option<&Character>) -> String {
    let template = String::from(
        "<code>{id}</code>. <b>{title}</b>\n{emoji} <i>{media_type}</i>\n\n{char_gender} <code>{char_id}</code> <b>{char_name}</b>",
    );

    template
        .replace("{id}", &series.id.to_string())
        .replace("{title}", &series.title)
        .replace("{emoji}", media_type_symbol(&series.media_type))
        .replace("{media_type}", &series.media_type.to_string())
        .replace(
            "{char_gender}",
            if let Some(character) = character {
                match character.gender {
                    Gender::Male => "💥",
                    Gender::Female => "🌸",
                    Gender::Other => "🍃",
                }
            } else {
                ""
            },
        )
        .replace(
            "{char_id}",
            &if let Some(character) = character {
                format!("{}.", character.id.to_string())
            } else {
                String::new()
            },
        )
        .replace(
            "{char_name}",
            &if let Some(character) = character {
                character.name.clone()
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
                    )
            } else {
                String::new()
            },
        )
}

pub fn media_type_symbol(media: &Media) -> &str {
    match media {
        Media::Anime => "📺",
        Media::Game => "🧩",
        Media::Manga => "📚",
        Media::Manhua => "📚",
        Media::Manhwa => "📚",
        Media::LightNovel => "📖",
        Media::VisualNovel => "🧩",
        Media::Unknown => "❓",
    }
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
