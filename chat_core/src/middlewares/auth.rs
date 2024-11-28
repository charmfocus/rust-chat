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

use super::TokenVerify;

pub async fn verify_token<T>(State(state): State<T>, req: Request, next: Next) -> Response
where
    T: TokenVerify + Clone + Send + Sync + 'static,
{
    let (mut parts, body) = req.into_parts();

    let token = TypedHeader::<Authorization<Bearer>>::from_request_parts(&mut parts, &state).await;

    let req = match token {
        Ok(TypedHeader(Authorization(bearer))) => {
            let token = bearer.token();
            match state.verify(token) {
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
    use std::sync::Arc;

    use crate::{DecodingKey, EncodingKey, User};

    use super::*;

    #[derive(Clone)]
    struct AppState(Arc<AppStateInner>);

    struct AppStateInner {
        ek: EncodingKey,
        dk: DecodingKey,
    }

    impl TokenVerify for AppState {
        type Error = ();

        fn verify(&self, token: &str) -> Result<User, Self::Error> {
            self.0.dk.verify(token).map_err(|_| ())
        }
    }

    use anyhow::Result;
    use axum::{body::Body, middleware::from_fn_with_state, routing::get, Router};
    use tower::ServiceExt;

    async fn handler(_req: Request) -> impl IntoResponse {
        (StatusCode::OK, "ok")
    }

    #[tokio::test]
    async fn verify_token_middleware_should_work() -> Result<()> {
        let state = AppState(Arc::new(AppStateInner {
            ek: EncodingKey::load(include_str!("../../fixtures/encoding.pem"))?,
            dk: DecodingKey::load(include_str!("../../fixtures/decoding.pem"))?,
        }));

        let user = User::new(1, 0, "wiki", "charmfocus@gmail.com");
        let token = state.0.ek.sign(user)?;

        let app = Router::new()
            .route("/", get(handler))
            .layer(from_fn_with_state(state.clone(), verify_token::<AppState>))
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
