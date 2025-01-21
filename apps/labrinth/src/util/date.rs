use chrono::Utc;

// 将时间戳转换为 ClickHouse 所需的格式
pub fn get_current_tenths_of_ms() -> i64 {
    Utc::now()
        .timestamp_nanos_opt()
        .expect("无法以纳秒精度表示该值.")
        / 100_000
}
