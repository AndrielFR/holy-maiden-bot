mod character;

use grammers_friendly::Router;

pub fn router() -> Router {
    Router::default().add_sub_router(character::router())
}
