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
