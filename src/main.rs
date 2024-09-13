use grammers_client::{Client, Config, InitParams};
use grammers_friendly::prelude::*;
use grammers_session::Session;
use holy_maiden_bot::{
    handlers,
    middlewares::{SaveChat, SendCharacter, SetLocale},
    modules::{Database, I18n},
    Result,
};

const SESSION_FILE: &str = "holy_maiden.session";

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    // Initialize the logger
    env_logger::init();

    // Load the configuration
    let config = holy_maiden_bot::Config::load()?;

    // Connect the client
    log::info!("connecting bot...");
    let client = Client::connect(Config {
        session: Session::load_file_or_create(SESSION_FILE)?,
        api_id: config.telegram.api_id,
        api_hash: config.telegram.api_hash.clone(),
        params: InitParams {
            catch_up: config.bot.catch_up,
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

    // Database
    let db = Database::default();
    db.connect().await;

    // Dispatcher
    Dispatcher::default()
        .add_module(db)
        .add_module(I18n::new("en-GB"))
        .add_middleware(Middleware::before(SaveChat))
        .add_middleware(Middleware::before(SetLocale::default()))
        .add_middleware(Middleware::before(SendCharacter::new(80..160)))
        .add_router(handlers::start())
        .add_router(handlers::help())
        .add_router(handlers::language())
        .add_router(handlers::collect())
        .add_router(handlers::list())
        .run(client.clone())
        .await?;

    // Save the session
    client.session().save_to_file(SESSION_FILE)?;
    log::info!("session saved");

    Ok(())
}
