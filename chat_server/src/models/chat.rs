use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::AppError;

use super::{Chat, ChatType};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CreateChat {
    pub name: String,
    pub members: Vec<i64>,
}

#[allow(dead_code)]
impl Chat {
    pub async fn create(
        input: &CreateChat,
        workspace_id: u64,
        pool: &PgPool,
    ) -> Result<Self, AppError> {
        let chat = sqlx::query_as(
            r#"
            INSERT INTO chats (workspace_id, name, type, members)
            VALUES ($1, $2, $3, $4)
            RETURNING id, workspace_id, name, owner_id, created_at
            "#,
        )
        .bind(workspace_id as i64)
        .bind(&input.name)
        .bind(ChatType::Group)
        .bind(&input.members)
        .fetch_one(pool)
        .await?;

        Ok(chat)
    }
}
