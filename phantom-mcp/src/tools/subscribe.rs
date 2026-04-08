use axum::response::sse::{Event, KeepAlive, Sse};
use std::convert::Infallible;
use std::time::Duration;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

/// Returns a SSE stream that yields CCT delta events.
/// Each event has:
///   event: "dom_delta"
///   data:  <CctDelta string per blueprint section 6.4.5>
pub async fn sse_handler(
    axum::extract::State(server): axum::extract::State<crate::McpServer>,
) -> Sse<impl futures_util::stream::Stream<Item = Result<Event, Infallible>>> {
    let rx = server.adapter.delta_tx.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(|msg| {
        match msg {
            Ok(delta) => Some(Ok(Event::default().event("dom_delta").data(delta))),
            Err(tokio_stream::wrappers::errors::BroadcastStreamRecvError::Lagged(n)) => {
                tracing::warn!("SSE subscriber lagged by {} messages", n);
                None // skip lagged events — don't disconnect
            }
        }
    });

    Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(30))
            .text("ping"),
    )
}
