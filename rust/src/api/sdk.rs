use std::str::FromStr;
use std::sync::Mutex;

use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};
use flutter_rust_bridge::{DartFnFuture, frb};
use sia_core::types::Hash256;
use tokio::io::AsyncReadExt;

use super::io::{dart_chunk_reader, run_local};
use super::keys::AppKey;
use super::object::{ObjectEvent, PinnedObject, slab_from_native};
use super::options::{DownloadOptions, UploadOptions};
use super::types::{Account, AddressProtocol, App, Host, NetAddress, ObjectsCursor, PinnedSector, PinnedSlab};

use crate::frb_generated::StreamSink;

/// The live SDK, returned from [Builder::register](super::builder::Builder::register)
/// or [Builder::connected](super::builder::Builder::connected).
#[frb(opaque)]
pub struct Sdk {
    pub(crate) inner: sia_storage::Sdk,
}

#[cfg(target_arch = "wasm32")]
unsafe impl Send for Sdk {}
#[cfg(target_arch = "wasm32")]
unsafe impl Sync for Sdk {}

impl Sdk {
    /// Returns the application key used by the SDK.
    #[frb(sync)]
    pub fn app_key(&self) -> AppKey {
        AppKey {
            inner: self.inner.app_key().clone(),
        }
    }

    /// Uploads an object to the Sia network by streaming bytes from a Dart
    /// pull callback. The callback returns the next chunk; an empty or `null`
    /// result signals EOF.
    ///
    /// Pass a fresh [PinnedObject::new] for new uploads. To resume or append
    /// to a previous upload, pass the object returned from the prior call.
    /// Note that appending changes the object's ID; the object must be
    /// re-pinned afterwards and any cached references updated.
    pub async fn upload(
        &self,
        object: &PinnedObject,
        source: impl Fn() -> DartFnFuture<Option<Vec<u8>>> + Send + Sync + 'static,
        options: &UploadOptions,
    ) -> Result<PinnedObject> {
        run_local(async {
            let opts = options.build();
            let snapshot = object.snapshot();
            let reader = dart_chunk_reader(source);
            let obj = self
                .inner
                .clone()
                .upload(snapshot, reader, opts)
                .await
                .map_err(|e| anyhow!("{e}"))?;
            Ok(PinnedObject {
                inner: Mutex::new(obj),
            })
        })
        .await
    }

    /// Downloads an object from the Sia network as a stream of byte chunks.
    pub async fn download(
        &self,
        object: &PinnedObject,
        sink: StreamSink<Vec<u8>>,
        options: &DownloadOptions,
    ) -> Result<()> {
        run_local(async {
            let opts = options.build();
            let snapshot = object.snapshot();
            let mut reader = self
                .inner
                .download(&snapshot, opts)
                .map_err(|e| anyhow!("{e}"))?;
            let mut buf = vec![0u8; 64 * 1024];
            loop {
                let n = reader.read(&mut buf).await.map_err(|e| anyhow!("{e}"))?;
                if n == 0 {
                    break;
                }
                sink.add(buf[..n].to_vec()).map_err(|e| anyhow!("{e}"))?;
            }
            Ok(())
        })
        .await
    }

    /// Returns a list of all usable hosts.
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

    /// Returns object events from the indexer for syncing.
    pub async fn object_events(
        &self,
        cursor: Option<ObjectsCursor>,
        limit: u32,
    ) -> Result<Vec<ObjectEvent>> {
        run_local(async {
            let cursor = match cursor {
                Some(c) => Some(sia_storage::ObjectsCursor {
                    after: c.after,
                    id: Hash256::from_str(&c.id).map_err(|e| anyhow!("{e}"))?,
                }),
                None => None,
            };
            let events = self
                .inner
                .object_events(cursor, Some(limit as usize))
                .await
                .map_err(|e| anyhow!("{e}"))?;
            Ok(events
                .into_iter()
                .map(|e| ObjectEvent {
                    id: e.id.to_string(),
                    deleted: e.deleted,
                    updated_at: e.updated_at,
                    object: e.object.map(Mutex::new),
                })
                .collect())
        })
        .await
    }

    /// Updates the metadata of an object stored in the indexer.
    pub async fn update_object_metadata(&self, object: &PinnedObject) -> Result<()> {
        run_local(async {
            let snapshot = object.snapshot();
            self.inner
                .update_object_metadata(&snapshot)
                .await
                .map_err(|e| anyhow!("{e}"))
        })
        .await
    }

    /// Deletes an object from the indexer.
    pub async fn delete_object(&self, key: String) -> Result<()> {
        run_local(async {
            let id = Hash256::from_str(&key).map_err(|e| anyhow!("{e}"))?;
            self.inner
                .delete_object(&id)
                .await
                .map_err(|e| anyhow!("{e}"))
        })
        .await
    }

    /// Returns metadata about a specific object.
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

    /// Returns metadata about a slab.
    pub async fn slab(&self, slab_id: String) -> Result<PinnedSlab> {
        run_local(async {
            let id = Hash256::from_str(&slab_id).map_err(|e| anyhow!("{e}"))?;
            let slab = self.inner.slab(&id).await.map_err(|e| anyhow!("{e}"))?;
            Ok(pinned_slab_from_native(slab))
        })
        .await
    }

    /// Unpins slabs not used by any object on the account.
    pub async fn prune_slabs(&self) -> Result<()> {
        run_local(async {
            self.inner.prune_slabs().await.map_err(|e| anyhow!("{e}"))
        })
        .await
    }

    /// Returns the current account.
    pub async fn account(&self) -> Result<Account> {
        run_local(async {
            let a = self.inner.account().await.map_err(|e| anyhow!("{e}"))?;
            Ok(account_from_native(a))
        })
        .await
    }

    /// Pins an object to the indexer.
    pub async fn pin_object(&self, object: &PinnedObject) -> Result<()> {
        run_local(async {
            let snapshot = object.snapshot();
            self.inner
                .pin_object(&snapshot)
                .await
                .map_err(|e| anyhow!("{e}"))
        })
        .await
    }

    /// Creates a signed URL for sharing an object until `valid_until`.
    #[frb(sync)]
    pub fn share_object(
        &self,
        object: &PinnedObject,
        valid_until: DateTime<Utc>,
    ) -> Result<String> {
        let snapshot = object.snapshot();
        let url = self
            .inner
            .share_object(&snapshot, valid_until)
            .map_err(|e| anyhow!("{e}"))?;
        Ok(url.to_string())
    }

    /// Retrieves a shared object from a signed URL.
    pub async fn shared_object(&self, shared_url: String) -> Result<PinnedObject> {
        run_local(async {
            let obj = self
                .inner
                .shared_object(shared_url)
                .await
                .map_err(|e| anyhow!("{e}"))?;
            Ok(PinnedObject {
                inner: Mutex::new(obj),
            })
        })
        .await
    }
}

// ---- internal conversions ----

fn host_from_native(h: sia_storage::Host) -> Host {
    Host {
        public_key: h.public_key.to_string(),
        addresses: h
            .addresses
            .iter()
            .map(|a| NetAddress {
                protocol: match a.protocol {
                    sia_core::types::v2::Protocol::SiaMux => AddressProtocol::SiaMux,
                    sia_core::types::v2::Protocol::QUIC => AddressProtocol::Quic,
                },
                address: a.address.clone(),
            })
            .collect(),
        country_code: h.country_code,
        latitude: h.latitude,
        longitude: h.longitude,
        good_for_upload: h.good_for_upload,
    }
}

fn account_from_native(a: sia_storage::Account) -> Account {
    Account {
        account_key: a.account_key.to_string(),
        max_pinned_data: a.max_pinned_data,
        remaining_storage: a.remaining_storage,
        pinned_data: a.pinned_data,
        pinned_size: a.pinned_size,
        ready: a.ready,
        app: App {
            id: a.app.id.to_string(),
            name: a.app.name,
            description: a.app.description,
            service_url: a.app.service_url,
            logo_url: a.app.logo_url,
        },
        last_used: a.last_used,
    }
}

fn pinned_slab_from_native(s: sia_storage::PinnedSlab) -> PinnedSlab {
    PinnedSlab {
        id: s.id.to_string(),
        encryption_key: s.encryption_key.as_ref().to_vec(),
        min_shards: s.min_shards,
        sectors: s
            .sectors
            .into_iter()
            .map(|sec| PinnedSector {
                root: sec.root.to_string(),
                host_key: sec.host_key.to_string(),
            })
            .collect(),
    }
}

// suppress unused-import warning when slab_from_native isn't used by sdk.rs
const _: fn() = || {
    let _ = slab_from_native;
};
