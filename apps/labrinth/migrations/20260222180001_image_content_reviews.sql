CREATE TABLE image_content_reviews (
    id bigint PRIMARY KEY,
    image_url text NOT NULL,
    raw_image_url text,
    uploader_id bigint NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    source_type varchar(32) NOT NULL,
    source_id bigint,
    project_id bigint REFERENCES mods(id),
    risk_labels text NOT NULL,
    status varchar(32) NOT NULL DEFAULT 'pending',
    created_at timestamptz NOT NULL DEFAULT CURRENT_TIMESTAMP,
    reviewed_by bigint REFERENCES users(id),
    reviewed_at timestamptz,
    review_notes text
);

CREATE INDEX image_content_reviews_status_idx ON image_content_reviews (status, created_at DESC);
CREATE INDEX image_content_reviews_uploader_idx ON image_content_reviews (uploader_id, status);
