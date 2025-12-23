use actix_cors::Cors;

// 更新这里时请同时更新 ratelimit.rs 中的 CORS 设置！
pub fn default_cors() -> Cors {
    Cors::default()
        .allow_any_origin()
        .allow_any_header()
        .allow_any_method()
        .max_age(3600)
        .send_wildcard()
}
