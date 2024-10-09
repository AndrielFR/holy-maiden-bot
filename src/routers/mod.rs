mod admin;
mod character;
mod collect;
mod help;
mod language;
mod list;
mod send_character;
mod start;

pub use admin::router as admin;
pub use character::router as character;
pub use collect::router as collect;
pub use help::router as help;
pub use language::router as language;
pub use list::router as list;
pub use send_character::router as send_character;
pub use start::router as start;
