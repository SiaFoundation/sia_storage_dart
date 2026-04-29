use std::str::FromStr;
use std::sync::Mutex;

use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};
use flutter_rust_bridge::frb;
use sia_core::signing::Signature;
use sia_core::types::Hash256;

use super::keys::AppKey;
use super::types::{PinnedSector, SealedObject, Slab};

/// An object pinned (or being pinned) to an indexer.
///
/// New objects are constructed via [PinnedObject::new] and uploaded with
/// [Sdk::upload](super::sdk::Sdk::upload). Use [PinnedObject::seal] /
/// [PinnedObject::open] to round-trip an object through opaque
/// [SealedObject] form for offline storage.
#[frb(opaque)]
pub struct PinnedObject {
    pub(crate) inner: Mutex<sia_storage::Object>,
}

#[cfg(target_arch = "wasm32")]
unsafe impl Send for PinnedObject {}
#[cfg(target_arch = "wasm32")]
unsafe impl Sync for PinnedObject {}

impl PinnedObject {
    /// Creates a new empty object.
    #[frb(sync)]
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(sia_storage::Object::default()),
        }
    }

    /// Opens a sealed object using the provided app key.
    #[frb(sync)]
    pub fn open(app_key: &AppKey, sealed: SealedObject) -> Result<Self> {
        let sealed: sia_storage::SealedObject = sealed_to_native(sealed)?;
        let obj = sealed
            .open(&app_key.inner)
            .map_err(|e| anyhow!("{e}"))?;
        Ok(Self {
            inner: Mutex::new(obj),
        })
    }

    /// Seals the object for offline storage.
    #[frb(sync)]
    pub fn seal(&self, app_key: &AppKey) -> SealedObject {
        let inner = self.inner.lock().expect("object mutex poisoned");
        sealed_from_native(inner.seal(&app_key.inner))
    }

    /// Returns the object's ID, hex-encoded.
    #[frb(sync)]
    pub fn id(&self) -> String {
        self.inner
            .lock()
            .expect("object mutex poisoned")
            .id()
            .to_string()
    }

    /// Returns the total size of the object in bytes.
    #[frb(sync)]
    pub fn size(&self) -> u64 {
        self.inner.lock().expect("object mutex poisoned").size()
    }

    /// Returns the total encoded size after erasure coding.
    #[frb(sync)]
    pub fn encoded_size(&self) -> u64 {
        self.inner
            .lock()
            .expect("object mutex poisoned")
            .encoded_size()
    }

    /// Returns the slabs that make up the object.
    #[frb(sync)]
    pub fn slabs(&self) -> Vec<Slab> {
        self.inner
            .lock()
            .expect("object mutex poisoned")
            .slabs()
            .iter()
            .cloned()
            .map(slab_from_native)
            .collect()
    }

    /// Returns the user-supplied metadata associated with the object.
    #[frb(sync)]
    pub fn metadata(&self) -> Vec<u8> {
        self.inner
            .lock()
            .expect("object mutex poisoned")
            .metadata
            .clone()
    }

    /// Replaces the user-supplied metadata.
    #[frb(sync)]
    pub fn update_metadata(&self, metadata: Vec<u8>) {
        self.inner
            .lock()
            .expect("object mutex poisoned")
            .metadata = metadata;
    }

    /// Returns the time the object was created.
    #[frb(sync)]
    pub fn created_at(&self) -> DateTime<Utc> {
        *self
            .inner
            .lock()
            .expect("object mutex poisoned")
            .created_at()
    }

    /// Returns the time the object was last updated.
    #[frb(sync)]
    pub fn updated_at(&self) -> DateTime<Utc> {
        *self
            .inner
            .lock()
            .expect("object mutex poisoned")
            .updated_at()
    }

    /// Snapshot the inner object for use with sia_storage methods.
    pub(crate) fn snapshot(&self) -> sia_storage::Object {
        self.inner
            .lock()
            .expect("object mutex poisoned")
            .clone()
    }
}

impl Default for PinnedObject {
    fn default() -> Self {
        Self::new()
    }
}

/// An object event from the indexer.
#[frb(opaque)]
pub struct ObjectEvent {
    pub(crate) id: String,
    pub(crate) deleted: bool,
    pub(crate) updated_at: DateTime<Utc>,
    pub(crate) object: Option<Mutex<sia_storage::Object>>,
}

#[cfg(target_arch = "wasm32")]
unsafe impl Send for ObjectEvent {}
#[cfg(target_arch = "wasm32")]
unsafe impl Sync for ObjectEvent {}

impl ObjectEvent {
    #[frb(sync, getter)]
    pub fn id(&self) -> String {
        self.id.clone()
    }

    #[frb(sync, getter)]
    pub fn deleted(&self) -> bool {
        self.deleted
    }

    #[frb(sync, getter)]
    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    #[frb(sync, getter)]
    pub fn object(&self) -> Option<PinnedObject> {
        self.object.as_ref().map(|m| PinnedObject {
            inner: Mutex::new(m.lock().expect("event object mutex poisoned").clone()),
        })
    }
}

/// Calculates the encoded size of data given the original size and erasure
/// coding parameters.
#[frb(sync)]
pub fn encoded_size(size: u64, data_shards: u8, parity_shards: u8) -> u64 {
    sia_storage::encoded_size(size, data_shards, parity_shards)
}

// ---- internal conversions used by this module and sdk.rs ----

pub(crate) fn slab_from_native(s: sia_storage::Slab) -> Slab {
    Slab {
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
        offset: s.offset,
        length: s.length,
    }
}

pub(crate) fn slab_to_native(s: Slab) -> Result<sia_storage::Slab> {
    Ok(sia_storage::Slab {
        encryption_key: sia_storage::EncryptionKey::try_from(s.encryption_key.as_slice())
            .map_err(|e| anyhow!("{e}"))?,
        min_shards: s.min_shards,
        sectors: s
            .sectors
            .into_iter()
            .map(|sec| {
                Ok(sia_storage::Sector {
                    host_key: sia_core::signing::PublicKey::from_str(&sec.host_key)
                        .map_err(|e| anyhow!("{e}"))?,
                    root: Hash256::from_str(&sec.root).map_err(|e| anyhow!("{e}"))?,
                })
            })
            .collect::<Result<_>>()?,
        offset: s.offset,
        length: s.length,
    })
}

pub(crate) fn sealed_from_native(o: sia_storage::SealedObject) -> SealedObject {
    SealedObject {
        id: o.id().to_string(),
        encrypted_data_key: o.encrypted_data_key.clone(),
        encrypted_metadata_key: o.encrypted_metadata_key.clone(),
        slabs: o.slabs.iter().cloned().map(slab_from_native).collect(),
        encrypted_metadata: o.encrypted_metadata.clone(),
        data_signature: o.data_signature.as_ref().to_vec(),
        metadata_signature: o.metadata_signature.as_ref().to_vec(),
        created_at: o.created_at,
        updated_at: o.updated_at,
    }
}

pub(crate) fn sealed_to_native(s: SealedObject) -> Result<sia_storage::SealedObject> {
    let id_expected = s.id.clone();
    let sealed = sia_storage::SealedObject {
        encrypted_data_key: s.encrypted_data_key,
        encrypted_metadata_key: s.encrypted_metadata_key,
        slabs: s
            .slabs
            .into_iter()
            .map(slab_to_native)
            .collect::<Result<_>>()?,
        encrypted_metadata: s.encrypted_metadata,
        data_signature: Signature::try_from(s.data_signature.as_slice())
            .map_err(|e| anyhow!("invalid data signature: {e}"))?,
        metadata_signature: Signature::try_from(s.metadata_signature.as_slice())
            .map_err(|e| anyhow!("invalid metadata signature: {e}"))?,
        created_at: s.created_at,
        updated_at: s.updated_at,
    };
    if sealed.id().to_string() != id_expected {
        return Err(anyhow!("sealed object contents mismatch"));
    }
    Ok(sealed)
}
