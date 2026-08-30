-- 项目激励开通申请表（作者发起，版主审核）
CREATE TABLE incentive_applications (
    id                bigserial PRIMARY KEY,
    project_id        bigint NOT NULL REFERENCES mods(id) ON DELETE CASCADE,
    applicant_user_id bigint NOT NULL REFERENCES users(id),
    reason            text,
    status            text NOT NULL DEFAULT 'pending',
    review_notes      text,
    reviewed_by       bigint REFERENCES users(id),
    reviewed_at       timestamptz,
    created_at        timestamptz NOT NULL DEFAULT NOW()
);

-- 一个项目同一时刻只能有一条 pending 申请
CREATE UNIQUE INDEX idx_incentive_appl_pending_per_project
    ON incentive_applications (project_id)
    WHERE status = 'pending';

CREATE INDEX idx_incentive_appl_status_created
    ON incentive_applications (status, created_at DESC);
