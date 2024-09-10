use rbatis::{crud, impl_delete, impl_select, impl_update};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct User {
    pub id: i64,
    pub username: Option<String>,
    pub first_name: String,
    pub last_name: Option<String>,
    pub language_code: String,
}

crud!(User {}, "users");
impl_delete!(User { delete_by_id(id:i64) => "`where id = #{id}`" }, "users");
impl_update!(User { update_by_id(id:i64) => "`where id = #{id}`" }, "users");
impl_select!(User { select_by_id(id:i64) -> Option => "`where id = #{id} limit 1`" }, "users");

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Group {
    pub id: i64,
    pub username: Option<String>,
    pub language_code: String,
}

crud!(Group {}, "groups");
impl_delete!(Group { delete_by_id(id:i64) => "`where id = #{id}`" }, "groups");
impl_update!(Group { update_by_id(id:i64) => "`where id = #{id}`" }, "groups");
impl_select!(Group { select_by_id(id:i64) -> Option => "`where id = #{id} limit 1`" }, "groups");

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Character {
    pub id: i64,
    pub name: String,
    pub value: u16,
    pub stars: u8,
    pub gender: String,
    pub anilist_id: i64,
}

crud!(Character {}, "characters");
impl_delete!(Character { delete_by_id(id:i64) => "`where id = #{id}`" }, "characters");
impl_update!(Character { update_by_id(id:i64) => "`where id = #{id}`" }, "characters");
impl_select!(Character { select_by_id(id:i64) -> Option => "`where id = #{id} limit 1`" }, "characters");
impl_select!(Character { select_random() -> Option => "`order by random() limit 1`" }, "characters");

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct GroupCharacter {
    pub group_id: i64,
    pub anilist_id: i64,
    pub message_id: i32,
    pub character_id: i64,
    pub collected_by: Option<i64>,
}

crud!(GroupCharacter {}, "group_characters");
impl_delete!(GroupCharacter { delete_by_id(id:i64) => "`where id = #{id}`" }, "group_characters");
impl_update!(GroupCharacter { update_by_id(id:i64) => "`where id = #{id}`" }, "group_characters");
impl_select!(GroupCharacter { select_by_id(id:i64) -> Option => "`where id = #{id} limit 1`" }, "group_characters");
impl_select!(GroupCharacter { select_latest(group_id:i64) -> Option => "`where group_id = ${group_id} order by id desc limit 1`" }, "group_characters");
