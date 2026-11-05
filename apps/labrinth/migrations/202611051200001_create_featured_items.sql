SET LOCAL lock_timeout = '3s';

CREATE TABLE featured_items (
    id          BIGSERIAL     PRIMARY KEY,
    section     VARCHAR(64)   NOT NULL,
    title       VARCHAR(128)  NOT NULL,
    description VARCHAR(512)  NOT NULL DEFAULT '',
    badge       VARCHAR(32)   NOT NULL DEFAULT '',
    image_url   VARCHAR(2048) NOT NULL,
    link        VARCHAR(2048) NOT NULL,
    project_id  BIGINT,
    sort_order  INTEGER       NOT NULL DEFAULT 0,
    enabled     BOOLEAN       NOT NULL DEFAULT TRUE,
    created_by  BIGINT,
    created_at  timestamptz   NOT NULL DEFAULT NOW(),
    updated_at  timestamptz   NOT NULL DEFAULT NOW()
);

CREATE INDEX featured_items_section_idx ON featured_items (section, sort_order, id);
