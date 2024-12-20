use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};

use crate::{
    models::{CreateUser, SigninUser},
    AppError, AppState, ErrorOutput,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthOutput {
    pub token: String,
}

pub(crate) async fn signup_handler(
    State(state): State<AppState>,
    Json(input): Json<CreateUser>,
) -> Result<impl IntoResponse, AppError> {
    let user = state.create_user(&input).await?;

    let token = state.ek.sign(user)?;
    // let mut header = HeaderMap::new();
    // header.insert("X-Auth-Token", token.parse()?);
    // Ok((StatusCode::CREATED, header))
    let body = Json(AuthOutput { token });
    Ok((StatusCode::CREATED, body))
}

pub(crate) async fn signin_handler(
    State(state): State<AppState>,
    Json(input): Json<SigninUser>,
) -> Result<impl IntoResponse, AppError> {
    let user = state.verify_user(&input).await?;
    match user {
        Some(user) => {
            let token = state.ek.sign(user)?;
            Ok((StatusCode::OK, Json(AuthOutput { token })).into_response())
        }
        None => {
            let body = Json(ErrorOutput::new("Invalid email or password"));
            Ok((StatusCode::FORBIDDEN, body).into_response())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use axum::body::to_bytes;

    #[tokio::test]
    async fn signup_should_work() -> Result<()> {
        let (_tdb, state) = AppState::new_for_test().await?;
        let input = CreateUser::new("wiki4", "wiki4@gmail.com", "default", "123456");
        let ret = signup_handler(State(state), Json(input))
            .await?
            .into_response();

        assert_eq!(ret.status(), StatusCode::CREATED);

        let body = to_bytes(ret.into_body(), usize::MAX).await?;
        let ret = serde_json::from_slice::<AuthOutput>(&body)?;
        assert!(!ret.token.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn signup_duplicate_user_should_409() -> Result<()> {
        let (_tdb, state) = AppState::new_for_test().await?;
        let fullname = "wiki";
        let email = "charmfocus@gmail.com";
        let workspace = "acme";
        let password = "123456";

        let input = CreateUser::new(fullname, email, workspace, password);

        let ret = signup_handler(State(state.clone()), Json(input.clone()))
            .await
            .into_response();

        assert_eq!(ret.status(), StatusCode::CONFLICT);
        let body = to_bytes(ret.into_body(), usize::MAX).await?;
        let ret = serde_json::from_slice::<ErrorOutput>(&body)?;
        assert_eq!(ret.error, format!("email already exists: {}", email));

        Ok(())
    }

    #[tokio::test]
    async fn signin_should_work() -> Result<()> {
        let email = "charmfocus@gmail.com";
        let password = "123456";

        let (_tdb, state) = AppState::new_for_test().await?;

        let input = SigninUser::new(email, password);
        let ret = signin_handler(State(state), Json(input))
            .await?
            .into_response();

        assert_eq!(ret.status(), StatusCode::OK);

        let body = to_bytes(ret.into_body(), usize::MAX).await?;
        let ret = serde_json::from_slice::<AuthOutput>(&body)?;
        assert!(!ret.token.is_empty());

        Ok(())
    }

    #[tokio::test]
    async fn signin_with_non_exist_user_should_403() -> Result<()> {
        let (_tdb, state) = AppState::new_for_test().await?;

        let input = SigninUser::new("non_exist_user@gmail.com", "123456");
        let ret = signin_handler(State(state), Json(input))
            .await
            .into_response();

        assert_eq!(ret.status(), StatusCode::FORBIDDEN);
        let body = to_bytes(ret.into_body(), usize::MAX).await?;
        let ret = serde_json::from_slice::<ErrorOutput>(&body)?;
        assert_eq!(ret.error, "Invalid email or password");

        Ok(())
    }
}
