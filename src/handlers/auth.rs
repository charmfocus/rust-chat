use axum::response::IntoResponse;

pub async fn signin_handler() -> impl IntoResponse {
    "signin"
}

pub async fn signup_handler() -> impl IntoResponse {
    "signup"
}
