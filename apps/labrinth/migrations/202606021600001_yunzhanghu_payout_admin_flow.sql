ALTER TABLE payouts
    ADD COLUMN yunzhanghu_order_id text NULL,
    ADD COLUMN yunzhanghu_submit_started_at timestamptz NULL,
    ADD COLUMN yunzhanghu_submit_finished_at timestamptz NULL,
    ADD COLUMN yunzhanghu_submit_attempts integer NOT NULL DEFAULT 0,
    ADD COLUMN yunzhanghu_submit_error text NULL,
    ADD COLUMN yunzhanghu_confirmed_by bigint REFERENCES users(id) NULL,
    ADD COLUMN admin_rejected_at timestamptz NULL,
    ADD COLUMN admin_rejected_by bigint REFERENCES users(id) NULL,
    ADD COLUMN admin_reject_reason text NULL;

CREATE INDEX payouts_yunzhanghu_waiting_admin_idx
    ON payouts (created)
    WHERE status = 'in-transit'
      AND method = 'yunzhanghu_alipay'
      AND platform_id IS NULL
      AND yunzhanghu_submit_started_at IS NULL;

CREATE INDEX payouts_yunzhanghu_reconcile_idx
    ON payouts (COALESCE(yunzhanghu_submit_started_at, created))
    WHERE status = 'in-transit'
      AND method = 'yunzhanghu_alipay'
      AND (
          platform_id IS NOT NULL
          OR yunzhanghu_submit_started_at IS NOT NULL
      );
