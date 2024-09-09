mod config;
pub mod database;
pub mod handlers;
pub mod middlewares;
pub mod modules;

pub use config::Config;

pub type Result = std::result::Result<(), Box<dyn std::error::Error>>;
