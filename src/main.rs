use std::{ops::ControlFlow, time::Duration};

use grammers_client::{session::Session, Client, Config, InitParams, ReconnectionPolicy};
use grammers_friendly::prelude::*;
use holy_maiden_bot::{
    middlewares::{SaveChat, SetLocale},
    modules::{Conversation, Database, I18n},
    routers, Result,
};

const SESSION_FILE: &str = "./holy_maiden.session";

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    // Initialize the logger
    env_logger::init();

    // Load the environment variables
    dotenvy::dotenv()?;

    // Load the configuration
    let config = holy_maiden_bot::Config::load()?;

    // Connect the client
    log::info!("connecting bot...");
    let client = Client::connect(Config {
        session: Session::load_file_or_create(SESSION_FILE)?,
        api_id: config.telegram.api_id,
        api_hash: config.telegram.api_hash,
        params: InitParams {
            catch_up: config.bot.catch_up,
            reconnection_policy: &AutoReconnectPolicy,
            flood_sleep_threshold: config.bot.flood_sleep_threshold,
            ..Default::default()
        },
    })
    .await?;
    log::info!("bot connected");

    if !client.is_authorized().await? {
        client.bot_sign_in(&config.bot.token).await?;
        client.session().save_to_file(SESSION_FILE)?;
        log::info!("bot authorized");
    }

    // Dispatcher
    Dispatcher::default()
        .add_module(Database::connect().await)
        .add_module(I18n::new("en-GB"))
        .add_module(Conversation::new(client.clone()))
        .add_middleware(Middleware::before(SaveChat))
        .add_middleware(Middleware::before(SetLocale))
        .add_router(routers::start())
        .add_router(routers::help())
        .add_router(routers::language())
        .add_router(routers::character())
        .add_router(routers::series())
        .add_router(routers::list())
        .add_router(routers::collect())
        .add_router(routers::admin())
        .add_router(routers::send_character())
        .ignore_updates_from_self(true)
        .run(client.clone())
        .await?;

    // Save the session
    client.session().save_to_file(SESSION_FILE)?;
    log::info!("session saved");

    Ok(())
}

struct AutoReconnectPolicy;

impl ReconnectionPolicy for AutoReconnectPolicy {
    fn should_retry(&self, attempts: usize) -> ControlFlow<(), Duration> {
        let duration = u64::pow(2, attempts as _);
        log::info!("reconnecting in {} seconds", duration);
        ControlFlow::Continue(Duration::from_millis(duration))
    }
}
