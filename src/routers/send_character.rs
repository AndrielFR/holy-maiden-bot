use grammers_client::{Client, Update};
use grammers_friendly::prelude::*;

use crate::{middlewares::SendCharacter, Result};

pub fn router() -> Router {
    Router::default()
        .add_middleware(Middleware::before(SendCharacter::new(37..40))) // TEMP: Tests porpuse
        .add_handler(Handler::new_message(mock, filters::private().not()))
}

// Just a mock handler, to let the middleware run
pub async fn mock(_client: &mut Client, _update: &mut Update, _data: &mut Data) -> Result<()> {
    Ok(())
}
