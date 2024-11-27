use std::mem;

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use serde::{Deserialize, Serialize};

use crate::{AppError, AppState, User};

use super::ChatUser;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CreateUser {
    pub fullname: String,
    pub email: String,
    pub workspace: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SigninUser {
    pub email: String,
    pub password: String,
}

impl AppState {
    // pub fn new(fullname: String, email: String) -> Self {
    //     Self {
    //         id: 0,
    //         fullname,
    //         email,
    //         created_at: chrono::Utc::now(),
    //     }
    // }

    pub async fn find_user_by_email(&self, email: &str) -> Result<Option<User>, AppError> {
        let user = sqlx::query_as(
            "SELECT id, workspace_id, fullname, email, created_at FROM users WHERE email = $1",
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await?;
        Ok(user)
    }

    /// Create a new user
    // TODO: use transaction for workspace creation and user creation
    pub async fn create_user(&self, input: &CreateUser) -> Result<User, AppError> {
        // check if workspace exists, if not create one
        let ws = match self.find_workspace_by_name(&input.workspace).await? {
            Some(ws) => ws,
            None => self.create_workspace(&input.workspace, 0).await?,
        };

        let user = self.find_user_by_email(&input.email).await?;

        if user.is_some() {
            return Err(AppError::EmailAlreadyExists(input.email.clone()));
        }

        let password_hash = hash_password(&input.password)?;

        let user: User = sqlx::query_as(
            r#"
            INSERT INTO users (workspace_id, workspace, fullname, email, password_hash)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, workspace_id, fullname, email, created_at
            "#,
        )
        .bind(ws.id)
        .bind(&ws.name)
        .bind(&input.fullname)
        .bind(&input.email)
        .bind(password_hash)
        .fetch_one(&self.pool)
        .await?;

        if ws.owner_id == 0 {
            ws.update_owner(ws.id as u64, user.id as u64, &self.pool)
                .await?;
        }

        Ok(user)
    }

    /// Verify email and password
    pub async fn verify_user(&self, input: &SigninUser) -> Result<Option<User>, AppError> {
        let user: Option<User> = sqlx::query_as(
            "SELECT id, workspace_id, fullname, email, password_hash, created_at FROM users WHERE email = $1",
        )
        .bind(&input.email)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(mut user) = user {
            let password_hash = mem::take(&mut user.password_hash);

            let is_valid = verify_password(&input.password, &password_hash.unwrap_or_default())?;
            if is_valid {
                return Ok(Some(user));
            }
        }
        Ok(None)
    }

    pub async fn fetch_chat_user_by_ids(&self, ids: &[i64]) -> Result<Vec<ChatUser>, AppError> {
        let users = sqlx::query_as(
            r#"
            SELECT id, fullname, email, created_at
            FROM users
            WHERE id = ANY($1)
            "#,
        )
        .bind(ids)
        .fetch_all(&self.pool)
        .await?;

        Ok(users)
    }

    pub async fn fetch_chat_users(&self, workspace_id: i64) -> Result<Vec<ChatUser>, AppError> {
        let users = sqlx::query_as(
            r#"
            SELECT id, fullname, email, created_at
            FROM users
            WHERE workspace_id = $1
            "#,
        )
        .bind(workspace_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(users)
    }
}

// use argon2 gen password
fn hash_password(password: &str) -> Result<String, AppError> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();

    let hash = argon2
        .hash_password(password.as_bytes(), &salt)?
        .to_string();
    Ok(hash)
}

fn verify_password(password: &str, hash: &str) -> Result<bool, AppError> {
    let argon2 = Argon2::default();
    let parsed_hash = argon2::PasswordHash::new(hash)?;
    let is_valid = argon2
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok();
    Ok(is_valid)
}

#[cfg(test)]
impl CreateUser {
    pub fn new(fullname: &str, email: &str, workspace: &str, password: &str) -> Self {
        Self {
            fullname: fullname.to_string(),
            email: email.to_string(),
            workspace: workspace.to_string(),
            password: password.to_string(),
        }
    }
}

#[cfg(test)]
impl SigninUser {
    pub fn new(email: &str, password: &str) -> Self {
        Self {
            email: email.to_string(),
            password: password.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use super::*;

    #[test]
    fn hash_password_and_verify_should_work() -> Result<()> {
        let password = "123456";
        let hash = hash_password(password)?;
        assert_eq!(hash.len(), 97);
        assert!(verify_password(password, &hash)?);
        Ok(())
    }

    #[tokio::test]
    async fn create_duplicate_user_should_fail() -> Result<()> {
        let (_tdb, state) = AppState::new_for_test().await?;
        let input = CreateUser::new("wiki", "charmfocus@gmail.com", "default", "123456");
        let ret = state.create_user(&input).await;
        assert!(ret.is_err());
        match ret {
            Err(AppError::EmailAlreadyExists(email)) => {
                assert_eq!(email, input.email);
            }
            _ => panic!("unexpected error"),
        }

        Ok(())
    }

    #[tokio::test]
    async fn create_and_verify_user_should_work() -> Result<()> {
        let (_tdb, state) = AppState::new_for_test().await?;
        let input = CreateUser::new("wiki3", "wiki3@gmail.com", "default", "123456");
        let user = state.create_user(&input).await?;
        assert_eq!(user.email, input.email);
        assert_eq!(user.fullname, input.fullname);
        assert!(user.id > 0);

        let user = state.find_user_by_email(&input.email).await?;
        assert!(user.is_some());
        let user = user.unwrap();
        assert_eq!(user.email, input.email);
        assert_eq!(user.fullname, input.fullname);

        let input = SigninUser::new(&input.email, &input.password);
        let user = state.verify_user(&input).await?;
        assert!(user.is_some());

        Ok(())
    }
}
