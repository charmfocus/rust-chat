use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::AppError;

use super::Message;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CreateMessage {
    pub content: String,
    pub files: Vec<String>,
}

#[allow(unused)]
impl Message {
    pub fn create(input: CreateMessage, pool: &PgPool) -> Result<Message, AppError> {
        // verify content - not empty
        if input.content.is_empty() {
            return Err(AppError::CreateMessageError(
                "Content cannot be empty".to_string(),
            ));
        }

        // verify files - not empty
        if input.files.is_empty() {
            return Err(AppError::CreateMessageError(
                "Files cannot be empty".to_string(),
            ));
        }
        todo!()
    }
}
