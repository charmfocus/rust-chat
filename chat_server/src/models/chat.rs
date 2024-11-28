use serde::{Deserialize, Serialize};

use crate::{AppError, AppState};

use chat_core::{Chat, ChatType};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CreateChat {
    pub name: Option<String>,
    pub members: Vec<i64>,
    pub public: bool,
}

#[allow(dead_code)]
impl AppState {
    pub async fn create_chat(
        &self,
        input: CreateChat,
        workspace_id: u64,
    ) -> Result<Chat, AppError> {
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
        let users = self.fetch_chat_user_by_ids(&input.members).await?;
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
        .fetch_one(&self.pool)
        .await?;

        Ok(chat)
    }

    pub async fn fetch_chats(&self, workspace_id: u64) -> Result<Vec<Chat>, AppError> {
        let chats = sqlx::query_as(
            r#"
                SELECT id, workspace_id, name, type, members, created_at
                FROM chats
                WHERE workspace_id = $1
                ORDER BY created_at DESC
            "#,
        )
        .bind(workspace_id as i64)
        .fetch_all(&self.pool)
        .await?;

        Ok(chats)
    }

    pub async fn get_chat_by_id(&self, id: u64) -> Result<Option<Chat>, AppError> {
        let chat = sqlx::query_as(
            r#"
                SELECT id, workspace_id, name, type, members, created_at
                FROM chats
                WHERE id = $1
            "#,
        )
        .bind(id as i64)
        .fetch_optional(&self.pool)
        .await?;

        Ok(chat)
    }

    pub async fn is_chat_member(&self, chat_id: u64, user_id: u64) -> Result<bool, AppError> {
        let is_member = sqlx::query(
            r#"
                    SELECT 1
                    FROM chats
                    WHERE id = $1 AND $2 = ANY(members)
            "#,
        )
        .bind(chat_id as i64)
        .bind(user_id as i64)
        .fetch_optional(&self.pool)
        .await?;

        Ok(is_member.is_some())
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
    use anyhow::Result;

    use super::*;

    #[tokio::test]
    async fn create_single_chat_should_work() -> Result<()> {
        let (_tdb, state) = AppState::new_for_test().await?;
        let input = CreateChat::new("", &[1, 2], false);
        let chat = state
            .create_chat(input, 1)
            .await
            .expect("create chat failed");

        assert_eq!(chat.workspace_id, 1);
        assert_eq!(chat.members, vec![1, 2]);
        assert_eq!(chat.r#type, ChatType::Single);
        Ok(())
    }

    #[tokio::test]
    async fn create_public_named_chat_should_work() -> Result<()> {
        let (_tdb, state) = AppState::new_for_test().await?;
        let input = CreateChat::new("general", &[1, 2, 3], true);
        let chat = state
            .create_chat(input, 1)
            .await
            .expect("create chat failed");

        assert_eq!(chat.workspace_id, 1);
        assert_eq!(chat.members, vec![1, 2, 3]);
        assert_eq!(chat.r#type, ChatType::PublicChannel);
        assert_eq!(chat.name, Some("general".to_string()));

        Ok(())
    }

    #[tokio::test]
    async fn chat_get_by_id_should_work() -> Result<()> {
        let (_tdb, state) = AppState::new_for_test().await?;
        let chat = state
            .get_chat_by_id(1)
            .await
            .unwrap()
            .expect("get chat by id failed");

        assert_eq!(chat.workspace_id, 1);
        assert_eq!(chat.members, vec![1, 2, 3, 4, 5]);
        assert_eq!(chat.r#type, ChatType::PublicChannel);
        assert_eq!(chat.name, Some("general".to_string()));

        Ok(())
    }

    #[tokio::test]
    async fn chat_fetch_all_should_work() -> Result<()> {
        let (_tdb, state) = AppState::new_for_test().await?;
        let chats = state.fetch_chats(1).await.expect("fetch all chats failed");

        assert_eq!(chats.len(), 4);

        Ok(())
    }

    #[tokio::test]
    async fn chat_is_member_should_work() -> Result<()> {
        let (_tdb, state) = AppState::new_for_test().await?;
        let is_member = state
            .is_chat_member(1, 1)
            .await
            .expect("is chat member failed");

        assert!(is_member);

        // user 6 doesn't exist
        let is_member = state
            .is_chat_member(1, 6)
            .await
            .expect("is chat member failed");

        assert!(!is_member);

        // chat 10 doesn't exist
        let is_member = state
            .is_chat_member(10, 1)
            .await
            .expect("is chat member failed");

        assert!(!is_member);

        // user 4 is not a member of chat 2
        let is_member = state
            .is_chat_member(2, 4)
            .await
            .expect("is chat member failed");

        assert!(!is_member);

        Ok(())
    }
}
