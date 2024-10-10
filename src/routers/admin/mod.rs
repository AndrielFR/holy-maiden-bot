mod character;
mod series;

use grammers_friendly::Router;

pub fn router() -> Router {
    Router::default()
        .add_sub_router(character::router())
        .add_sub_router(series::router())
}
