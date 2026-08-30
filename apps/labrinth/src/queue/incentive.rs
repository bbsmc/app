use crate::routes::ApiError;
use dashmap::DashMap;
use rust_decimal::Decimal;
use rust_decimal::prelude::FromPrimitive;
use sqlx::PgPool;
use std::collections::HashSet;
use std::net::Ipv6Addr;
use std::str::FromStr;

const WEEK_SECS: i64 = 7 * 86_400;

const TIER1_END: i64 = 1_000;
const TIER2_END: i64 = 10_000;
const RATE_TIER1: &str = "0.02";
const RATE_TIER2: &str = "0.01";
const RATE_TIER3: &str = "0.008";

#[derive(serde::Deserialize, Clone, Debug, PartialEq)]
pub(crate) struct SnapshotMember {
    pub user_id: i64,
    pub split: f64,
}

pub(crate) struct ProjectPayoutMembers {
    pub team_id: i64,
    pub members: Vec<SnapshotMember>,
}

#[derive(Clone, Debug)]
pub struct IncentiveEvent {
    pub project_id: u64,
    pub user_id: u64,
    pub user_identity: Option<String>,
    pub ip_identity: String,
    pub week_bucket: i64,
}

pub struct IncentiveQueue {
    pending: DashMap<(u64, String, i64), IncentiveEvent>,
}

impl Default for IncentiveQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl IncentiveQueue {
    pub fn new() -> Self {
        Self {
            pending: DashMap::with_capacity(1024),
        }
    }

    pub fn add(
        &self,
        project_id: u64,
        user_id: u64,
        ip: Ipv6Addr,
        recorded_at_secs: i64,
    ) {
        let user_identity = if user_id != 0 {
            Some(format!("u_{user_id}"))
        } else {
            None
        };
        let ip_identity = crate::util::ip::ip_to_identity_64(ip);
        let week_bucket = recorded_at_secs / WEEK_SECS;

        let key = (project_id, ip_identity.clone(), week_bucket);
        self.pending.entry(key).or_insert(IncentiveEvent {
            project_id,
            user_id,
            user_identity,
            ip_identity,
            week_bucket,
        });
    }

    pub async fn index(&self, pool: &PgPool) -> Result<(), ApiError> {
        let snapshot: Vec<IncentiveEvent> =
            self.pending.iter().map(|e| e.value().clone()).collect();
        self.pending.clear();

        for evt in snapshot {
            if let Err(e) = process_event(&evt, pool).await {
                tracing::warn!(
                    "incentive event failed (project {}): {:?}",
                    evt.project_id,
                    e
                );
            }
        }

        Ok(())
    }
}

async fn process_event(
    evt: &IncentiveEvent,
    pool: &PgPool,
) -> Result<(), sqlx::Error> {
    // 1. 仅已审核开通激励的项目累计奖励。
    let enabled = sqlx::query!(
        r#"SELECT EXISTS(SELECT 1 FROM incentive_enabled_projects WHERE project_id = $1) AS "exists!""#,
        evt.project_id as i64,
    )
    .fetch_one(pool)
    .await?
    .exists;
    if !enabled {
        return Ok(());
    }

    // 2. 取项目有效分账成员；若下载用户是项目/组织成员则跳过
    let project_payout =
        match current_project_payout_members(pool, evt.project_id as i64)
            .await?
        {
            Some(p) => p,
            None => return Ok(()),
        };

    if evt.user_id != 0 {
        let is_project_member = project_payout
            .members
            .iter()
            .any(|m| m.user_id == evt.user_id as i64);
        if is_project_member {
            return Ok(());
        }
    }

    // 3. 取 split 快照（事件发生时点的归属比例）。保留 split=0 的成员；
    // 项目 team 覆盖组织 team，结算时若全员为 0 则按旧收益逻辑等分。
    let split_snapshot = serde_json::json!(
        project_payout
            .members
            .iter()
            .map(|m| serde_json::json!({
                "user_id": m.user_id,
                "split": m.split,
            }))
            .collect::<Vec<_>>()
    );

    // 4. 事务：行锁 lifetime → 计算单价 → 插入事件 → 更新计数器
    let mut tx = pool.begin().await?;

    let lifetime_row = sqlx::query!(
        "
        SELECT lifetime_eff_downloads FROM incentive_project_counters
        WHERE project_id = $1 FOR UPDATE
        ",
        evt.project_id as i64,
    )
    .fetch_optional(&mut *tx)
    .await?;
    let lifetime = lifetime_row.map(|r| r.lifetime_eff_downloads).unwrap_or(0);

    let payout = next_unit_payout(lifetime);

    let inserted = sqlx::query!(
        "
        INSERT INTO incentive_download_events
        (project_id, team_id, user_identity, ip_identity, week_bucket, payout_amount, status, split_snapshot)
        VALUES ($1, $2, $3, $4, $5, $6, 'pending', $7)
        ON CONFLICT DO NOTHING
        ",
        evt.project_id as i64,
        project_payout.team_id,
        evt.user_identity.as_deref(),
        evt.ip_identity,
        evt.week_bucket,
        payout,
        split_snapshot,
    )
    .execute(&mut *tx)
    .await?;

    if inserted.rows_affected() == 0 {
        tx.rollback().await?;
        return Ok(());
    }

    sqlx::query!(
        "
        INSERT INTO incentive_project_counters
            (project_id, lifetime_eff_downloads, pending_amount, last_event_at, updated_at)
        VALUES ($1, 1, $2, NOW(), NOW())
        ON CONFLICT (project_id) DO UPDATE SET
            lifetime_eff_downloads = incentive_project_counters.lifetime_eff_downloads + 1,
            pending_amount = incentive_project_counters.pending_amount + EXCLUDED.pending_amount,
            last_event_at = NOW(),
            updated_at = NOW()
        ",
        evt.project_id as i64,
        payout,
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(())
}

fn next_unit_payout(current_lifetime: i64) -> Decimal {
    let s = if current_lifetime < TIER1_END {
        RATE_TIER1
    } else if current_lifetime < TIER2_END {
        RATE_TIER2
    } else {
        RATE_TIER3
    };
    Decimal::from_str(s).unwrap_or_default()
}

pub(crate) fn payable_members(
    mut members: Vec<SnapshotMember>,
) -> Vec<SnapshotMember> {
    members.retain(|m| m.split.is_finite() && m.split >= 0.0);
    if members.is_empty() {
        return members;
    }

    let total_positive: f64 = members
        .iter()
        .filter(|m| m.split > 0.0)
        .map(|m| m.split)
        .sum();

    if total_positive <= 0.0 {
        for member in &mut members {
            member.split = 1.0;
        }
        members
    } else {
        members.into_iter().filter(|m| m.split > 0.0).collect()
    }
}

fn merge_project_payout_members(
    mut org_members: Vec<SnapshotMember>,
    project_members: Vec<SnapshotMember>,
) -> Vec<SnapshotMember> {
    let project_user_ids: HashSet<i64> =
        project_members.iter().map(|m| m.user_id).collect();
    org_members.retain(|m| !project_user_ids.contains(&m.user_id));

    let mut members = org_members;
    members.extend(project_members);
    members.sort_by_key(|m| m.user_id);
    members
}

async fn current_team_members(
    pool: &PgPool,
    team_id: i64,
) -> Result<Vec<SnapshotMember>, sqlx::Error> {
    let rows = sqlx::query!(
        "
        SELECT user_id, payouts_split
        FROM team_members
        WHERE team_id = $1 AND accepted = TRUE
        ORDER BY user_id
        ",
        team_id,
    )
    .fetch_all(pool)
    .await?;

    use rust_decimal::prelude::ToPrimitive;
    Ok(rows
        .into_iter()
        .map(|r| SnapshotMember {
            user_id: r.user_id,
            split: r.payouts_split.to_f64().unwrap_or(0.0),
        })
        .collect())
}

pub(crate) async fn current_project_payout_members(
    pool: &PgPool,
    project_id: i64,
) -> Result<Option<ProjectPayoutMembers>, sqlx::Error> {
    let project = sqlx::query!(
        r#"
        SELECT m.team_id, o.team_id AS "organization_team_id?"
        FROM mods m
        LEFT JOIN organizations o ON m.organization_id = o.id
        WHERE m.id = $1
        "#,
        project_id,
    )
    .fetch_optional(pool)
    .await?;

    let Some(project) = project else {
        return Ok(None);
    };

    let org_members = match project.organization_team_id {
        Some(team_id) => current_team_members(pool, team_id).await?,
        None => Vec::new(),
    };
    let project_members = current_team_members(pool, project.team_id).await?;

    Ok(Some(ProjectPayoutMembers {
        team_id: project.team_id,
        members: merge_project_payout_members(org_members, project_members),
    }))
}

/// 7 天前 pending 的事件结算到 payouts_values，按事件发生时的 split 快照拆分
pub async fn settle_pending(pool: &PgPool) -> Result<u64, sqlx::Error> {
    let events = sqlx::query!(
        "
        SELECT id, project_id, team_id, payout_amount, split_snapshot
        FROM incentive_download_events
        WHERE status = 'pending'
          AND recorded_at < NOW() - INTERVAL '7 days'
        ORDER BY id ASC
        LIMIT 20000
        ",
    )
    .fetch_all(pool)
    .await?;

    let mut settled: u64 = 0;

    for e in events {
        // 优先使用事件快照；缺失或旧版空快照回退到当前成员。
        let mut members: Vec<SnapshotMember> = match &e.split_snapshot {
            Some(v) => serde_json::from_value(v.clone()).unwrap_or_default(),
            None => current_project_payout_members(pool, e.project_id)
                .await?
                .map(|p| p.members)
                .unwrap_or_default(),
        };

        if members.is_empty() {
            members = current_project_payout_members(pool, e.project_id)
                .await?
                .map(|p| p.members)
                .unwrap_or_default();
        }
        let members = payable_members(members);
        let total_split: f64 = members.iter().map(|m| m.split).sum();

        let mut tx = pool.begin().await?;

        if members.is_empty() || total_split <= 0.0 {
            // 没有任何 accepted 成员，才视为无可分账对象并作废。
            sqlx::query!(
                "
                UPDATE incentive_download_events
                SET status = 'voided', settled_at = NOW()
                WHERE id = $1
                ",
                e.id,
            )
            .execute(&mut *tx)
            .await?;
            sqlx::query!(
                "
                UPDATE incentive_project_counters
                SET pending_amount = pending_amount - $2,
                    voided_amount = voided_amount + $2,
                    updated_at = NOW()
                WHERE project_id = $1
                ",
                e.project_id,
                e.payout_amount,
            )
            .execute(&mut *tx)
            .await?;
            tx.commit().await?;
            continue;
        }

        // 按 split 比例拆分写入 payouts_values（使用事件快照）
        for m in &members {
            let frac = m.split / total_split;
            let frac_dec = Decimal::from_f64(frac).unwrap_or_default();
            let amount = e.payout_amount * frac_dec;

            sqlx::query!(
                "
                INSERT INTO payouts_values (user_id, mod_id, amount, created, date_available)
                VALUES ($1, $2, $3, NOW(), NOW())
                ",
                m.user_id,
                e.project_id,
                amount,
            )
            .execute(&mut *tx)
            .await?;
        }

        sqlx::query!(
            "
            UPDATE incentive_download_events
            SET status = 'settled', settled_at = NOW()
            WHERE id = $1
            ",
            e.id,
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query!(
            "
            UPDATE incentive_project_counters
            SET pending_amount = pending_amount - $2,
                settled_amount = settled_amount + $2,
                updated_at = NOW()
            WHERE project_id = $1
            ",
            e.project_id,
            e.payout_amount,
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        settled += 1;
    }

    Ok(settled)
}

/// 清理 3 个月前已终结的事件明细。pending 不删除，避免异常时丢失未结算金额。
pub async fn cleanup_old_events(pool: &PgPool) -> Result<u64, sqlx::Error> {
    let deleted = sqlx::query!(
        "
        DELETE FROM incentive_download_events
        WHERE recorded_at < NOW() - INTERVAL '3 months'
          AND status IN ('settled', 'voided')
        ",
    )
    .execute(pool)
    .await?
    .rows_affected();

    Ok(deleted)
}

/// 异常监测：阈值通过环境变量配置，避免攻击者按已知阈值精确绕过
pub async fn detect_anomalies(pool: &PgPool) -> Result<u64, sqlx::Error> {
    let ratio: f64 = dotenvy::var("INCENTIVE_ANOMALY_RATIO")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(3.0);
    let min_count: i64 = dotenvy::var("INCENTIVE_ANOMALY_MIN_COUNT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(100);

    let ratio_dec = Decimal::from_f64(ratio).unwrap_or_default();

    let inserted = sqlx::query!(
        "
        WITH today AS (
            SELECT project_id, COUNT(*)::bigint AS cnt
            FROM incentive_download_events
            WHERE recorded_at >= CURRENT_DATE
              AND recorded_at < CURRENT_DATE + INTERVAL '1 day'
            GROUP BY project_id
        ),
        baseline AS (
            SELECT project_id, AVG(daily_cnt)::numeric AS avg_cnt
            FROM (
                SELECT project_id, DATE_TRUNC('day', recorded_at) AS d, COUNT(*) AS daily_cnt
                FROM incentive_download_events
                WHERE recorded_at >= CURRENT_DATE - INTERVAL '8 days'
                  AND recorded_at < CURRENT_DATE
                GROUP BY project_id, d
            ) sub
            GROUP BY project_id
        )
        INSERT INTO incentive_anomaly_alerts
            (project_id, alert_date, daily_count, baseline_avg, ratio)
        SELECT t.project_id, CURRENT_DATE, t.cnt, COALESCE(b.avg_cnt, 0),
               CASE WHEN COALESCE(b.avg_cnt, 0) = 0 THEN 999.99
                    ELSE ROUND((t.cnt::numeric / b.avg_cnt)::numeric, 2) END
        FROM today t LEFT JOIN baseline b USING (project_id)
        WHERE t.cnt >= $1
          AND t.cnt > GREATEST(COALESCE(b.avg_cnt, 0) * $2, $1::numeric)
        ON CONFLICT (project_id, alert_date) DO NOTHING
        ",
        min_count,
        ratio_dec,
    )
    .execute(pool)
    .await?
    .rows_affected();

    Ok(inserted)
}

/// 写一条审计日志（操作可追溯）
pub async fn audit_log(
    pool: &PgPool,
    actor_user_id: Option<i64>,
    action: &str,
    target_type: &str,
    target_id: i64,
    metadata: Option<serde_json::Value>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "
        INSERT INTO incentive_audit_log
            (actor_user_id, action, target_type, target_id, metadata)
        VALUES ($1, $2, $3, $4, $5)
        ",
        actor_user_id,
        action,
        target_type,
        target_id,
        metadata,
    )
    .execute(pool)
    .await?;
    Ok(())
}

/// 把指定项目所有 pending 事件转为 voided（关闭激励且选择清空 pending 时调用）
pub async fn void_project_pending(
    pool: &PgPool,
    project_id: i64,
) -> Result<i64, sqlx::Error> {
    let mut tx = pool.begin().await?;

    let voided_amount = sqlx::query_scalar!(
        r#"
        SELECT COALESCE(SUM(payout_amount), 0)::numeric AS "voided_amount!"
        FROM incentive_download_events
        WHERE project_id = $1 AND status = 'pending'
        "#,
        project_id,
    )
    .fetch_one(&mut *tx)
    .await?;

    let res = sqlx::query!(
        "
        UPDATE incentive_download_events
        SET status = 'voided', settled_at = NOW()
        WHERE project_id = $1 AND status = 'pending'
        ",
        project_id,
    )
    .execute(&mut *tx)
    .await?;

    if res.rows_affected() > 0 {
        sqlx::query!(
            "
            UPDATE incentive_project_counters
            SET pending_amount = 0,
                voided_amount = voided_amount + $2,
                updated_at = NOW()
            WHERE project_id = $1
            ",
            project_id,
            voided_amount,
        )
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(res.rows_affected() as i64)
}

#[cfg(test)]
mod tests {
    use super::{
        SnapshotMember, merge_project_payout_members, payable_members,
    };

    #[test]
    fn payable_members_equalizes_all_zero_splits() {
        let members = payable_members(vec![
            SnapshotMember {
                user_id: 1,
                split: 0.0,
            },
            SnapshotMember {
                user_id: 2,
                split: 0.0,
            },
        ]);

        assert_eq!(
            members,
            vec![
                SnapshotMember {
                    user_id: 1,
                    split: 1.0,
                },
                SnapshotMember {
                    user_id: 2,
                    split: 1.0,
                },
            ]
        );
    }

    #[test]
    fn payable_members_ignores_zero_when_positive_splits_exist() {
        let members = payable_members(vec![
            SnapshotMember {
                user_id: 1,
                split: 0.0,
            },
            SnapshotMember {
                user_id: 2,
                split: 25.0,
            },
        ]);

        assert_eq!(
            members,
            vec![SnapshotMember {
                user_id: 2,
                split: 25.0,
            }]
        );
    }

    #[test]
    fn merge_project_payout_members_project_overrides_org_member() {
        let members = merge_project_payout_members(
            vec![
                SnapshotMember {
                    user_id: 1,
                    split: 75.0,
                },
                SnapshotMember {
                    user_id: 2,
                    split: 25.0,
                },
            ],
            vec![
                SnapshotMember {
                    user_id: 1,
                    split: 0.0,
                },
                SnapshotMember {
                    user_id: 3,
                    split: 0.0,
                },
            ],
        );

        assert_eq!(
            members,
            vec![
                SnapshotMember {
                    user_id: 1,
                    split: 0.0,
                },
                SnapshotMember {
                    user_id: 2,
                    split: 25.0,
                },
                SnapshotMember {
                    user_id: 3,
                    split: 0.0,
                },
            ]
        );
    }
}
