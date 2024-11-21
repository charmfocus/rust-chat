use std::mem;

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::{AppError, User};

use super::Workspace;

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

impl User {
    // pub fn new(fullname: String, email: String) -> Self {
    //     Self {
    //         id: 0,
    //         fullname,
    //         email,
    //         created_at: chrono::Utc::now(),
    //     }
    // }

    pub async fn find_by_email(email: &str, pool: &PgPool) -> Result<Option<Self>, AppError> {
        let user = sqlx::query_as(
            "SELECT id, workspace_id, fullname, email, created_at FROM users WHERE email = $1",
        )
        .bind(email)
        .fetch_optional(pool)
        .await?;
        Ok(user)
    }

    /// Create a new user
    // TODO: use transaction for workspace creation and user creation
    pub async fn create(input: &CreateUser, pool: &PgPool) -> Result<Self, AppError> {
        // check if workspace exists, if not create one
        let ws = match Workspace::find_by_name(&input.workspace, pool).await? {
            Some(ws) => ws,
            None => Workspace::create(&input.workspace, 0, pool).await?,
        };

        let user = Self::find_by_email(&input.email, pool).await?;

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
        .bind(ws.id as i64)
        .bind(&ws.name)
        .bind(&input.fullname)
        .bind(&input.email)
        .bind(password_hash)
        .fetch_one(pool)
        .await?;

        if ws.owner_id == 0 {
            ws.update_owner(ws.id as u64, user.id as u64, pool).await?;
        }

        Ok(user)
    }

    /// add user to workspace
    /// update workspace_id in users table
    pub async fn add_to_workspace(
        &self,
        workspace_id: u64,
        pool: &PgPool,
    ) -> Result<Self, AppError> {
        let user = sqlx::query_as(
            r#"
            UPDATE users
            SET workspace_id = $1
            WHERE id = $2
            RETURNING id, workspace_id, fullname, email, created_at
            "#,
        )
        .bind(workspace_id as i64)
        .bind(self.id)
        .fetch_one(pool)
        .await?;

        Ok(user)
    }

    /// Verify email and password
    pub async fn verify(input: &SigninUser, pool: &PgPool) -> Result<Option<User>, AppError> {
        let user: Option<User> = sqlx::query_as(
            "SELECT id, workspace_id, fullname, email, password_hash, created_at FROM users WHERE email = $1",
        )
        .bind(&input.email)
        .fetch_optional(pool)
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
    use std::path::Path;

    use super::*;
    use sqlx_db_tester::TestPg;

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
        let tdb = TestPg::new(
            "postgres://postgres:123456@localhost:5432".to_string(),
            Path::new("../migrations"),
        );

        let pool = tdb.get_pool().await;
        let input = CreateUser::new("wiki", "charmfocus@gmail.com", "default", "123456");
        User::create(&input, &pool).await?;
        let ret = User::create(&input, &pool).await;
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
        let tdb = TestPg::new(
            "postgres://postgres:123456@localhost:5432".to_string(),
            Path::new("../migrations"),
        );

        let pool = tdb.get_pool().await;
        let input = CreateUser::new("wiki", "charmfocus@gmail.com", "default", "123456");
        let user = User::create(&input, &pool).await?;
        assert_eq!(user.email, input.email);
        assert_eq!(user.fullname, input.fullname);
        assert!(user.id > 0);

        let user = User::find_by_email(&input.email, &pool).await?;
        assert!(user.is_some());
        let user = user.unwrap();
        assert_eq!(user.email, input.email);
        assert_eq!(user.fullname, input.fullname);

        let input = SigninUser::new(&input.email, &input.password);
        let user = User::verify(&input, &pool).await?;
        assert!(user.is_some());

        Ok(())
    }
}
