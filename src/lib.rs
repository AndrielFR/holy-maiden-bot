mod config;
pub mod database;
pub mod filters;
pub mod middlewares;
pub mod modules;
pub mod routers;
pub mod utils;

pub use config::Config;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
