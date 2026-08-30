use crate::models::projects::MonetizationStatus;
use crate::routes::ApiError;
use chrono::{DateTime, Datelike, Duration, TimeZone, Utc};
use dashmap::DashMap;
use futures::TryStreamExt;
use rust_decimal::Decimal;
use serde::Deserialize;
use sqlx::PgPool;
use sqlx::postgres::PgQueryResult;
use std::collections::HashMap;

/// 提现通道占位。
///
/// 旧版 PayPal / Tremendous / Venmo 客户端已全部下线；
/// 该结构保留下来给后续接入云账户(Yunzhanghu)等支付方使用，
/// 这样 `web::Data<PayoutsQueue>` 的依赖注入与各路由 extractor 不需要改动。
pub struct PayoutsQueue;

impl Default for PayoutsQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl PayoutsQueue {
    pub fn new() -> Self {
        PayoutsQueue
    }
}

#[derive(Deserialize)]
pub struct AditudePoints {
    #[serde(rename = "pointsList")]
    pub points_list: Vec<AditudePoint>,
}

#[derive(Deserialize)]
pub struct AditudePoint {
    pub metric: AditudeMetric,
    pub time: AditudeTime,
}

#[derive(Deserialize)]
pub struct AditudeMetric {
    pub revenue: Option<Decimal>,
    pub impressions: Option<u128>,
    pub cpm: Option<Decimal>,
}

#[derive(Deserialize)]
pub struct AditudeTime {
    pub seconds: u64,
}

pub async fn make_aditude_request(
    metrics: &[&str],
    range: &str,
    interval: &str,
) -> Result<Vec<AditudePoints>, ApiError> {
    let request = reqwest::Client::new()
        .post("https://cloud.aditude.io/api/public/insights/metrics")
        .bearer_auth(&dotenvy::var("ADITUDE_API_KEY")?)
        .json(&serde_json::json!({
            "metrics": metrics,
            "range": range,
            "interval": interval
        }))
        .send()
        .await?
        .error_for_status()?;

    let text = request.text().await?;

    let json: Vec<AditudePoints> = serde_json::from_str(&text)?;

    Ok(json)
}

pub async fn process_payout(
    pool: &PgPool,
    client: &clickhouse::Client,
) -> Result<(), ApiError> {
    let start: DateTime<Utc> = DateTime::from_naive_utc_and_offset(
        (Utc::now() - Duration::days(1))
            .date_naive()
            .and_hms_nano_opt(0, 0, 0, 0)
            .unwrap_or_default(),
        Utc,
    );

    let results = sqlx::query!(
        "SELECT EXISTS(SELECT 1 FROM payouts_values WHERE created = $1)",
        start,
    )
    .fetch_one(pool)
    .await?;

    if results.exists.unwrap_or(false) {
        return Ok(());
    }

    let end = start + Duration::days(1);
    #[derive(Deserialize, clickhouse::Row)]
    struct ProjectMultiplier {
        pub page_views: u64,
        pub project_id: u64,
    }

    let (views_values, views_sum, downloads_values, downloads_sum) = futures::future::try_join4(
        client
            .query(
                r#"
                SELECT COUNT(1) page_views, project_id
                FROM views
                WHERE (recorded BETWEEN ? AND ?) AND (project_id != 0) AND (monetized = TRUE)
                GROUP BY project_id
                ORDER BY page_views DESC
                "#,
            )
            .bind(start.timestamp())
            .bind(end.timestamp())
            .fetch_all::<ProjectMultiplier>(),
        client
            .query("SELECT COUNT(1) FROM views WHERE (recorded BETWEEN ? AND ?) AND (project_id != 0) AND (monetized = TRUE)")
            .bind(start.timestamp())
            .bind(end.timestamp())
            .fetch_one::<u64>(),
        client
            .query(
                r#"
                SELECT COUNT(1) page_views, project_id
                FROM downloads
                WHERE (recorded BETWEEN ? AND ?) AND (user_id != 0)
                GROUP BY project_id
                ORDER BY page_views DESC
                "#,
            )
            .bind(start.timestamp())
            .bind(end.timestamp())
            .fetch_all::<ProjectMultiplier>(),
        client
            .query("SELECT COUNT(1) FROM downloads WHERE (recorded BETWEEN ? AND ?) AND (user_id != 0)")
            .bind(start.timestamp())
            .bind(end.timestamp())
            .fetch_one::<u64>(),
    )
        .await?;

    let mut transaction = pool.begin().await?;

    struct PayoutMultipliers {
        sum: u64,
        values: HashMap<u64, u64>,
    }

    let mut views_values = views_values
        .into_iter()
        .map(|x| (x.project_id, x.page_views))
        .collect::<HashMap<u64, u64>>();
    let downloads_values = downloads_values
        .into_iter()
        .map(|x| (x.project_id, x.page_views))
        .collect::<HashMap<u64, u64>>();

    for (key, value) in downloads_values.iter() {
        let counter = views_values.entry(*key).or_insert(0);
        *counter += *value;
    }

    let multipliers: PayoutMultipliers = PayoutMultipliers {
        sum: downloads_sum + views_sum,
        values: views_values,
    };

    struct Project {
        // user_id, payouts_split
        team_members: Vec<(i64, Decimal)>,
    }

    let mut projects_map: HashMap<i64, Project> = HashMap::new();

    let project_ids = multipliers
        .values
        .keys()
        .map(|x| *x as i64)
        .collect::<Vec<i64>>();

    let project_org_members = sqlx::query!(
        "
        SELECT m.id id, tm.user_id user_id, tm.payouts_split payouts_split
        FROM mods m
        INNER JOIN organizations o ON m.organization_id = o.id
        INNER JOIN team_members tm on o.team_id = tm.team_id AND tm.accepted = TRUE
        WHERE m.id = ANY($1) AND m.monetization_status = $2 AND m.status = ANY($3) AND m.organization_id IS NOT NULL
        ",
        &project_ids,
        MonetizationStatus::Monetized.as_str(),
        &*crate::models::projects::ProjectStatus::iterator()
            .filter(|x| !x.is_hidden())
            .map(|x| x.to_string())
            .collect::<Vec<String>>(),
    )
    .fetch(&mut *transaction)
    .try_fold(DashMap::new(), |acc: DashMap<i64, HashMap<i64, Decimal>>, r| {
        acc.entry(r.id)
            .or_default()
            .insert(r.user_id, r.payouts_split);
        async move { Ok(acc) }
    })
    .await?;

    let project_team_members = sqlx::query!(
        "
        SELECT m.id id, tm.user_id user_id, tm.payouts_split payouts_split
        FROM mods m
        INNER JOIN team_members tm on m.team_id = tm.team_id AND tm.accepted = TRUE
        WHERE m.id = ANY($1) AND m.monetization_status = $2 AND m.status = ANY($3)
        ",
        &project_ids,
        MonetizationStatus::Monetized.as_str(),
        &*crate::models::projects::ProjectStatus::iterator()
            .filter(|x| !x.is_hidden())
            .map(|x| x.to_string())
            .collect::<Vec<String>>(),
    )
    .fetch(&mut *transaction)
    .try_fold(
        DashMap::new(),
        |acc: DashMap<i64, HashMap<i64, Decimal>>, r| {
            acc.entry(r.id)
                .or_default()
                .insert(r.user_id, r.payouts_split);
            async move { Ok(acc) }
        },
    )
    .await?;

    for project_id in project_ids {
        let team_members: HashMap<i64, Decimal> = project_team_members
            .remove(&project_id)
            .unwrap_or((0, HashMap::new()))
            .1;
        let org_team_members: HashMap<i64, Decimal> = project_org_members
            .remove(&project_id)
            .unwrap_or((0, HashMap::new()))
            .1;

        let mut all_team_members = vec![];

        for (user_id, payouts_split) in org_team_members {
            if !team_members.contains_key(&user_id) {
                all_team_members.push((user_id, payouts_split));
            }
        }
        for (user_id, payouts_split) in team_members {
            all_team_members.push((user_id, payouts_split));
        }

        // if all team members are set to zero, we treat as an equal revenue distribution
        if all_team_members.iter().all(|x| x.1 == Decimal::ZERO) {
            all_team_members
                .iter_mut()
                .for_each(|x| x.1 = Decimal::from(1));
        }

        projects_map.insert(
            project_id,
            Project {
                team_members: all_team_members,
            },
        );
    }

    let aditude_res = make_aditude_request(
        &["METRIC_IMPRESSIONS", "METRIC_REVENUE"],
        "Yesterday",
        "1d",
    )
    .await?;

    let aditude_amount: Decimal = aditude_res
        .iter()
        .map(|x| {
            x.points_list
                .iter()
                .filter_map(|x| x.metric.revenue)
                .sum::<Decimal>()
        })
        .sum();
    let aditude_impressions: u128 = aditude_res
        .iter()
        .map(|x| {
            x.points_list
                .iter()
                .filter_map(|x| x.metric.impressions)
                .sum::<u128>()
        })
        .sum();

    // BBSMC 的广告收入分成
    let platform_cut = Decimal::from(1) / Decimal::from(4);
    // Clean.io fee (ad antimalware). Per 1000 impressions.
    let clean_io_fee = Decimal::from(8) / Decimal::from(1000);

    let net_revenue = aditude_amount
        - (clean_io_fee * Decimal::from(aditude_impressions)
            / Decimal::from(1000));

    let payout = net_revenue * (Decimal::from(1) - platform_cut);

    // Ad payouts are Net 60 from the end of the month
    let available = {
        let now = Utc::now().date_naive();

        let year = now.year();
        let month = now.month();

        // Get the first day of the next month
        let last_day_of_month = if month == 12 {
            Utc.with_ymd_and_hms(year + 1, 1, 1, 0, 0, 0).unwrap()
        } else {
            Utc.with_ymd_and_hms(year, month + 1, 1, 0, 0, 0).unwrap()
        };

        last_day_of_month + Duration::days(59)
    };

    let (
        mut insert_user_ids,
        mut insert_project_ids,
        mut insert_payouts,
        mut insert_starts,
        mut insert_availables,
    ) = (Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for (id, project) in projects_map {
        if let Some(value) = &multipliers.values.get(&(id as u64)) {
            let project_multiplier: Decimal =
                Decimal::from(**value) / Decimal::from(multipliers.sum);

            let sum_splits: Decimal =
                project.team_members.iter().map(|x| x.1).sum();

            if sum_splits > Decimal::ZERO {
                for (user_id, split) in project.team_members {
                    let payout: Decimal =
                        payout * project_multiplier * (split / sum_splits);

                    if payout > Decimal::ZERO {
                        insert_user_ids.push(user_id);
                        insert_project_ids.push(id);
                        insert_payouts.push(payout);
                        insert_starts.push(start);
                        insert_availables.push(available);
                    }
                }
            }
        }
    }

    sqlx::query!(
        "
        INSERT INTO payouts_values (user_id, mod_id, amount, created, date_available)
        SELECT * FROM UNNEST ($1::bigint[], $2::bigint[], $3::numeric[], $4::timestamptz[], $5::timestamptz[])
        ",
        &insert_user_ids[..],
        &insert_project_ids[..],
        &insert_payouts[..],
        &insert_starts[..],
        &insert_availables[..]
    )
    .execute(&mut *transaction)
    .await?;

    transaction.commit().await?;

    Ok(())
}

// Used for testing, should be the same as the above function
pub async fn insert_payouts(
    insert_user_ids: Vec<i64>,
    insert_project_ids: Vec<i64>,
    insert_payouts: Vec<Decimal>,
    insert_starts: Vec<DateTime<Utc>>,
    insert_availables: Vec<DateTime<Utc>>,
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
) -> sqlx::Result<PgQueryResult> {
    sqlx::query!(
        "
        INSERT INTO payouts_values (user_id, mod_id, amount, created, date_available)
        SELECT * FROM UNNEST ($1::bigint[], $2::bigint[], $3::numeric[], $4::timestamptz[], $5::timestamptz[])
        ",
        &insert_user_ids[..],
        &insert_project_ids[..],
        &insert_payouts[..],
        &insert_starts[..],
        &insert_availables[..],
    )
    .execute(&mut **transaction)
    .await
}
