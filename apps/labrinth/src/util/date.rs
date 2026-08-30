use chrono::{DateTime, FixedOffset, Utc};

/// 应用统一使用的时区名（北京时间），同时配置在 PG `SET TIME ZONE`
/// 与 ClickHouse `session_timezone` / `toStartOf*` 第三参数中。
///
/// 集中维护是为了：
/// 1. 修改时区只改一处
/// 2. 提醒新写代码时与 SQL 约定保持一致
pub const APP_TZ_NAME: &str = "Asia/Shanghai";

/// 将 [`APP_TZ_NAME`] 表示为 [`FixedOffset`]，用于 chrono 的格式化与显示。
///
/// 中国大陆不实施夏令时，固定偏移即可，不需要 `chrono-tz`。
#[inline]
pub fn app_tz() -> FixedOffset {
    FixedOffset::east_opt(8 * 3600).expect("APP_TZ_NAME 偏移构造失败")
}

/// 将 UTC 时间转换为应用时区时间，便于 `format` 输出。
#[inline]
pub fn to_app_tz(dt: DateTime<Utc>) -> DateTime<FixedOffset> {
    dt.with_timezone(&app_tz())
}

/// 当前应用时区时间，常用于日志或返回给前端展示。
#[inline]
pub fn app_now() -> DateTime<FixedOffset> {
    to_app_tz(Utc::now())
}

/// 按 `%Y-%m-%d %H:%M:%S` 格式化为应用时区字符串。
#[inline]
pub fn format_app_tz(dt: DateTime<Utc>) -> String {
    to_app_tz(dt).format("%Y-%m-%d %H:%M:%S").to_string()
}

// 将时间戳转换为 ClickHouse 所需的格式
pub fn get_current_tenths_of_ms() -> i64 {
    Utc::now()
        .timestamp_nanos_opt()
        .expect("无法以纳秒精度表示该值.")
        / 100_000
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn app_tz_is_plus_eight() {
        assert_eq!(app_tz().local_minus_utc(), 8 * 3600);
    }

    #[test]
    fn format_app_tz_outputs_beijing_time() {
        let dt = Utc.with_ymd_and_hms(2026, 5, 9, 0, 0, 0).unwrap();
        // UTC 0:00 → 北京 8:00
        assert_eq!(format_app_tz(dt), "2026-05-09 08:00:00");
    }
}
