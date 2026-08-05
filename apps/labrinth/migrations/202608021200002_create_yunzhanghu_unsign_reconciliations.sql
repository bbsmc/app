-- 云账户解约回调最终一致性核对。
--
-- 回调先把同一身份的本地资料切换为 release:webhook marker，再由带租约的
-- reconciliation 任务查询远端并原子收敛。表中只保存控制面元数据，不保存 PII。
CREATE TABLE yunzhanghu_unsign_reconciliations (
    nonce               text        PRIMARY KEY,
    release_at          timestamptz NOT NULL,
    status              text        NOT NULL DEFAULT 'pending',
    lease_owner         text,
    lease_expires_at    timestamptz,
    attempt_count       integer     NOT NULL DEFAULT 0,
    last_attempt_at     timestamptz,
    next_attempt_at     timestamptz NOT NULL DEFAULT NOW(),
    last_remote_status  integer,
    last_error          text,
    resolved_status     text,
    resolved_at         timestamptz,
    created_at          timestamptz NOT NULL DEFAULT NOW(),
    updated_at          timestamptz NOT NULL DEFAULT NOW(),

    CONSTRAINT yunzhanghu_unsign_reconciliations_status_check
        CHECK (status IN ('pending', 'resolved')),
    CONSTRAINT yunzhanghu_unsign_reconciliations_resolved_status_check
        CHECK (resolved_status IS NULL OR resolved_status IN ('signed', 'terminated', 'ignored')),
    CONSTRAINT yunzhanghu_unsign_reconciliations_attempt_count_check
        CHECK (attempt_count >= 0),
    CONSTRAINT yunzhanghu_unsign_reconciliations_lease_check
        CHECK ((lease_owner IS NULL) = (lease_expires_at IS NULL)),
    CONSTRAINT yunzhanghu_unsign_reconciliations_resolution_check
        CHECK (
            (
                status = 'pending'
                AND resolved_at IS NULL
                AND resolved_status IS NULL
            )
            OR (
                status = 'resolved'
                AND resolved_at IS NOT NULL
                AND resolved_status IS NOT NULL
                AND lease_owner IS NULL
                AND lease_expires_at IS NULL
            )
        )
);

-- 部署前已经写入的安全 marker 也必须进入调度表，否则新轮询只看 reconciliation
-- 表后它们将永久无法收敛。严格限制格式后再转换时间戳，避免历史脏数据阻断迁移。
INSERT INTO yunzhanghu_unsign_reconciliations (
    nonce,
    release_at,
    next_attempt_at
)
SELECT DISTINCT
    sign_nonce,
    to_timestamp(split_part(sign_nonce, ':', 3)::double precision),
    NOW()
FROM user_yunzhanghu_profiles
WHERE sign_status = 'signing'
  AND sign_nonce ~ '^release:webhook:[0-9]+:'
ON CONFLICT (nonce) DO NOTHING;

CREATE INDEX yunzhanghu_unsign_reconciliations_due_idx
    ON yunzhanghu_unsign_reconciliations (next_attempt_at, created_at)
    WHERE status = 'pending';
