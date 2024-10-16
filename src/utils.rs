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

pub fn construct_character_info(character: &Character, series: Option<Series>) -> String {
    let template = construct_character_partial_info(character, true, 0)
        + "{series_type} <i>{series_title}</i>\n\n⭐: {bubble}";

    template
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

pub fn construct_character_partial_info(
    character: &Character,
    show_artist: bool,
    space_count: usize,
) -> String {
    let template = String::from("{gender} <code>{id}</code>. <b>{name}</b>\n");

    let name = character.name.clone()
        + &if show_artist {
            format!(
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
        };

    let character_id_length = character.id.to_string().len();

    template
        .replace(
            "{id}",
            &format!(
                "{0}</code><code>{1}",
                if space_count > character_id_length {
                    " ".repeat(space_count - character_id_length)
                } else {
                    String::new()
                },
                character.id
            ),
        )
        .replace("{gender}", gender_symbol(&character.gender))
        .replace("{name}", &name)
}

pub fn construct_series_info(
    series: &Series,
    total_characters: usize,
    show_artist: bool,
) -> String {
    let mut template = String::from("{emoji} <code>{id}</code>. <b>{title}</b>");

    if total_characters > 0 {
        template += &format!(" | 👨‍👩‍👧‍👦 : {}", total_characters);
    }

    if show_artist {
        template += &format!(
            " | 🎨 {}.",
            if !(series.image_link == "." || series.image_link == "0") {
                format!("<a href='{0}'>{1}</a>", series.image_link, series.artist)
            } else {
                series.artist.clone()
            }
        )
    }

    template += "\n\n";

    template
        .replace("{emoji}", media_type_symbol(&series.media_type))
        .replace("{id}", &series.id.to_string())
        .replace("{title}", &series.title)
        .replace("{media_type}", &series.media_type.to_string())
}

pub fn media_type_symbol(media: &Media) -> &str {
    match media {
        Media::Anime => "📺",
        Media::Game => "🧩",
        Media::Manga => "📚",
        Media::Manhua => "📚",
        Media::Manhwa => "📓",
        Media::LightNovel => "📖",
        Media::VisualNovel => "🧩",
        Media::Unknown => "❓",
    }
}

pub fn gender_symbol(gender: &Gender) -> &str {
    match gender {
        Gender::Male => "💥",
        Gender::Female => "🌸",
        Gender::Other => "🍃",
    }
}

pub async fn upload_banner(
    client: &mut Client,
    mut series: Series,
    conn: &mut RBatis,
) -> Result<Option<Uploaded>> {
    if let Some(bytes) = series.banner {
        // Update series's banner bytes
        series.banner = Some(bytes.clone());
        Series::update_by_id(conn, &series, series.id).await?;

        let mut stream = Cursor::new(&bytes);

        Ok(Some(
            client
                .upload_stream(
                    &mut stream,
                    bytes.len(),
                    format!("series_{}-{}.jpg", series.id, series.title),
                )
                .await?,
        ))
    } else {
        Ok(None)
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
