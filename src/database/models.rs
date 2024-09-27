use rbatis::{crud, impl_delete, impl_select, impl_update};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct User {
    pub id: i64,
    pub username: Option<String>,
    pub full_name: String,
    pub language_code: String,

    pub owned_characters: Option<Vec<i64>>,
}

crud!(User {}, "users");
impl_delete!(User { delete_by_id(id: i64) => "`where id = #{id}`" }, "users");
impl_update!(User { update_by_id(id: i64) => "`where id = #{id}`" }, "users");
impl_select!(User { select_by_id(id: i64) -> Option => "`where id = #{id}`" }, "users");

#[derive(Deserialize, Serialize)]
pub struct Group {
    pub id: i64,
    pub title: String,
    pub username: Option<String>,
    pub language_code: String,

    pub last_character_id: Option<i64>,
    pub last_character_message_id: Option<i32>,
}

crud!(Group {}, "groups");
impl_delete!(Group { delete_by_id(id: i64) => "`where id = #{id}`" }, "groups");
impl_update!(Group { update_by_id(id: i64) => "`where id = #{id}`" }, "groups");
impl_select!(Group { select_by_id(id: i64) -> Option => "`where id = #{id}`" }, "groups");

#[derive(Debug, Deserialize, Serialize)]
pub struct Character {
    pub id: i64,
    pub name: String,
    pub stars: u8,

    pub available: i32,
}

crud!(Character {}, "characters");
impl_delete!(Character { delete_by_id(id: i64) => "`where id = #{id}`" }, "characters");
impl_update!(Character { update_by_id(id: i64) => "`where id = #{id}`" }, "characters");
impl_select!(Character { select_by_id(id: i64) -> Option => "`where id = #{id} limit 1`" }, "characters");
impl_select!(Character { random() -> Option => "`where available = 1 order by random() limit 1`" }, "characters");
