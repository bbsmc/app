ALTER TABLE mods ADD COLUMN IF NOT EXISTS default_type varchar(100) NOT NULL DEFAULT 'project';
ALTER TABLE mods ADD COLUMN IF NOT EXISTS default_game_version varchar(2048) NOT NULL DEFAULT '';
ALTER TABLE mods ADD COLUMN IF NOT EXISTS default_game_loaders varchar(2048) NOT NULL DEFAULT '';