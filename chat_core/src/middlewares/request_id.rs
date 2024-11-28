use axum::{extract::Request, http::HeaderValue, middleware::Next, response::Response};

use super::REQUEST_ID_HEADER;

pub async fn set_request_id(mut req: Request, next: Next) -> Response {
    let id = match req.headers().get(REQUEST_ID_HEADER) {
        Some(v) => v.clone(),
        None => {
            let request_id = uuid::Uuid::now_v7().to_string();
            let header_value =
                HeaderValue::from_str(&request_id).expect("Invalid request ID header value");
            req.headers_mut()
                .insert(REQUEST_ID_HEADER, header_value.clone());
            header_value
        }
    };

    let mut res = next.run(req).await;

    res.headers_mut().insert(REQUEST_ID_HEADER, id);
    res
}
