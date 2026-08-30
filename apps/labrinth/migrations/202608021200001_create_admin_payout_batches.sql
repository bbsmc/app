CREATE TABLE admin_payout_batches (
    id varchar(32) PRIMARY KEY,
    -- 审计快照故意不设外键：管理员账号删除后仍保留原始操作者 ID/用户名。
    requested_by bigint NOT NULL,
    requested_by_username varchar(39) NOT NULL,
    status varchar(16) NOT NULL DEFAULT 'previewed',
    created_at timestamptz NOT NULL DEFAULT NOW(),
    expires_at timestamptz NOT NULL,
    processing_started_at timestamptz,
    processing_owner varchar(32),
    heartbeat_at timestamptz,
    finished_at timestamptz,
    CONSTRAINT admin_payout_batches_status_check CHECK (
        status IN ('previewed', 'processing', 'completed', 'partial')
    ),
    CONSTRAINT admin_payout_batches_expiry_check CHECK (
        expires_at > created_at
    ),
    CONSTRAINT admin_payout_batches_state_check CHECK (
        (
            status = 'previewed'
            AND processing_started_at IS NULL
            AND processing_owner IS NULL
            AND heartbeat_at IS NULL
            AND finished_at IS NULL
        ) OR (
            status = 'processing'
            AND processing_started_at IS NOT NULL
            AND processing_owner IS NOT NULL
            AND heartbeat_at IS NOT NULL
            AND finished_at IS NULL
        ) OR (
            status IN ('completed', 'partial')
            AND processing_started_at IS NOT NULL
            AND processing_owner IS NULL
            AND heartbeat_at IS NULL
            AND finished_at IS NOT NULL
        )
    )
);

CREATE INDEX admin_payout_batches_expired_previews
    ON admin_payout_batches (expires_at)
    WHERE status = 'previewed';

CREATE TABLE admin_payout_batch_items (
    batch_id varchar(32) NOT NULL
        REFERENCES admin_payout_batches(id) ON DELETE CASCADE,
    -- 用户删除后仍保留本次预览和执行的审计快照。
    user_id bigint NOT NULL,
    username varchar(39) NOT NULL,
    amount numeric(40, 2) NOT NULL,
    profile_updated_at timestamptz NOT NULL,
    sources jsonb NOT NULL,
    status varchar(16) NOT NULL DEFAULT 'pending',
    error_code varchar(64),
    PRIMARY KEY (batch_id, user_id),
    CONSTRAINT admin_payout_batch_items_amount_check CHECK (amount >= 5),
    CONSTRAINT admin_payout_batch_items_sources_check CHECK (
        jsonb_typeof(sources) = 'array' AND jsonb_array_length(sources) > 0
    ),
    CONSTRAINT admin_payout_batch_items_status_check CHECK (
        status IN ('pending', 'created', 'skipped', 'failed')
    ),
    CONSTRAINT admin_payout_batch_items_error_check CHECK (
        (status IN ('pending', 'created') AND error_code IS NULL)
        OR (status IN ('skipped', 'failed') AND error_code IS NOT NULL)
    )
);

CREATE INDEX admin_payout_batch_items_user_id
    ON admin_payout_batch_items (user_id);

-- 一个用户可按云账户单笔限额拆成多笔。金额计划在 preview 时固化，apply
-- 只允许把 planned 原子推进到 created。payout_id 不设外键，以免订单或用户
-- 删除后丢失本次批处理的不可变审计编号。
CREATE TABLE admin_payout_batch_orders (
    batch_id varchar(32) NOT NULL,
    user_id bigint NOT NULL,
    chunk_index integer NOT NULL,
    amount numeric(40, 2) NOT NULL,
    status varchar(16) NOT NULL DEFAULT 'planned',
    payout_id bigint UNIQUE,
    PRIMARY KEY (batch_id, user_id, chunk_index),
    CONSTRAINT admin_payout_batch_orders_item_fk
        FOREIGN KEY (batch_id, user_id)
        REFERENCES admin_payout_batch_items(batch_id, user_id)
        ON DELETE CASCADE,
    CONSTRAINT admin_payout_batch_orders_chunk_index_check CHECK (
        chunk_index >= 0
    ),
    CONSTRAINT admin_payout_batch_orders_amount_check CHECK (
        amount >= 5 AND amount <= 50000
    ),
    CONSTRAINT admin_payout_batch_orders_state_check CHECK (
        (status = 'planned' AND payout_id IS NULL)
        OR (status = 'created' AND payout_id IS NOT NULL)
    )
);

CREATE TABLE admin_payout_batch_exclusions (
    batch_id varchar(32) NOT NULL
        REFERENCES admin_payout_batches(id) ON DELETE CASCADE,
    -- 与 item 相同，保留用户删除前的安全审计快照。
    user_id bigint NOT NULL,
    username varchar(39) NOT NULL,
    amount numeric(40, 2) NOT NULL,
    reason_code varchar(64) NOT NULL,
    PRIMARY KEY (batch_id, user_id),
    CONSTRAINT admin_payout_batch_exclusions_amount_check CHECK (amount >= 0),
    CONSTRAINT admin_payout_batch_exclusions_reason_check CHECK (
        reason_code IN (
            'below_minimum',
            'banned',
            'missing_profile',
            'not_signed',
            'sign_operation_pending',
            'incomplete_profile',
            'profile_unreadable',
            'source_attribution_failed'
        )
    )
);

-- 批量提现执行事件采用独立、只追加的审计流。这里不保存用户、管理员或 payout
-- 外键，避免相关业务行删除后破坏审计记录；batch 外键使用默认 RESTRICT，确保
-- 一旦开始执行便不能通过删除 batch 级联抹除事件。
CREATE TABLE admin_payout_batch_events (
    id bigserial PRIMARY KEY,
    batch_id varchar(32) NOT NULL REFERENCES admin_payout_batches(id),
    event_type varchar(32) NOT NULL,
    actor_user_id bigint NOT NULL,
    actor_username varchar(39) NOT NULL,
    processing_owner varchar(32) NOT NULL,
    user_id bigint,
    details jsonb NOT NULL DEFAULT '{}'::jsonb,
    created_at timestamptz NOT NULL DEFAULT NOW(),
    CONSTRAINT admin_payout_batch_events_type_check CHECK (
        event_type IN (
            'apply_requested',
            'processing_started',
            'processing_taken_over',
            'item_created',
            'item_skipped',
            'item_failed',
            'batch_completed',
            'batch_partial'
        )
    ),
    CONSTRAINT admin_payout_batch_events_details_check CHECK (
        jsonb_typeof(details) = 'object'
    )
);

CREATE INDEX admin_payout_batch_events_batch_id
    ON admin_payout_batch_events (batch_id, id);

CREATE INDEX admin_payout_batch_events_user_id
    ON admin_payout_batch_events (user_id, created_at)
    WHERE user_id IS NOT NULL;

CREATE FUNCTION reject_admin_payout_batch_event_mutation()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    RAISE EXCEPTION 'admin_payout_batch_events is append-only';
END;
$$;

CREATE TRIGGER admin_payout_batch_events_no_update_or_delete
    BEFORE UPDATE OR DELETE ON admin_payout_batch_events
    FOR EACH ROW
    EXECUTE FUNCTION reject_admin_payout_batch_event_mutation();

CREATE TRIGGER admin_payout_batch_events_no_truncate
    BEFORE TRUNCATE ON admin_payout_batch_events
    FOR EACH STATEMENT
    EXECUTE FUNCTION reject_admin_payout_batch_event_mutation();
