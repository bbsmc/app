ALTER TABLE wiki_cache ADD COLUMN again_count bigint NOT NULL DEFAULT 0;
ALTER TABLE wiki_cache ADD COLUMN again_time timestamp with time zone NOT NULL DEFAULT CURRENT_TIMESTAMP;

ALTER TABLE users ADD COLUMN wiki_overtake_count bigint NOT NULL DEFAULT 0;
ALTER TABLE users ADD COLUMN wiki_ban_time timestamp with time zone NOT NULL DEFAULT CURRENT_TIMESTAMP;