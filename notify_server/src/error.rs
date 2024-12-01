use axum::{
    body::Body,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorOutput {
    pub error: String,
}

impl ErrorOutput {
    pub fn new(error: impl Into<String>) -> Self {
        Self {
            error: error.into(),
        }
    }
}

#[derive(Error, Debug)]
pub enum AppError {
    #[error("io error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("jwt error: {0}")]
    JwtError(#[from] jwt_simple::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response<Body> {
        let status = match &self {
            AppError::JwtError(_) => StatusCode::FORBIDDEN,
            AppError::IoError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };

        (status, Json(ErrorOutput::new(self.to_string()))).into_response()
    }
}
