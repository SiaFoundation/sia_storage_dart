use chrono::{DateTime, Utc};

/// Metadata about an application connecting to the indexer.
pub struct AppMetadata {
    pub id: Vec<u8>,
    pub name: String,
    pub description: String,
    pub service_url: String,
    pub logo_url: Option<String>,
    pub callback_url: Option<String>,
}

/// The protocol used in a network address.
pub enum AddressProtocol {
    SiaMux,
    Quic,
}

/// A network address of a storage provider on the Sia network.
pub struct NetAddress {
    pub protocol: AddressProtocol,
    pub address: String,
}

/// Information about a storage provider on the Sia network.
pub struct Host {
    pub public_key: String,
    pub addresses: Vec<NetAddress>,
    pub country_code: String,
    pub latitude: f64,
    pub longitude: f64,
    pub good_for_upload: bool,
}

/// A sector stored on a specific host.
#[derive(Clone)]
pub struct PinnedSector {
    pub root: String,
    pub host_key: String,
}

/// A pinned slab from the indexer.
pub struct PinnedSlab {
    pub id: String,
    pub encryption_key: Vec<u8>,
    pub min_shards: u8,
    pub sectors: Vec<PinnedSector>,
}

/// A slab representing a contiguous erasure-coded segment of a file.
pub struct Slab {
    pub encryption_key: Vec<u8>,
    pub min_shards: u8,
    pub sectors: Vec<PinnedSector>,
    pub offset: u32,
    pub length: u32,
}

/// A sealed object for offline storage.
pub struct SealedObject {
    pub id: String,
    pub encrypted_data_key: Vec<u8>,
    pub encrypted_metadata_key: Vec<u8>,
    pub slabs: Vec<Slab>,
    pub encrypted_metadata: Vec<u8>,
    pub data_signature: Vec<u8>,
    pub metadata_signature: Vec<u8>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A cursor for paginating through objects.
pub struct ObjectsCursor {
    pub id: String,
    pub after: DateTime<Utc>,
}

/// Application info.
pub struct App {
    pub id: String,
    pub name: String,
    pub description: String,
    pub service_url: Option<String>,
    pub logo_url: Option<String>,
}

/// An account registered on the indexer.
pub struct Account {
    pub account_key: String,
    pub max_pinned_data: u64,
    pub remaining_storage: u64,
    pub pinned_data: u64,
    pub pinned_size: u64,
    pub ready: bool,
    pub app: App,
    pub last_used: DateTime<Utc>,
}

/// Progress information about a successfully uploaded or downloaded shard.
#[derive(Clone)]
pub struct ShardProgress {
    pub host_key: String,
    pub shard_size: u32,
    pub shard_index: u32,
    pub slab_index: u32,
    pub elapsed_ms: f64,
}

// UploadOptions and DownloadOptions live in `super::options` as opaque
// types so they can carry frb-bound progress callbacks alongside their
// primitive fields.
