mod config;
mod error;
mod handlers;
mod middlewares;
mod models;

use anyhow::Context;
use chat_core::{
    middlewares::{set_layer, verify_token, TokenVerify},
    DecodingKey, EncodingKey,
};
use handlers::*;
use middlewares::verify_chat;
use sqlx::PgPool;
use std::{fmt, ops::Deref, sync::Arc};
use tokio::fs;

use axum::{
    middleware::from_fn_with_state,
    routing::{get, post},
    Router,
};
pub use config::AppConfig;
pub use error::{AppError, ErrorOutput};
pub use models::*;

#[derive(Debug, Clone)]
pub struct AppState {
    inner: Arc<AppStateInner>,
}

#[allow(unused)]
pub struct AppStateInner {
    pub config: AppConfig,
    pub dk: DecodingKey,
    pub ek: EncodingKey,
    pub pool: PgPool,
}

pub async fn get_router(state: AppState) -> Result<Router, AppError> {
    // let state = AppState::try_new(config).await?;

    let chat = Router::new()
        .route(
            "/:id",
            get(get_chat_handler)
                .patch(update_chat_handler)
                .delete(delete_chat_handler)
                .post(send_message_handler),
        )
        .route("/:id/messages", get(list_message_handler))
        .layer(from_fn_with_state(state.clone(), verify_chat))
        .route("/", get(list_chat_handler).post(create_chat_handler));

    let api = Router::new()
        .route("/users", get(list_chat_users_handler))
        .nest("/chats", chat)
        .route("/upload", post(upload_handler))
        .route("/files/:workspace_id/*path", get(file_handler))
        .layer(from_fn_with_state(state.clone(), verify_token::<AppState>))
        // routes doesn't need token verification
        .route("/signin", post(signin_handler))
        .route("/signup", post(signup_handler));
    let app = Router::new()
        .route("/", get(index_handler))
        .nest("/api", api)
        .with_state(state);
    Ok(set_layer(app))
}

// 当我调用 state.config = state.inner.config
impl Deref for AppState {
    type Target = AppStateInner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl TokenVerify for AppState {
    type Error = AppError;
    fn verify(&self, token: &str) -> Result<chat_core::User, Self::Error> {
        let user = self.dk.verify(token)?;
        Ok(user)
    }
}

impl AppState {
    pub async fn try_new(config: AppConfig) -> Result<Self, AppError> {
        fs::create_dir_all(&config.server.base_dir)
            .await
            .context("create base_dir failed")?;
        let ek = EncodingKey::load(&config.auth.ek).context("load ek key")?;
        let dk = DecodingKey::load(&config.auth.pk).context("load dk key")?;
        let pool = PgPool::connect(&config.server.db_url)
            .await
            .context("connect db")?;
        Ok(Self {
            inner: Arc::new(AppStateInner {
                config,
                ek,
                dk,
                pool,
            }),
        })
    }
}

impl fmt::Debug for AppStateInner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AppStateInner")
            .field("config", &self.config)
            .finish()
    }
}

#[cfg(feature = "test-util")]
mod test_util {
    use std::{path::Path, sync::Arc};

    use sqlx_db_tester::TestPg;
    use url::Url;

    use super::*;

    use crate::{AppConfig, AppError, AppState, AppStateInner};

    impl AppState {
        pub async fn new_for_test() -> Result<(sqlx_db_tester::TestPg, Self), AppError> {
            let config = AppConfig::load()?;
            let dk = DecodingKey::load(&config.auth.pk).context("load dk key")?;
            let ek = EncodingKey::load(&config.auth.ek).context("load ek key")?;
            let db_url = Url::parse(&config.server.db_url).context("parse db url")?;
            let server_base_rul = format!(
                "{}://{}:{}@{}",
                db_url.scheme(),
                db_url.username(),
                db_url.password().unwrap_or_default(),
                db_url.host_str().unwrap_or_default()
            );
            // let server_url = "postgres://postgres:123456@127.0.0.1:5432";
            let (tdb, pool) = get_test_pool(Some(&server_base_rul)).await;
            let state = Self {
                inner: Arc::new(AppStateInner {
                    config,
                    ek,
                    dk,
                    pool,
                }),
            };

            Ok((tdb, state))
        }
    }

    pub async fn get_test_pool(url: Option<&str>) -> (TestPg, PgPool) {
        let url = url.unwrap_or("postgres://postgres:123456@127.0.0.1:5432");

        let tdb = TestPg::new(url.to_string(), Path::new("../migrations"));
        let pool = tdb.get_pool().await;

        (tdb, pool)
    }
}
