use actix_web::guard::GuardContext;

pub const ADMIN_KEY_HEADER: &str = "Modrinth-Admin";
pub fn admin_key_guard(ctx: &GuardContext) -> bool {
    let admin_key = std::env::var("LABRINTH_ADMIN_KEY")
        .expect("未提供管理员密钥，这应该在 check_env_vars 中被捕获");
    ctx.head()
        .headers()
        .get(ADMIN_KEY_HEADER)
        .is_some_and(|it| it.as_bytes() == admin_key.as_bytes())
}
