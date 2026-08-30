CREATE TABLE payout_yunzhanghu_order_details (
    payout_id                       bigint PRIMARY KEY REFERENCES payouts(id) ON DELETE CASCADE,
    order_id                        text NOT NULL,
    platform_id                     text,
    pay                             numeric(40, 20) NOT NULL DEFAULT 0,
    user_real_amount                numeric(40, 20),
    user_real_excluding_vat_amount  numeric(40, 20),
    user_fee                        numeric(40, 20) NOT NULL DEFAULT 0,
    received_user_fee               numeric(40, 20) NOT NULL DEFAULT 0,
    tax                             numeric(40, 20) NOT NULL DEFAULT 0,
    received_tax_amount             numeric(40, 20) NOT NULL DEFAULT 0,
    personal_tax                    numeric(40, 20) NOT NULL DEFAULT 0,
    value_added_tax                 numeric(40, 20) NOT NULL DEFAULT 0,
    additional_tax                  numeric(40, 20) NOT NULL DEFAULT 0,
    user_personal_tax               numeric(40, 20) NOT NULL DEFAULT 0,
    user_value_added_tax            numeric(40, 20) NOT NULL DEFAULT 0,
    user_additional_tax             numeric(40, 20) NOT NULL DEFAULT 0,
    received_personal_tax           numeric(40, 20) NOT NULL DEFAULT 0,
    user_received_personal_tax      numeric(40, 20) NOT NULL DEFAULT 0,
    received_value_added_tax        numeric(40, 20) NOT NULL DEFAULT 0,
    user_received_value_added_tax   numeric(40, 20) NOT NULL DEFAULT 0,
    received_additional_tax         numeric(40, 20) NOT NULL DEFAULT 0,
    user_received_additional_tax    numeric(40, 20) NOT NULL DEFAULT 0,
    raw_status                      text,
    raw_status_detail               text,
    queried_at                      timestamptz NOT NULL DEFAULT NOW()
);

CREATE INDEX payout_yunzhanghu_order_details_order_id_idx
    ON payout_yunzhanghu_order_details (order_id);
