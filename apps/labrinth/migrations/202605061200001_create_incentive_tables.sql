-- 1. 项目激励准入开关
CREATE TABLE incentive_enabled_projects (
    project_id  bigint PRIMARY KEY REFERENCES mods(id) ON DELETE CASCADE,
    enabled_at  timestamptz NOT NULL DEFAULT NOW(),
    enabled_by  bigint NOT NULL REFERENCES users(id),
    notes       text
);

-- 2. 项目累计计数器 + 待发金额
CREATE TABLE incentive_project_counters (
    project_id              bigint PRIMARY KEY REFERENCES mods(id) ON DELETE CASCADE,
    lifetime_eff_downloads  bigint NOT NULL DEFAULT 0,
    pending_amount          numeric(12, 4) NOT NULL DEFAULT 0,
    settled_amount          numeric(12, 4) NOT NULL DEFAULT 0,
    voided_amount           numeric(12, 4) NOT NULL DEFAULT 0,
    last_event_at           timestamptz,
    updated_at              timestamptz NOT NULL DEFAULT NOW()
);

-- 3. 每次有效下载事件（双判重 + 状态机）
CREATE TABLE incentive_download_events (
    id              bigserial PRIMARY KEY,
    project_id      bigint NOT NULL REFERENCES mods(id) ON DELETE CASCADE,
    team_id         bigint NOT NULL,
    user_identity   text,
    ip_identity     text NOT NULL,
    week_bucket     bigint NOT NULL,
    payout_amount   numeric(12, 6) NOT NULL,
    status          text NOT NULL DEFAULT 'pending',
    recorded_at     timestamptz NOT NULL DEFAULT NOW(),
    settled_at      timestamptz
);

CREATE UNIQUE INDEX idx_incentive_evt_user_dedup
    ON incentive_download_events (project_id, user_identity, week_bucket)
    WHERE user_identity IS NOT NULL;

CREATE UNIQUE INDEX idx_incentive_evt_ip_dedup
    ON incentive_download_events (project_id, ip_identity, week_bucket);

CREATE INDEX idx_incentive_evt_pending_release
    ON incentive_download_events (recorded_at)
    WHERE status = 'pending';

CREATE INDEX idx_incentive_evt_project_recorded
    ON incentive_download_events (project_id, recorded_at);

-- 4. 异常下载告警
CREATE TABLE incentive_anomaly_alerts (
    id              bigserial PRIMARY KEY,
    project_id      bigint NOT NULL REFERENCES mods(id) ON DELETE CASCADE,
    alert_date      date NOT NULL,
    daily_count     bigint NOT NULL,
    baseline_avg    numeric(10, 2) NOT NULL,
    ratio           numeric(8, 2) NOT NULL,
    handled         boolean NOT NULL DEFAULT FALSE,
    handled_by      bigint REFERENCES users(id),
    handled_at      timestamptz,
    notes           text,
    created_at      timestamptz NOT NULL DEFAULT NOW(),
    UNIQUE (project_id, alert_date)
);

CREATE INDEX idx_incentive_alert_unhandled
    ON incentive_anomaly_alerts (created_at DESC)
    WHERE handled = FALSE;
