//! Streaming bridge between Dart and Rust + a LocalSet wrapper for wasm.
//!
//! - Uploads pull data from a Dart `() -> Future<Uint8List?>` callback (null
//!   marks EOF) by polling the future across multiple `poll_next` calls and
//!   adapting it into [tokio::io::AsyncRead] via [tokio_util::io::StreamReader].
//!   No background task is spawned, so this is portable across native and
//!   wasm targets where `tokio::spawn` is not generally available.
//! - [run_local] wraps a future in `LocalSet::run_until` on wasm32. The Sia
//!   Storage SDK uses `tokio::task::spawn_local` on wasm, which requires a
//!   `LocalSet` to be active in the calling task. Every async binding method
//!   that touches the SDK delegates through this helper.

use std::io;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use bytes::Bytes;
use flutter_rust_bridge::DartFnFuture;
use futures_core::{Future, Stream};
use tokio_util::io::StreamReader;

type DartChunkFn = dyn Fn() -> DartFnFuture<Option<Vec<u8>>> + Send + Sync + 'static;

/// A `Stream<Item = io::Result<Bytes>>` driven by a Dart callback.
pub(crate) struct DartChunkStream {
    pull: Arc<DartChunkFn>,
    in_flight: Option<Pin<Box<dyn Future<Output = Option<Vec<u8>>> + Send>>>,
}

impl DartChunkStream {
    pub fn new<F>(pull: F) -> Self
    where
        F: Fn() -> DartFnFuture<Option<Vec<u8>>> + Send + Sync + 'static,
    {
        Self {
            pull: Arc::new(pull),
            in_flight: None,
        }
    }
}

impl Stream for DartChunkStream {
    type Item = io::Result<Bytes>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            if self.in_flight.is_none() {
                let fut = (self.pull)();
                self.in_flight = Some(fut);
            }
            let fut = self.in_flight.as_mut().unwrap();
            match fut.as_mut().poll(cx) {
                Poll::Ready(opt) => {
                    self.in_flight = None;
                    match opt {
                        None => return Poll::Ready(None),
                        Some(chunk) if chunk.is_empty() => return Poll::Ready(None),
                        Some(chunk) => return Poll::Ready(Some(Ok(Bytes::from(chunk)))),
                    }
                }
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

/// Adapts a Dart pull callback into [tokio::io::AsyncRead].
pub(crate) fn dart_chunk_reader<F>(pull: F) -> StreamReader<DartChunkStream, Bytes>
where
    F: Fn() -> DartFnFuture<Option<Vec<u8>>> + Send + Sync + 'static,
{
    StreamReader::new(DartChunkStream::new(pull))
}

/// Runs `fut` inside a fresh `tokio::task::LocalSet` on wasm32; on native
/// targets it just awaits the future. The SDK's wasm path uses
/// `tokio::task::spawn_local` for fire-and-forget tasks (e.g. host warm-up),
/// which panics without an enclosing LocalSet.
///
/// Long-lived background tasks the SDK spawns inside this scope are dropped
/// when the wrapped future completes — acceptable for one-shot operations
/// (upload / download / fetch) but means an idle SDK won't continuously
/// refresh its host list on wasm.
#[cfg(target_arch = "wasm32")]
pub(crate) async fn run_local<F: futures_core::Future>(fut: F) -> F::Output {
    tokio::task::LocalSet::new().run_until(fut).await
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) async fn run_local<F: futures_core::Future>(fut: F) -> F::Output {
    fut.await
}
