use anyhow::{Result, anyhow};
use flutter_rust_bridge::frb;
use sia_core::signing::Signature;

use super::types::KeyStats;

/// An AppKey is used to sign requests to the indexer.
///
/// AppKeys can be registered with an indexer during onboarding with a
/// [Builder](super::builder::Builder). They are derived from a BIP-39 recovery
/// phrase, which can be generated using [generate_recovery_phrase].
///
/// They must be stored securely by the application and never shared publicly.
#[frb(opaque)]
pub struct AppKey {
    pub(crate) inner: sia_storage::AppKey,
}

// On wasm32, sia_storage's internals use Rc instead of Arc; the type is
// !Send + !Sync. wasm32 is single-threaded, so we assert Send + Sync
// manually for frb's bounds.
#[cfg(target_arch = "wasm32")]
unsafe impl Send for AppKey {}
#[cfg(target_arch = "wasm32")]
unsafe impl Sync for AppKey {}

impl AppKey {
    /// Imports an AppKey from a 32-byte buffer.
    #[frb(sync)]
    pub fn new(key: Vec<u8>) -> Result<Self> {
        let seed: [u8; 32] = key
            .as_slice()
            .try_into()
            .map_err(|_| anyhow!("app keys must be 32 bytes"))?;
        Ok(AppKey {
            inner: sia_storage::AppKey::import(seed),
        })
    }

    /// Exports the AppKey as a 32-byte buffer.
    #[frb(sync)]
    pub fn export(&self) -> Vec<u8> {
        self.inner.export().to_vec()
    }

    /// Signs a message using the AppKey.
    #[frb(sync)]
    pub fn sign(&self, message: Vec<u8>) -> Vec<u8> {
        self.inner.sign(&message).as_ref().to_vec()
    }

    /// Returns the public key corresponding to the AppKey, hex-encoded.
    #[frb(sync)]
    pub fn public_key(&self) -> String {
        self.inner.public_key().to_string()
    }

    /// Verifies a signature for a given message using the AppKey.
    #[frb(sync)]
    pub fn verify_signature(&self, message: Vec<u8>, signature: Vec<u8>) -> Result<bool> {
        let sig = Signature::try_from(signature.as_slice())
            .map_err(|e| anyhow!("invalid signature: {e}"))?;
        Ok(self.inner.public_key().verify(&message, &sig))
    }
}

/// A SharingKey grants read-only access to the objects attached to it.
///
/// Owners create one with [Sdk::create_sharing_key](super::sdk::Sdk::create_sharing_key)
/// and hand out its exported seed; recipients connect with
/// [SharedSdk::connect](super::shared::SharedSdk::connect).
#[frb(opaque)]
pub struct SharingKey {
    pub(crate) inner: sia_storage::SharingKey,
}

#[cfg(target_arch = "wasm32")]
unsafe impl Send for SharingKey {}
#[cfg(target_arch = "wasm32")]
unsafe impl Sync for SharingKey {}

impl SharingKey {
    /// Imports a SharingKey from a 32-byte seed.
    #[frb(sync)]
    pub fn new(seed: Vec<u8>) -> Result<Self> {
        let seed: [u8; 32] = seed
            .as_slice()
            .try_into()
            .map_err(|_| anyhow!("sharing keys must be 32 bytes"))?;
        Ok(SharingKey {
            inner: sia_storage::SharingKey::import(seed),
        })
    }

    /// Exports the SharingKey's seed as a 32-byte buffer.
    #[frb(sync)]
    pub fn export(&self) -> Vec<u8> {
        self.inner.export().to_vec()
    }

    /// Returns the public key corresponding to the SharingKey, hex-encoded.
    #[frb(sync)]
    pub fn public_key(&self) -> String {
        self.inner.public_key().to_string()
    }
}

/// A sharing key owned by the account, along with its description and stats.
#[frb(opaque)]
pub struct SharingKeyRecord {
    pub(crate) inner: sia_storage::KeyRecord,
}

#[cfg(target_arch = "wasm32")]
unsafe impl Send for SharingKeyRecord {}
#[cfg(target_arch = "wasm32")]
unsafe impl Sync for SharingKeyRecord {}

impl SharingKeyRecord {
    #[frb(sync, getter)]
    pub fn key(&self) -> SharingKey {
        SharingKey {
            inner: self.inner.key.clone(),
        }
    }

    #[frb(sync, getter)]
    pub fn description(&self) -> String {
        self.inner.description.clone()
    }

    #[frb(sync, getter)]
    pub fn stats(&self) -> KeyStats {
        key_stats_from_native(self.inner.stats.clone())
    }
}

/// Generates a new BIP-39 12-word recovery phrase.
#[frb(sync)]
pub fn generate_recovery_phrase() -> String {
    sia_storage::generate_recovery_phrase()
}

/// Validates a BIP-39 recovery phrase.
#[frb(sync)]
pub fn validate_recovery_phrase(phrase: String) -> Result<()> {
    sia_storage::validate_recovery_phrase(&phrase).map_err(|e| anyhow!("{e}"))
}

pub(crate) fn key_stats_from_native(s: sia_storage::KeyStats) -> KeyStats {
    KeyStats {
        object_count: s.object_count,
        object_size: s.object_size,
        pinned_data: s.pinned_data,
        pinned_size: s.pinned_size,
        expires_at: s.expires_at,
        created_at: s.created_at,
        updated_at: s.updated_at,
    }
}
