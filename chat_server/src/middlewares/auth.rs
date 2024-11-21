use axum::{
    extract::{FromRequestParts, Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use axum_extra::{
    headers::{authorization::Bearer, Authorization},
    TypedHeader,
};
use tracing::warn;

use crate::AppState;

pub async fn verify_token(State(state): State<AppState>, req: Request, next: Next) -> Response {
    let (mut parts, body) = req.into_parts();

    let token = TypedHeader::<Authorization<Bearer>>::from_request_parts(&mut parts, &state).await;

    let req = match token {
        Ok(TypedHeader(Authorization(bearer))) => {
            let token = bearer.token();
            match state.dk.verify(token) {
                Ok(user) => {
                    let mut req = Request::from_parts(parts, body);
                    req.extensions_mut().insert(user);
                    req
                }
                Err(err) => {
                    let msg = format!("verify token error: {:?}", err);
                    warn!(msg);
                    return (StatusCode::FORBIDDEN, msg).into_response();
                }
            }
        }
        Err(err) => {
            let msg = format!("parse Authorization header error: {:?}", err);
            warn!(msg);
            return (StatusCode::UNAUTHORIZED, msg).into_response();
        }
    };

    next.run(req).await
}

#[cfg(test)]
mod tests {
    use crate::{AppConfig, User};

    use super::*;

    use anyhow::Result;
    use axum::{body::Body, middleware::from_fn_with_state, routing::get, Router};
    use tower::ServiceExt;

    async fn handler(_req: Request) -> impl IntoResponse {
        (StatusCode::OK, "ok").into_response()
    }

    #[tokio::test]
    async fn verify_token_middleware_should_work() -> Result<()> {
        let config = AppConfig::load()?;
        let (_tdb, state) = AppState::new_for_test(config).await?;

        let user = User::new(1, "wiki", "charmfocus@gmail.com");
        let token = state.ek.sign(user)?;

        let app = Router::new()
            .route("/", get(handler))
            .layer(from_fn_with_state(state.clone(), verify_token))
            .with_state(state);

        // good token
        let req = Request::builder()
            .uri("/")
            .header("Authorization", format!("Bearer {}", token))
            .body(Body::empty())?;
        let res = app.clone().oneshot(req).await?;
        assert_eq!(res.status(), StatusCode::OK);

        // no token
        let req = Request::builder().uri("/").body(Body::empty())?;
        let res = app.clone().oneshot(req).await?;
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

        // bad token
        let req = Request::builder()
            .uri("/")
            .header("Authorization", format!("Bearer {}", "bad token"))
            .body(Body::empty())?;
        let res = app.clone().oneshot(req).await?;
        assert_eq!(res.status(), StatusCode::FORBIDDEN);

        Ok(())
    }
}
