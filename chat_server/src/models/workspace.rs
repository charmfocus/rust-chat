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
    pub async fn fetch_all_chat_users(id: u64, pool: &PgPool) -> Result<Vec<ChatUser>, AppError> {
        let users = sqlx::query_as(
            r#"
                SELECT id, fullname, email, created_at
                FROM users
                WHERE workspace_id = $1
                ORDER BY id
                "#,
        )
        .bind(id as i64)
        .fetch_all(pool)
        .await?;
        Ok(users)
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::{models::CreateUser, User};

    use super::*;

    use anyhow::Result;
    use sqlx_db_tester::TestPg;

    #[tokio::test]
    async fn workespace_should_create_and_set_owner() -> Result<()> {
        let tdb = TestPg::new(
            "postgres://postgres:123456@localhost".to_string(),
            Path::new("../migrations"),
        );

        let pool = tdb.get_pool().await;
        let input = CreateUser::new("wiki", "charmfocus@gmail.com", "default", "123456");
        let user = User::create(&input, &pool).await?;
        let ws = Workspace::create("test", user.id as u64, &pool).await?;
        assert_eq!(ws.name, "test");

        let user = user.add_to_workspace(ws.id as u64, &pool).await?;
        assert_eq!(user.workspace_id, ws.id);
        assert_eq!(ws.owner_id, user.id);

        Ok(())
    }

    #[tokio::test]
    async fn workspace_should_find_by_name() -> Result<()> {
        let tdb = TestPg::new(
            "postgres://postgres:123456@localhost".to_string(),
            Path::new("../migrations"),
        );

        let pool = tdb.get_pool().await;

        Workspace::create("test", 0, &pool).await?;
        let ws = Workspace::find_by_name("test", &pool).await?;

        assert_eq!(ws.unwrap().name, "test");
        Ok(())
    }

    #[tokio::test]
    async fn workspace_should_fetch_all_chat_users() -> Result<()> {
        let tdb = TestPg::new(
            "postgres://postgres:123456@localhost".to_string(),
            Path::new("../migrations"),
        );

        let pool = tdb.get_pool().await;
        let ws = Workspace::create("test", 0, &pool).await?;
        assert_eq!(ws.name, "test");
        let input = CreateUser::new("wiki", "charmfocus@gmail.com", &ws.name, "123456");
        let user1 = User::create(&input, &pool).await?;
        assert_eq!(user1.fullname, "wiki");

        let input = CreateUser::new("wukun", "wukun@gmail.com", &ws.name, "123456");
        let user2 = User::create(&input, &pool).await?;
        assert_eq!(user2.fullname, "wukun");

        let users = Workspace::fetch_all_chat_users(ws.id as u64, &pool).await?;
        assert_eq!(users.len(), 2);
        assert_eq!(users[0].id, user1.id);

        Ok(())
    }
}
