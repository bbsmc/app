-- 1. 事件结算时按发生时的 split 快照（防 split 偷换）
ALTER TABLE incentive_download_events
    ADD COLUMN split_snapshot jsonb;
COMMENT ON COLUMN incentive_download_events.split_snapshot
    IS '事件发生时项目团队的 split 快照: [{"user_id":..., "split":...}, ...]';

-- 2. 审计日志（追踪所有激励相关的状态变更）
CREATE TABLE incentive_audit_log (
    id              bigserial PRIMARY KEY,
    actor_user_id   bigint REFERENCES users(id),
    action          text NOT NULL,
    target_type     text NOT NULL,
    target_id       bigint NOT NULL,
    metadata        jsonb,
    created_at      timestamptz NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_incentive_audit_action_time
    ON incentive_audit_log (action, created_at DESC);
CREATE INDEX idx_incentive_audit_target
    ON incentive_audit_log (target_type, target_id);
CREATE INDEX idx_incentive_audit_actor
    ON incentive_audit_log (actor_user_id, created_at DESC);
