CREATE INDEX IF NOT EXISTS mods_organization_status_downloads_id
    ON mods (organization_id, status, downloads DESC, id)
    WHERE organization_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS mods_team_status_downloads_id
    ON mods (team_id, status, downloads DESC, id);

CREATE INDEX IF NOT EXISTS versions_mod_status_id
    ON versions (mod_id, status, id);

CREATE INDEX IF NOT EXISTS organizations_lower_slug
    ON organizations (LOWER(slug));

CREATE INDEX IF NOT EXISTS users_lower_username
    ON users (LOWER(username));

CREATE INDEX IF NOT EXISTS collections_user_status_updated_id
    ON collections (user_id, status, updated DESC, id);

CREATE INDEX IF NOT EXISTS team_members_user_accepted_team
    ON team_members (user_id, accepted, team_id);

CREATE INDEX IF NOT EXISTS discussions_user_deleted_created_id
    ON discussions (user_id, deleted, created_at DESC, id);

CREATE INDEX IF NOT EXISTS posts_user_deleted_created_id
    ON posts (user_id, deleted, created_at DESC, id);

CREATE INDEX IF NOT EXISTS posts_discussion_deleted_created_id
    ON posts (discussion_id, deleted, created_at DESC, id);

CREATE INDEX IF NOT EXISTS mods_forum_id
    ON mods (forum, id)
    WHERE forum IS NOT NULL;
