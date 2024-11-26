use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::AppError;

use super::{Chat, ChatType, ChatUser};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CreateChat {
    pub name: Option<String>,
    pub members: Vec<i64>,
    pub public: bool,
}

#[allow(dead_code)]
impl Chat {
    pub async fn create(
        input: CreateChat,
        workspace_id: u64,
        pool: &PgPool,
    ) -> Result<Self, AppError> {
        let len = input.members.len();
        if len < 2 {
            return Err(AppError::CreateChatError(
                "Chat must have at least 2 members".to_string(),
            ));
        }

        if len > 8 && input.name.is_none() {
            return Err(AppError::CreateChatError(
                "Group chat with more than 8 members must have a name".to_string(),
            ));
        }

        // verify if all members exists
        let users = ChatUser::fetch_by_ids(&input.members, pool).await?;
        if users.len() != len {
            return Err(AppError::CreateChatError(
                "Some members do not exist".to_string(),
            ));
        }

        let chat_type = match (&input.name, len) {
            (None, 2) => ChatType::Single,
            (None, _) => ChatType::Group,
            (Some(_), _) => {
                if input.public {
                    ChatType::PublicChannel
                } else {
                    ChatType::PrivateChannel
                }
            }
        };

        let chat = sqlx::query_as(
            r#"
                    INSERT INTO chats (workspace_id, name, type, members)
                    VALUES ($1, $2, $3, $4)
                    RETURNING id, workspace_id, name, type, members, created_at
                    "#,
        )
        .bind(workspace_id as i64)
        .bind(&input.name)
        .bind(chat_type)
        .bind(&input.members)
        .fetch_one(pool)
        .await?;

        Ok(chat)
    }

    pub async fn fetch_all(workspace_id: u64, pool: &PgPool) -> Result<Vec<Self>, AppError> {
        let chats = sqlx::query_as(
            r#"
                SELECT id, workspace_id, name, type, members, created_at
                FROM chats
                WHERE workspace_id = $1
                ORDER BY created_at DESC
            "#,
        )
        .bind(workspace_id as i64)
        .fetch_all(pool)
        .await?;

        Ok(chats)
    }

    pub async fn get_by_id(id: u64, pool: &PgPool) -> Result<Option<Self>, AppError> {
        let chat = sqlx::query_as(
            r#"
                SELECT id, workspace_id, name, type, members, created_at
                FROM chats
                WHERE id = $1
            "#,
        )
        .bind(id as i64)
        .fetch_optional(pool)
        .await?;

        Ok(chat)
    }
}

#[cfg(test)]
impl CreateChat {
    pub fn new(name: &str, members: &[i64], public: bool) -> Self {
        let name = if name.is_empty() {
            None
        } else {
            Some(name.to_string())
        };

        Self {
            name,
            members: members.to_vec(),
            public,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::test_util::get_test_pool;

    use super::*;

    #[tokio::test]
    async fn create_single_chat_should_work() {
        let (_tdb, pool) = get_test_pool(None).await;
        let input = CreateChat::new("", &[1, 2], false);
        let chat = Chat::create(input, 1, &pool)
            .await
            .expect("create chat failed");

        assert_eq!(chat.workspace_id, 1);
        assert_eq!(chat.members, vec![1, 2]);
        assert_eq!(chat.r#type, ChatType::Single);
    }

    #[tokio::test]
    async fn create_public_named_chat_should_work() {
        let (_tdb, pool) = get_test_pool(None).await;
        let input = CreateChat::new("general", &[1, 2, 3], true);
        let chat = Chat::create(input, 1, &pool)
            .await
            .expect("create chat failed");

        assert_eq!(chat.workspace_id, 1);
        assert_eq!(chat.members, vec![1, 2, 3]);
        assert_eq!(chat.r#type, ChatType::PublicChannel);
        assert_eq!(chat.name, Some("general".to_string()));
    }

    #[tokio::test]
    async fn chat_get_by_id_should_work() {
        let (_tdb, pool) = get_test_pool(None).await;
        let chat = Chat::get_by_id(1, &pool)
            .await
            .unwrap()
            .expect("get chat by id failed");

        assert_eq!(chat.workspace_id, 1);
        assert_eq!(chat.members, vec![1, 2, 3, 4, 5]);
        assert_eq!(chat.r#type, ChatType::PublicChannel);
        assert_eq!(chat.name, Some("general".to_string()));
    }

    #[tokio::test]
    async fn chat_fetch_all_should_work() {
        let (_tdb, pool) = get_test_pool(None).await;
        let chats = Chat::fetch_all(1, &pool)
            .await
            .expect("fetch all chats failed");

        assert_eq!(chats.len(), 4);
    }
}