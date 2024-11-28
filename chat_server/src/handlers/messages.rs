use axum::{
    extract::{Multipart, Path, Query, State},
    http::HeaderMap,
    response::IntoResponse,
    Extension, Json,
};
use tokio::fs;
use tracing::warn;

use crate::{AppError, AppState, ChatFile, CreateMessage, ListMessages};
use chat_core::User;

pub(crate) async fn send_message_handler(
    Extension(user): Extension<User>,
    State(state): State<AppState>,
    Path(id): Path<u64>,
    Json(input): Json<CreateMessage>,
) -> Result<impl IntoResponse, AppError> {
    let msg = state.create_message(input, id, user.id as _).await?;

    Ok(Json(msg))
}

pub(crate) async fn list_message_handler(
    State(state): State<AppState>,
    Path(id): Path<u64>,
    Query(input): Query<ListMessages>,
) -> Result<impl IntoResponse, AppError> {
    let messages = state.list_message(input, id).await?;
    Ok(Json(messages))
}

pub(crate) async fn file_handler(
    Extension(user): Extension<User>,
    State(state): State<AppState>,
    Path((workspace_id, path)): Path<(i64, String)>,
) -> Result<impl IntoResponse, AppError> {
    if user.workspace_id != workspace_id {
        return Err(AppError::NotFound(
            "file doesn't exist or you don't have permission to access it.".to_string(),
        ));
    }

    let base_dir = state.config.server.base_dir.join(workspace_id.to_string());
    let path = base_dir.join(path);
    if !path.exists() {
        return Err(AppError::NotFound("file doesn't exist".to_string()));
    }

    let mime = mime_guess::from_path(&path).first_or_octet_stream();
    let body = fs::read(path).await?;

    let mut headers = HeaderMap::new();
    headers.insert("content-type", mime.to_string().parse().unwrap());

    Ok((headers, body))
}

pub(crate) async fn upload_handler(
    Extension(user): Extension<User>,
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, AppError> {
    let workspace_id = user.workspace_id as u64;
    let base_dir = &state.config.server.base_dir;
    let mut files = vec![];

    while let Some(field) = multipart.next_field().await.unwrap() {
        let name = field.name().map(|s| s.to_string());
        let filename = field.file_name().map(|s| s.to_string());
        let data = field.bytes().await;

        let (Some(filename), Ok(data)) = (&filename, data) else {
            warn!(
                "Failed to read multipart field - name: {:?}, file name: {:?}",
                name.unwrap_or_default(),
                filename
            );
            continue;
        };

        let file = ChatFile::new(workspace_id, filename, &data);
        let path = file.path(base_dir);
        if path.exists() {
            warn!("File {} already exists: {:?}", &filename, &path);
            continue;
        } else {
            fs::create_dir_all(path.parent().expect("file path parent should exists")).await?;
            fs::write(path, data).await?;
        }

        files.push(file.url());
    }

    Ok(Json(files))
}
