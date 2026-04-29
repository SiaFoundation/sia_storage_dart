use anyhow::{Result, anyhow};
use flutter_rust_bridge::frb;
use sia_core::signing::Signature;

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
