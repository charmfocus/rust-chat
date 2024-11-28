mod chat;
mod file;
mod messages;
mod user;
mod workspace;

pub use chat::CreateChat;
pub use messages::{CreateMessage, ListMessages};
use serde::{Deserialize, Serialize};
pub use user::{CreateUser, SigninUser};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChatFile {
    pub workspace_id: u64,
    pub ext: String, // extract ext from filename or mime type
    pub hash: String,
}
