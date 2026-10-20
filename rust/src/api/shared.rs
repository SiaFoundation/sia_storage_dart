use std::str::FromStr;
use std::sync::Mutex;

use anyhow::{Result, anyhow};
use flutter_rust_bridge::frb;
use sia_core::types::Hash256;

use super::io::run_local;
use super::keys::key_stats_from_native;
use super::object::PinnedObject;
use super::options::DownloadOptions;
use super::sdk::{host_from_native, stream_download};
use super::types::{Host, KeyStats, ObjectSummary};

use crate::frb_generated::StreamSink;

/// Read-only access to the objects attached to a sharing key, paid for by
/// the key's owner. Connect with [SharedSdk::connect] using a seed from
/// [SharingKey::save](super::keys::SharingKey::save).
#[frb(opaque)]
pub struct SharedSdk {
    pub(crate) inner: sia_storage::SharedSdk,
}

#[cfg(target_arch = "wasm32")]
unsafe impl Send for SharedSdk {}
#[cfg(target_arch = "wasm32")]
unsafe impl Sync for SharedSdk {}

impl SharedSdk {
    /// Connects to the indexer with a sharing key's 32-byte seed.
    pub async fn connect(indexer_url: String, seed: Vec<u8>) -> Result<SharedSdk> {
        run_local(async {
            let seed: [u8; 32] = seed
                .as_slice()
                .try_into()
                .map_err(|_| anyhow!("sharing keys must be 32 bytes"))?;
            let inner = sia_storage::SharedSdk::connect(indexer_url, seed)
                .await
                .map_err(|e| anyhow!("{e}"))?;
            Ok(SharedSdk { inner })
        })
        .await
    }

    /// Returns the sharing key's usage and lifetime information.
    pub async fn stats(&self) -> Result<KeyStats> {
        run_local(async {
            let stats = self.inner.stats().await.map_err(|e| anyhow!("{e}"))?;
            Ok(key_stats_from_native(stats))
        })
        .await
    }

    /// Returns a shared object by ID.
    pub async fn object(&self, key: String) -> Result<PinnedObject> {
        run_local(async {
            let id = Hash256::from_str(&key).map_err(|e| anyhow!("{e}"))?;
            let obj = self.inner.object(&id).await.map_err(|e| anyhow!("{e}"))?;
            Ok(PinnedObject {
                inner: Mutex::new(obj),
            })
        })
        .await
    }

    /// Lists the shared objects. Omit `offset` or `limit` to use the
    /// indexer's default paging.
    pub async fn objects(
        &self,
        offset: Option<u64>,
        limit: Option<u64>,
    ) -> Result<Vec<PinnedObject>> {
        run_local(async {
            let objects = self
                .inner
                .objects(offset, limit)
                .await
                .map_err(|e| anyhow!("{e}"))?;
            Ok(objects
                .into_iter()
                .map(|o| PinnedObject {
                    inner: Mutex::new(o),
                })
                .collect())
        })
        .await
    }

    /// Lists the shared objects without their slabs. Omit `offset` or `limit`
    /// to use the indexer's default paging.
    pub async fn object_summaries(
        &self,
        offset: Option<u64>,
        limit: Option<u64>,
    ) -> Result<Vec<ObjectSummary>> {
        run_local(async {
            let summaries = self
                .inner
                .object_summaries(offset, limit)
                .await
                .map_err(|e| anyhow!("{e}"))?;
            Ok(summaries
                .into_iter()
                .map(|s| ObjectSummary {
                    id: s.id.to_string(),
                    size: s.size,
                    metadata: s.metadata,
                    created_at: s.created_at,
                    updated_at: s.updated_at,
                })
                .collect())
        })
        .await
    }

    /// Returns the hosts storing the shared objects.
    pub async fn hosts(&self) -> Result<Vec<Host>> {
        run_local(async {
            let hosts = self
                .inner
                .hosts(Default::default())
                .await
                .map_err(|e| anyhow!("{e}"))?;
            Ok(hosts.into_iter().map(host_from_native).collect())
        })
        .await
    }
}

/// Downloads a shared object from the Sia network as a stream of byte chunks.
///
/// A free function rather than a method so the generated Dart class doesn't
/// gain a member that would shadow the `SharedSdkStreams.download` extension method in
/// `lib/sia_storage.dart`.
pub async fn shared_download(
    sdk: &SharedSdk,
    object: &PinnedObject,
    sink: StreamSink<Vec<u8>>,
    options: &DownloadOptions,
) -> Result<()> {
    run_local(async {
        let opts = options.build();
        let snapshot = object.snapshot();
        let reader = sdk
            .inner
            .download(&snapshot, opts)
            .map_err(|e| anyhow!("{e}"))?;
        stream_download(reader, sink).await
    })
    .await
}
