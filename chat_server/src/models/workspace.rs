use anyhow::Result;
use sqlx::PgPool;

use crate::AppError;

use super::{ChatUser, Workspace};

impl Workspace {
    pub async fn create(name: &str, user_id: u64, pool: &PgPool) -> Result<Self, AppError> {
        let ws = sqlx::query_as(
            r#"
            INSERT INTO workspaces (name, owner_id)
            VALUES ($1, $2)
            RETURNING id, name, owner_id, created_at
            "#,
        )
        .bind(name)
        .bind(user_id as i64)
        .fetch_one(pool)
        .await?;
        Ok(ws)
    }

    pub async fn update_owner(
        &self,
        id: u64,
        owner_id: u64,
        pool: &PgPool,
    ) -> Result<Self, AppError> {
        // update owner_id in two cases 1) owner_id = 0 2) owner's ws_id = id
        let ws = sqlx::query_as(
            r#"
            UPDATE workspaces
            SET owner_id = $2
            WHERE id = $1
            RETURNING id, name, owner_id, created_at
            "#,
        )
        .bind(id as i64)
        .bind(owner_id as i64)
        .fetch_one(pool)
        .await?;
        Ok(ws)
    }

    pub async fn find_by_name(name: &str, pool: &PgPool) -> Result<Option<Self>, AppError> {
        let ws = sqlx::query_as(
            r#"
            SELECT id, name, owner_id, created_at
            FROM workspaces
            WHERE name = $1
            "#,
        )
        .bind(name)
        .fetch_optional(pool)
        .await?;
        Ok(ws)
    }

    #[allow(dead_code)]
    pub async fn find_by_id(id: u64, pool: &PgPool) -> Result<Option<Self>, AppError> {
        let ws = sqlx::query_as(
            r#"
            SELECT id, name, owner_id, created_at
            FROM workspaces
            WHERE id = $1
            "#,
        )
        .bind(id as i64)
        .fetch_optional(pool)
        .await?;
        Ok(ws)
    }

    #[allow(dead_code)]
    pub async fn fetch_all_chat_users(
        workspace_id: u64,
        pool: &PgPool,
    ) -> Result<Vec<ChatUser>, AppError> {
        let users = sqlx::query_as(
            r#"
                SELECT id, fullname, email, created_at
                FROM users
                WHERE workspace_id = $1
                ORDER BY id
                "#,
        )
        .bind(workspace_id as i64)
        .fetch_all(pool)
        .await?;
        Ok(users)
    }
}

#[cfg(test)]
mod tests {
    use crate::{models::CreateUser, test_util::get_test_pool, User};

    use super::*;

    use anyhow::Result;

    #[tokio::test]
    async fn workespace_should_create_and_set_owner() -> Result<()> {
        let (_tdb, pool) = get_test_pool(Some("postgres://postgres:123456@localhost")).await;
        let ws = Workspace::create("test", 0, &pool).await?;

        assert_eq!(ws.name, "test");

        let input = CreateUser::new(&ws.name, "wiki2@gmail.com", &ws.name, "123456");
        let user = User::create(&input, &pool).await?;
        assert_eq!(user.workspace_id, ws.id);

        let ws = Workspace::find_by_id(ws.id as u64, &pool).await?;
        assert_eq!(ws.unwrap().owner_id, user.id);

        Ok(())
    }

    #[tokio::test]
    async fn workspace_should_find_by_name() -> Result<()> {
        let (_tdb, pool) = get_test_pool(Some("postgres://postgres:123456@localhost")).await;
        let ws = Workspace::find_by_name("acme", &pool).await?;

        assert_eq!(ws.unwrap().name, "acme");
        Ok(())
    }

    #[tokio::test]
    async fn workspace_should_fetch_all_chat_users() -> Result<()> {
        let (_tdb, pool) = get_test_pool(Some("postgres://postgres:123456@localhost")).await;

        let users = Workspace::fetch_all_chat_users(1, &pool).await?;
        assert_eq!(users.len(), 4);

        Ok(())
    }
}
