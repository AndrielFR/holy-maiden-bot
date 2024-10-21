use rbatis::{crud, impl_delete, impl_select, impl_update, RBatis};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Clone, Default, Deserialize, Serialize)]
pub struct Character {
    pub id: i64,
    pub name: String,
    pub stars: u8,
    pub image: Option<Vec<u8>>,
    pub gender: Gender,
    pub artist: String,
    pub aliases: Vec<String>,
    pub liked_by: Vec<i64>,
    pub series_id: i64,
    pub image_link: String,

    pub anilist_id: Option<i64>,
}

crud!(Character {}, "characters");
impl_delete!(Character { delete_by_id(id: i64) => "`where id = #{id}`" }, "characters");
impl_update!(Character { update_by_id(id: i64) => "`where id = #{id}`" }, "characters");
impl_select!(Character { select_by_id(id: i64) -> Option => "`where id = #{id} limit 1`" }, "characters");
impl_select!(Character { select_by_name(name: &str) -> Option => "`where name like #{'%' + name + '%'} or aliases like #{'%' + name + '%'} order by name limit 1`" }, "characters");
impl_select!(Character { select_by_series(series_id: i64) -> Vec => "`where series_id = #{series_id}`" }, "characters");
impl_select!(Character { select_page(page: u16, limit: u16) => "`limit #{limit} offset #{(page - 1) * limit}`" }, "characters");
impl_select!(Character { select_page_by_name(name: &str, page: u16, limit: u16) -> Vec => "`where name like #{'%' + name + '%'} or aliases like #{'%' + name + '%'} order by name limit #{limit} offset #{(page - 1) * limit}`" }, "characters");
impl_select!(Character { select_page_by_series(series_id: i64, page: u16, limit: u16) -> Vec => "`where series_id = #{series_id} order by name limit #{limit} offset #{(page - 1) * limit}`" }, "characters");
impl_select!(Character { select_last() -> Option => "`order by id desc limit 1`" }, "characters");
impl_select!(Character { select_random() -> Option => "`order by random() limit 1`" }, "characters");

impl Character {
    pub async fn count_by_series(conn: &mut RBatis, series_id: i64) -> rbatis::Result<usize> {
        let count: u64 = conn
            .query_decode(
                "select count(*) as count from characters where series_id = ?",
                vec![rbs::to_value!(series_id)],
            )
            .await?;

        Ok(count as usize)
    }
}

#[derive(Default, Deserialize, Serialize)]
pub struct Group {
    pub id: i64,
    pub title: String,
    pub username: Option<String>,
    pub language_code: String,
}

crud!(Group {}, "groups");
impl_update!(Group { update_by_id(id: i64) => "`where id = #{id}`" }, "groups");
impl_select!(Group { select_by_id(id: i64) -> Option => "`where id = #{id}`" }, "groups");

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct GroupCharacter {
    pub group_id: i64,
    pub character_id: i64,
    pub last_message_id: i32,

    #[serde(deserialize_with = "bool_from_int", serialize_with = "bool_to_int")]
    pub available: bool,
}

crud!(GroupCharacter {}, "groups_characters");
impl_delete!(GroupCharacter { delete_by_id(group_id: i64, character_id: i64) => "`where group_id = #{group_id} and character_id = #{character_id}`" }, "groups_characters");
impl_update!(GroupCharacter { update_by_id(group_id: i64, character_id: i64) => "`where group_id = #{group_id} and character_id = #{character_id}`" }, "groups_characters");
impl_select!(GroupCharacter { select_by_id(group_id: i64, character_id: i64) -> Option => "`where group_id = #{group_id} and character_id = #{character_id} limit 1`" }, "groups_characters");
impl_select!(GroupCharacter { select_last_by_id(group_id: i64) -> Option => "`where group_id = #{group_id} order by last_message_id desc limit 1`" }, "groups_characters");

#[derive(Clone, Default, Deserialize, Serialize)]
pub struct Series {
    pub id: i64,
    pub title: String,
    pub artist: String,
    pub banner: Option<Vec<u8>>,
    pub aliases: Vec<String>,
    pub liked_by: Vec<i64>,
    pub image_link: String,
    pub media_type: Media,
}

crud!(Series {}, "series");
impl_delete!(Series { delete_by_id(id: i64) => "`where id = #{id}`" }, "series");
impl_update!(Series { update_by_id(id: i64) => "`where id = #{id}`" }, "series");
impl_select!(Series { select_by_id(id: i64) -> Option => "`where id = #{id}`" }, "series");
impl_select!(Series { select_by_title(title: &str) -> Option => "`where title like #{'%' + title + '%'} or aliases like #{'%' + title + '%'} order by title limit 1`" }, "series");
impl_select!(Series { select_page_by_title(title: &str, page: u16, limit: u16) -> Vec => "`where title like #{'%' + title + '%'} or aliases like #{'%' + title + '%'} order by title limit #{limit} offset #{(page - 1) * limit}`" }, "series");
impl_select!(Series { select_last() -> Option => "`order by id desc limit 1`" }, "series");

#[derive(Default, Deserialize, Serialize)]
pub struct User {
    pub id: i64,
    pub username: Option<String>,
    pub full_name: String,
    pub language_code: String,
}

crud!(User {}, "users");
impl_update!(User { update_by_id(id: i64) => "`where id = #{id}`" }, "users");
impl_select!(User { select_by_id(id: i64) -> Option => "`where id = #{id}`" }, "users");

#[derive(Default, Deserialize, Serialize)]
pub struct UserCharacters {
    pub user_id: i64,
    pub group_id: i64,
    pub characters_id: Vec<i64>,
}

crud!(UserCharacters {}, "users_characters");
impl_delete!(UserCharacters { delete_by_id(user_id: i64, group_id: i64) => "`where user_id = #{user_id} and group_id = #{group_id}`" }, "users_characters");
impl_update!(UserCharacters { update_by_id(user_id: i64, group_id: i64) => "`where user_id = #{user_id} and group_id = #{group_id}`" }, "users_characters");
impl_select!(UserCharacters { select_by_id(user_id: i64, group_id: i64) -> Option => "`where user_id = #{user_id} and group_id = #{group_id}`" }, "users_characters");

impl UserCharacters {
    pub async fn select_or_insert_by_id(
        conn: &mut RBatis,
        user_id: i64,
        group_id: i64,
    ) -> rbatis::Result<Option<Self>> {
        if let user_characters @ Some(_) = Self::select_by_id(conn, user_id, group_id).await? {
            Ok(user_characters)
        } else {
            let user_characters = Self {
                user_id,
                group_id,
                characters_id: Vec::new(),
            };
            Self::insert(conn, &user_characters).await?;

            Ok(Some(user_characters))
        }
    }
}

fn bool_from_int<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    match u8::deserialize(deserializer)? {
        0 => Ok(false),
        _ => Ok(true),
    }
}

fn bool_to_int<S>(value: &bool, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_u8(if *value { 1 } else { 0 })
}

#[derive(Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Gender {
    #[default]
    Male,
    Female,
    Other,
}

impl std::fmt::Display for Gender {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Male => "Male",
            Self::Female => "Female",
            Self::Other => "Other",
        })
    }
}

#[derive(Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Media {
    Anime,
    Game,
    Manga,
    Manhua,
    Manhwa,
    LightNovel,
    VisualNovel,
    #[default]
    Unknown,
}

impl std::fmt::Display for Media {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Anime => "Anime",
            Self::Game => "Game",
            Self::Manga => "Manga",
            Self::Manhua => "Manhua",
            Self::Manhwa => "Manhwa",
            Self::LightNovel => "Light Novel",
            Self::VisualNovel => "Visual Novel",
            Self::Unknown => "Unknown",
        })
    }
}
