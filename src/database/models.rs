use rbatis::{crud, impl_delete, impl_select, impl_update, RBatis};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Clone, Default, Deserialize, Serialize)]
pub struct Character {
    pub id: i64,
    pub name: String,
    pub stars: u8,
    pub image: Option<Vec<u8>>,
    pub gender: Gender,
    pub artist: Option<String>,
    pub image_link: Option<String>,

    pub anilist_id: Option<i64>,
}

crud!(Character {}, "characters");
impl_delete!(Character { delete_by_id(id: i64) => "`where id = #{id}`" }, "characters");
impl_update!(Character { update_by_id(id: i64) => "`where id = #{id}`" }, "characters");
impl_select!(Character { select_by_id(id: i64) -> Option => "`where id = #{id} limit 1`" }, "characters");
impl_select!(Character { select_page(page: u16, limit: u16) => "`offset #{(page - 1) * limit} limit #{limit}`" }, "characters");
impl_select!(Character { select_last() -> Option => "`order by id desc limit 1`" }, "characters");
impl_select!(Character { select_random() -> Option => "`order by random() limit 1`" }, "characters");

#[derive(Deserialize, Serialize)]
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

crud!(GroupCharacter {}, "group_characters");
impl_delete!(GroupCharacter { delete_by_id(group_id: i64, character_id: i64) => "`where group_id = #{group_id} and character_id = #{character_id}`" }, "group_characters");
impl_update!(GroupCharacter { update_by_id(group_id: i64, character_id: i64) => "`where group_id = #{group_id} and character_id = #{character_id}`" }, "group_characters");
impl_select!(GroupCharacter { select_by_id(group_id: i64, character_id: i64) -> Option => "`where group_id = #{group_id} and character_id = #{character_id} limit 1`" }, "group_characters");
impl_select!(GroupCharacter { select_last_by_id(group_id: i64) -> Option => "`where group_id = #{group_id} order by last_message_id desc limit 1`" }, "group_characters");

#[derive(Deserialize, Serialize)]
pub struct User {
    pub id: i64,
    pub username: Option<String>,
    pub full_name: String,
    pub language_code: String,
}

crud!(User {}, "users");
impl_update!(User { update_by_id(id: i64) => "`where id = #{id}`" }, "users");
impl_select!(User { select_by_id(id: i64) -> Option => "`where id = #{id}`" }, "users");

#[derive(Deserialize, Serialize)]
pub struct UserCharacters {
    pub user_id: i64,
    pub group_id: i64,
    pub characters_id: Vec<i64>,
}

crud!(UserCharacters {}, "user_characters");
impl_delete!(UserCharacters { delete_by_id(user_id: i64, group_id: i64) => "`where user_id = #{user_id} and group_id = #{group_id}`" }, "user_characters");
impl_update!(UserCharacters { update_by_id(user_id: i64, group_id: i64) => "`where user_id = #{user_id} and group_id = #{group_id}`" }, "user_characters");
impl_select!(UserCharacters { select_by_id(user_id: i64, group_id: i64) -> Option => "`where user_id = #{user_id} and group_id = #{group_id}`" }, "user_characters");

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
    #[serde(untagged)]
    Other(String),
}

impl std::fmt::Display for Gender {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Male => "male",
            Self::Female => "female",
            Self::Other(other) => other.as_str(),
        })
    }
}
