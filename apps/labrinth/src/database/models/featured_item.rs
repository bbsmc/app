use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct DBFeaturedItem {
    pub id: i64,
    pub section: String,
    pub title: String,
    pub description: String,
    pub badge: String,
    pub image_url: String,
    pub link: String,
    pub project_id: Option<i64>,
    pub sort_order: i32,
    pub enabled: bool,
    pub created_by: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct NewFeaturedItem {
    pub section: String,
    pub title: String,
    pub description: String,
    pub badge: String,
    pub image_url: String,
    pub link: String,
    pub project_id: Option<i64>,
    pub enabled: bool,
    pub created_by: Option<i64>,
}

impl DBFeaturedItem {
    pub async fn list_enabled<'a, E>(exec: E) -> Result<Vec<Self>, sqlx::Error>
    where
        E: sqlx::Executor<'a, Database = sqlx::Postgres>,
    {
        sqlx::query_as!(
            DBFeaturedItem,
            "SELECT id, section, title, description, badge, image_url, link, project_id,
                    sort_order, enabled, created_by, created_at, updated_at
             FROM featured_items
             WHERE enabled = TRUE
             ORDER BY section, sort_order, id"
        )
        .fetch_all(exec)
        .await
    }

    pub async fn list_section<'a, E>(
        section: &str,
        exec: E,
    ) -> Result<Vec<Self>, sqlx::Error>
    where
        E: sqlx::Executor<'a, Database = sqlx::Postgres>,
    {
        sqlx::query_as!(
            DBFeaturedItem,
            "SELECT id, section, title, description, badge, image_url, link, project_id,
                    sort_order, enabled, created_by, created_at, updated_at
             FROM featured_items
             WHERE section = $1
             ORDER BY sort_order, id",
            section
        )
        .fetch_all(exec)
        .await
    }

    pub async fn get<'a, E>(
        id: i64,
        exec: E,
    ) -> Result<Option<Self>, sqlx::Error>
    where
        E: sqlx::Executor<'a, Database = sqlx::Postgres>,
    {
        sqlx::query_as!(
            DBFeaturedItem,
            "SELECT id, section, title, description, badge, image_url, link, project_id,
                    sort_order, enabled, created_by, created_at, updated_at
             FROM featured_items
             WHERE id = $1",
            id
        )
        .fetch_optional(exec)
        .await
    }

    pub async fn insert<'a, E>(
        item: NewFeaturedItem,
        exec: E,
    ) -> Result<Self, sqlx::Error>
    where
        E: sqlx::Executor<'a, Database = sqlx::Postgres>,
    {
        sqlx::query_as!(
            DBFeaturedItem,
            "INSERT INTO featured_items
                (section, title, description, badge, image_url, link, project_id, enabled, created_by, sort_order)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9,
                (SELECT COALESCE(MAX(sort_order) + 1, 0) FROM featured_items WHERE section = $10))
             RETURNING id, section, title, description, badge, image_url, link, project_id,
                       sort_order, enabled, created_by, created_at, updated_at",
            item.section,
            item.title,
            item.description,
            item.badge,
            item.image_url,
            item.link,
            item.project_id,
            item.enabled,
            item.created_by,
            item.section
        )
        .fetch_one(exec)
        .await
    }

    pub async fn update<'a, E>(
        &self,
        exec: E,
    ) -> Result<Option<Self>, sqlx::Error>
    where
        E: sqlx::Executor<'a, Database = sqlx::Postgres>,
    {
        sqlx::query_as!(
            DBFeaturedItem,
            "UPDATE featured_items
             SET title = $2, description = $3, badge = $4, image_url = $5, link = $6,
                 project_id = $7, enabled = $8, updated_at = NOW()
             WHERE id = $1
             RETURNING id, section, title, description, badge, image_url, link, project_id,
                       sort_order, enabled, created_by, created_at, updated_at",
            self.id,
            self.title,
            self.description,
            self.badge,
            self.image_url,
            self.link,
            self.project_id,
            self.enabled
        )
        .fetch_optional(exec)
        .await
    }

    pub async fn delete<'a, E>(id: i64, exec: E) -> Result<bool, sqlx::Error>
    where
        E: sqlx::Executor<'a, Database = sqlx::Postgres>,
    {
        let result =
            sqlx::query!("DELETE FROM featured_items WHERE id = $1", id)
                .execute(exec)
                .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn section_ids<'a, E>(
        section: &str,
        exec: E,
    ) -> Result<Vec<i64>, sqlx::Error>
    where
        E: sqlx::Executor<'a, Database = sqlx::Postgres>,
    {
        sqlx::query_scalar!(
            "SELECT id FROM featured_items WHERE section = $1 ORDER BY sort_order, id",
            section
        )
        .fetch_all(exec)
        .await
    }

    pub async fn set_sort_order<'a, E>(
        id: i64,
        section: &str,
        sort_order: i32,
        exec: E,
    ) -> Result<(), sqlx::Error>
    where
        E: sqlx::Executor<'a, Database = sqlx::Postgres>,
    {
        sqlx::query!(
            "UPDATE featured_items SET sort_order = $3, updated_at = NOW()
             WHERE id = $1 AND section = $2",
            id,
            section,
            sort_order
        )
        .execute(exec)
        .await?;
        Ok(())
    }
}
