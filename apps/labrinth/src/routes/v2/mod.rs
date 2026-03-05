mod bans;
mod creator;
mod moderation;
mod notifications;
pub(crate) mod project_creation;
mod projects;
mod reports;
mod statistics;
pub mod tags;
mod teams;
mod threads;
mod users;
mod version_creation;
pub mod version_file;
mod versions;

pub use super::ApiError;
use crate::util::cors::default_cors;

pub fn config(cfg: &mut actix_web::web::ServiceConfig) {
    cfg.service(
        actix_web::web::scope("v2")
            .wrap(default_cors())
            .configure(super::internal::admin::config)
            // Todo: 分离这些- 它们也需要遵循 v2-v3 转换
            .configure(super::internal::session::config)
            .configure(super::internal::flows::config)
            .configure(super::internal::pats::config)
            .configure(bans::config)
            .configure(creator::config)
            .configure(moderation::config)
            .configure(notifications::config)
            .configure(project_creation::config)
            .configure(projects::config)
            .configure(reports::config)
            .configure(statistics::config)
            .configure(tags::config)
            .configure(teams::config)
            .configure(threads::config)
            .configure(users::config)
            .configure(version_file::config)
            .configure(versions::config),
    );
}
