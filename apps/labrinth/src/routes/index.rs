use actix_web::{HttpResponse, get};
use serde_json::json;

#[get("/")]
pub async fn index_get() -> HttpResponse {
    let data = json!({
        "name": "BBSMC",
        "version": env!("CARGO_PKG_VERSION"),
        "documentation": "https://bbsmc.net",
        "about": "Welcome traveler!"
    });

    HttpResponse::Ok().json(data)
}
