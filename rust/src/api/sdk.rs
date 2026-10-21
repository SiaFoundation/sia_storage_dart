use std::str::FromStr;
use std::sync::Mutex;

use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};
use flutter_rust_bridge::{DartFnFuture, frb};
use sia_core::types::Hash256;
use tokio::io::AsyncReadExt;

use super::io::{dart_chunk_reader, run_local};
use super::keys::{AppKey, SharingKey, SharingKeyRecord};
use super::object::{ObjectEvent, PinnedObject};
use super::options::{DownloadOptions, UploadOptions};
use super::types::{
    Account, AddressProtocol, App, Host, NetAddress, ObjectEventSummary, ObjectSummary,
    ObjectsCursor,
};
use super::upload::PackedUpload;

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

    /// Returns object events from the indexer without the objects' slabs,
    /// which makes the response a small fraction of the size of
    /// [Sdk::object_events]. Fetch the full object with [Sdk::object] to
    /// download it.
    pub async fn object_event_summaries(
        &self,
        cursor: Option<ObjectsCursor>,
        limit: u32,
    ) -> Result<Vec<ObjectEventSummary>> {
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
                .object_event_summaries(cursor, Some(limit as usize))
                .await
                .map_err(|e| anyhow!("{e}"))?;
            Ok(events
                .into_iter()
                .map(|e| ObjectEventSummary {
                    id: e.id.to_string(),
                    deleted: e.deleted,
                    updated_at: e.updated_at,
                    object: e.object.map(|o| ObjectSummary {
                        id: o.id.to_string(),
                        size: o.size,
                        metadata: o.metadata,
                        created_at: o.created_at,
                        updated_at: o.updated_at,
                    }),
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

    /// Unpins slabs not used by any object on the account. Only slabs pinned
    /// before `before` are pruned; `None` leaves the cutoff to the indexer,
    /// which spares recently pinned slabs so an in-progress upload is not
    /// pruned out from under itself.
    pub async fn prune_slabs(&self, before: Option<DateTime<Utc>>) -> Result<()> {
        run_local(async {
            self.inner
                .prune_slabs(before)
                .await
                .map_err(|e| anyhow!("{e}"))
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
    pub fn object_share_url(
        &self,
        object: &PinnedObject,
        valid_until: DateTime<Utc>,
    ) -> Result<String> {
        let snapshot = object.snapshot();
        let url = self
            .inner
            .object_share_url(&snapshot, valid_until)
            .map_err(|e| anyhow!("{e}"))?;
        Ok(url.to_string())
    }

    /// Retrieves a shared object from a signed URL.
    pub async fn object_from_share_url(&self, share_url: String) -> Result<PinnedObject> {
        run_local(async {
            let obj = self
                .inner
                .object_from_share_url(share_url)
                .await
                .map_err(|e| anyhow!("{e}"))?;
            Ok(PinnedObject {
                inner: Mutex::new(obj),
            })
        })
        .await
    }

    /// Creates a sharing key granting read-only access to the objects
    /// attached to it with [Sdk::share_object]. `expires_at` of `None`
    /// creates a key that never expires.
    pub async fn create_sharing_key(
        &self,
        description: String,
        expires_at: Option<DateTime<Utc>>,
    ) -> Result<SharingKey> {
        run_local(async {
            let key = self
                .inner
                .create_sharing_key(sia_storage::SharingKeyOptions {
                    description,
                    expires_at,
                })
                .await
                .map_err(|e| anyhow!("{e}"))?;
            Ok(SharingKey { inner: key })
        })
        .await
    }

    /// Lists the account's sharing keys. Omit `offset` or `limit` to use the
    /// indexer's default paging.
    pub async fn sharing_keys(
        &self,
        offset: Option<u64>,
        limit: Option<u64>,
    ) -> Result<Vec<SharingKeyRecord>> {
        run_local(async {
            let records = self
                .inner
                .sharing_keys(offset, limit)
                .await
                .map_err(|e| anyhow!("{e}"))?;
            Ok(records
                .into_iter()
                .map(|r| SharingKeyRecord { inner: r })
                .collect())
        })
        .await
    }

    /// Fetches the indexer's current record for `key`.
    pub async fn sharing_key(&self, key: &SharingKey) -> Result<SharingKeyRecord> {
        run_local(async {
            let record = self
                .inner
                .sharing_key(&key.inner)
                .await
                .map_err(|e| anyhow!("{e}"))?;
            Ok(SharingKeyRecord { inner: record })
        })
        .await
    }

    /// Attaches an object the account owns to `key` so recipients of the key
    /// can read it. Attaching an already attached object replaces it, so a
    /// failed call can be retried.
    pub async fn share_object(&self, key: &SharingKey, object: &PinnedObject) -> Result<()> {
        run_local(async {
            let snapshot = object.snapshot();
            self.inner
                .share_object(&key.inner, &snapshot)
                .await
                .map_err(|e| anyhow!("{e}"))
        })
        .await
    }

    /// Lists the objects attached to `key`. Omit `offset` or `limit` to use
    /// the indexer's default paging.
    pub async fn shared_objects(
        &self,
        key: &SharingKey,
        offset: Option<u64>,
        limit: Option<u64>,
    ) -> Result<Vec<PinnedObject>> {
        run_local(async {
            let objects = self
                .inner
                .shared_objects(&key.inner, offset, limit)
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

    /// Detaches the object with ID `object_id` from `key`.
    pub async fn unshare_object(&self, key: &SharingKey, object_id: String) -> Result<()> {
        run_local(async {
            let id = Hash256::from_str(&object_id).map_err(|e| anyhow!("{e}"))?;
            self.inner
                .unshare_object(&key.inner, &id)
                .await
                .map_err(|e| anyhow!("{e}"))
        })
        .await
    }

    /// Deletes `key` along with all of its object attachments. Downloads
    /// already in flight may keep reading for up to five minutes.
    pub async fn revoke_sharing_key(&self, key: &SharingKey) -> Result<()> {
        run_local(async {
            self.inner
                .revoke_sharing_key(&key.inner)
                .await
                .map_err(|e| anyhow!("{e}"))
        })
        .await
    }
}

/// Uploads an object to the Sia network by streaming bytes from a Dart pull
/// callback. The callback returns the next chunk; an empty or `null` result
/// signals EOF.
///
/// Pass a fresh [PinnedObject::new] for new uploads. To resume or append to a
/// previous upload, pass the object returned from the prior call. Note that
/// appending changes the object's ID; the object must be re-pinned afterwards
/// and any cached references updated.
///
/// A free function rather than a method so the generated Dart class doesn't
/// gain a member that would shadow the `SdkStreams.upload` extension method in
/// `lib/sia_storage.dart`.
pub async fn upload(
    sdk: &Sdk,
    object: &PinnedObject,
    source: impl Fn() -> DartFnFuture<Option<Vec<u8>>> + Send + Sync + 'static,
    options: &UploadOptions,
) -> Result<PinnedObject> {
    run_local(async {
        let opts = options.build();
        let snapshot = object.snapshot();
        let reader = dart_chunk_reader(source);
        let obj = sdk
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

/// Uploads the file at `path` to the Sia network. Behaves like [upload]
/// otherwise, but reads the file natively instead of streaming chunks across
/// the language boundary. Unsupported on web.
///
/// A free function rather than a method so the generated Dart class doesn't
/// gain a member that would shadow the `SdkStreams.uploadPath` extension method in
/// `lib/sia_storage.dart`.
pub async fn upload_path(
    sdk: &Sdk,
    object: &PinnedObject,
    path: String,
    options: &UploadOptions,
) -> Result<PinnedObject> {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (sdk, object, path, options);
        Err(anyhow!("path uploads are not supported on web"))
    }
    #[cfg(not(target_arch = "wasm32"))]
    run_local(async {
        let opts = options.build();
        let snapshot = object.snapshot();
        let obj = sdk
            .inner
            .upload_path(snapshot, path, opts)
            .await
            .map_err(|e| anyhow!("{e}"))?;
        Ok(PinnedObject {
            inner: Mutex::new(obj),
        })
    })
    .await
}

/// Begins a packed upload, batching multiple small objects into shared slabs
/// to avoid the per-object padding of [upload]. Add objects with
/// [packed_upload_add](super::upload::packed_upload_add) and complete the
/// upload with [PackedUpload::finalize](super::upload::PackedUpload::finalize).
///
/// A free function rather than a method so the generated Dart class doesn't
/// gain a member that would shadow the `SdkStreams.uploadPacked` extension method in
/// `lib/sia_storage.dart`.
#[frb(sync)]
pub fn upload_packed(sdk: &Sdk, options: &UploadOptions) -> Result<PackedUpload> {
    let upload = sdk
        .inner
        .upload_packed(options.build_packed())
        .map_err(|e| anyhow!("{e}"))?;
    Ok(PackedUpload {
        inner: Mutex::new(Some(upload)),
    })
}

/// Downloads an object from the Sia network as a stream of byte chunks.
///
/// A free function rather than a method so the generated Dart class doesn't
/// gain a member that would shadow the `SdkStreams.download` extension method in
/// `lib/sia_storage.dart`.
pub async fn download(
    sdk: &Sdk,
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

// ---- internal conversions ----

pub(crate) async fn stream_download(
    mut reader: sia_storage::Download,
    sink: StreamSink<Vec<u8>>,
) -> Result<()> {
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let n = reader.read(&mut buf).await.map_err(|e| anyhow!("{e}"))?;
        if n == 0 {
            break;
        }
        sink.add(buf[..n].to_vec()).map_err(|e| anyhow!("{e}"))?;
    }
    Ok(())
}

pub(crate) fn host_from_native(h: sia_storage::Host) -> Host {
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
