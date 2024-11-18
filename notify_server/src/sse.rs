use std::{convert::Infallible, time::Duration};

use axum::response::{sse::Event, Sse};
use axum_extra::{headers, TypedHeader};
use futures::{stream, Stream};
use tokio_stream::StreamExt;

pub(crate) async fn sse_handler(
    TypedHeader(user_agent): TypedHeader<headers::UserAgent>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    println!("`{}` connected", user_agent.as_str());

    // A `Stream` that repeats an event every second.
    //
    // You can also
    // https://docs.rs/tokio-stream
    let stream = stream::repeat_with(|| Event::default().data("hi!"))
        .map(Ok)
        .throttle(Duration::from_secs(1));

    Sse::new(stream)
}
