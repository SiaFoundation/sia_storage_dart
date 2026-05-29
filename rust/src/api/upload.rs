use std::sync::Mutex;

use anyhow::{Result, anyhow};
use flutter_rust_bridge::{DartFnFuture, frb};

use super::io::{dart_chunk_reader, run_local};
use super::object::PinnedObject;

/// A packed upload, returned from
/// [Sdk::upload_packed](super::sdk::Sdk::upload_packed).
///
/// Packs multiple small objects into shared slabs to avoid the per-object
/// padding of [Sdk::upload](super::sdk::Sdk::upload). Add each object with
/// [PackedUpload::add], then call [PackedUpload::finalize] to flush the packed
/// slabs and obtain the resulting objects. The objects must be pinned to the
/// indexer afterwards.
#[frb(opaque)]
pub struct PackedUpload {
    pub(crate) inner: Mutex<Option<sia_storage::PackedUpload>>,
}

#[cfg(target_arch = "wasm32")]
unsafe impl Send for PackedUpload {}
#[cfg(target_arch = "wasm32")]
unsafe impl Sync for PackedUpload {}

impl PackedUpload {
    /// Finalizes the upload, flushing all packed slabs to hosts, and returns
    /// the resulting objects in the order they were added. The objects must be
    /// pinned to the indexer afterwards. The handle cannot be used again.
    pub async fn finalize(&self) -> Result<Vec<PinnedObject>> {
        run_local(async {
            let upload = self
                .inner
                .lock()
                .expect("packed upload mutex poisoned")
                .take()
                .ok_or_else(|| anyhow!("packed upload already finalized"))?;
            let objects = upload.finalize().await.map_err(|e| anyhow!("{e}"))?;
            Ok(objects
                .into_iter()
                .map(|o| PinnedObject {
                    inner: Mutex::new(o),
                })
                .collect())
        })
        .await
    }

    /// Returns the number of bytes remaining until the current slab reaches its
    /// optimal packed size. Adding an object larger than this starts a new
    /// slab; prioritize objects that fit to minimize padding.
    #[frb(sync)]
    pub fn remaining(&self) -> Result<u64> {
        self.with(|u| u.remaining())
    }

    /// Returns the cumulative length in bytes of all objects added so far.
    #[frb(sync)]
    pub fn length(&self) -> Result<u64> {
        self.with(|u| u.length())
    }

    /// Returns the optimal size in bytes of each packed slab.
    #[frb(sync)]
    pub fn optimal_data_size(&self) -> Result<u64> {
        self.with(|u| u.optimal_data_size() as u64)
    }

    /// Returns the number of slabs the upload will produce once finalized.
    #[frb(sync)]
    pub fn slabs(&self) -> Result<u64> {
        self.with(|u| u.slabs() as u64)
    }

    fn with<T>(&self, f: impl FnOnce(&sia_storage::PackedUpload) -> T) -> Result<T> {
        let guard = self.inner.lock().expect("packed upload mutex poisoned");
        let upload = guard
            .as_ref()
            .ok_or_else(|| anyhow!("packed upload already finalized"))?;
        Ok(f(upload))
    }
}

/// Adds an object to the upload by streaming bytes from a Dart pull callback.
/// The callback returns the next chunk; an empty or `null` result signals EOF.
/// Returns the number of bytes consumed.
///
/// If the reader errors part-way, no object is registered for the failed call
/// and the upload may still be continued or finalized. Objects are returned in
/// add order from [PackedUpload::finalize].
pub async fn packed_upload_add(
    upload: &PackedUpload,
    source: impl Fn() -> DartFnFuture<Option<Vec<u8>>> + Send + Sync + 'static,
) -> Result<u64> {
    run_local(async {
        let mut packed = upload
            .inner
            .lock()
            .expect("packed upload mutex poisoned")
            .take()
            .ok_or_else(|| anyhow!("packed upload already finalized"))?;
        let reader = dart_chunk_reader(source);
        let res = packed.add(reader).await;
        *upload.inner.lock().expect("packed upload mutex poisoned") = Some(packed);
        res.map_err(|e| anyhow!("{e}"))
    })
    .await
}
