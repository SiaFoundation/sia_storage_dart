use std::sync::{Arc, Mutex};

use anyhow::Result;
use flutter_rust_bridge::frb;

use super::types::ShardProgress;

use crate::frb_generated::StreamSink;

/// Options for an upload, including an optional shard-progress stream.
///
/// Construct via [UploadOptions::new]; subscribe to per-shard progress by
/// calling [UploadOptions::shard_progress] before passing the options to
/// [Sdk::upload](super::sdk::Sdk::upload). The progress stream closes when
/// the upload completes (successfully or otherwise).
#[frb(opaque)]
pub struct UploadOptions {
    data_shards: Option<u8>,
    parity_shards: Option<u8>,
    max_inflight: Option<u32>,
    progress_sink: Mutex<Option<StreamSink<ShardProgress>>>,
}

#[cfg(target_arch = "wasm32")]
unsafe impl Send for UploadOptions {}
#[cfg(target_arch = "wasm32")]
unsafe impl Sync for UploadOptions {}

impl UploadOptions {
    /// Constructs an [UploadOptions] handle. All primitive arguments are
    /// optional; `None` keeps the SDK default.
    #[frb(sync)]
    pub fn new(
        data_shards: Option<u8>,
        parity_shards: Option<u8>,
        max_inflight: Option<u32>,
    ) -> Self {
        Self {
            data_shards,
            parity_shards,
            max_inflight,
            progress_sink: Mutex::new(None),
        }
    }

    /// Subscribe to per-shard upload progress. Returns a stream that emits
    /// one event per completed shard.
    pub fn shard_progress(&self, sink: StreamSink<ShardProgress>) -> Result<()> {
        *self
            .progress_sink
            .lock()
            .expect("upload options mutex poisoned") = Some(sink);
        Ok(())
    }

    pub(crate) fn build(&self) -> sia_storage::UploadOptions {
        let mut opts = sia_storage::UploadOptions::default();
        if let Some(v) = self.data_shards {
            opts.data_shards = v;
        }
        if let Some(v) = self.parity_shards {
            opts.parity_shards = v;
        }
        if let Some(v) = self.max_inflight {
            opts.max_inflight = v as usize;
        }
        let sink = self
            .progress_sink
            .lock()
            .expect("upload options mutex poisoned")
            .as_ref()
            .cloned();
        if let Some(sink) = sink {
            opts.shard_uploaded = Some(Arc::new(move |p: sia_storage::ShardProgress| {
                let _ = sink.add(shard_progress_from_native(p));
            }));
        }
        opts
    }
}

/// Options for a download, including an optional shard-progress stream.
#[frb(opaque)]
pub struct DownloadOptions {
    max_inflight: Option<u8>,
    offset: Option<u64>,
    length: Option<u64>,
    progress_sink: Mutex<Option<StreamSink<ShardProgress>>>,
}

#[cfg(target_arch = "wasm32")]
unsafe impl Send for DownloadOptions {}
#[cfg(target_arch = "wasm32")]
unsafe impl Sync for DownloadOptions {}

impl DownloadOptions {
    /// Constructs a [DownloadOptions] handle. All primitive arguments are
    /// optional; `None` keeps the SDK default.
    #[frb(sync)]
    pub fn new(max_inflight: Option<u8>, offset: Option<u64>, length: Option<u64>) -> Self {
        Self {
            max_inflight,
            offset,
            length,
            progress_sink: Mutex::new(None),
        }
    }

    /// Subscribe to per-shard download progress. Returns a stream that emits
    /// one event per completed shard.
    pub fn shard_progress(&self, sink: StreamSink<ShardProgress>) -> Result<()> {
        *self
            .progress_sink
            .lock()
            .expect("download options mutex poisoned") = Some(sink);
        Ok(())
    }

    pub(crate) fn build(&self) -> sia_storage::DownloadOptions {
        let mut opts = sia_storage::DownloadOptions::default();
        if let Some(v) = self.max_inflight {
            opts.max_inflight = v as usize;
        }
        if let Some(v) = self.offset {
            opts.offset = v;
        }
        opts.length = self.length;
        let sink = self
            .progress_sink
            .lock()
            .expect("download options mutex poisoned")
            .as_ref()
            .cloned();
        if let Some(sink) = sink {
            opts.shard_downloaded = Some(Arc::new(move |p: sia_storage::ShardProgress| {
                let _ = sink.add(shard_progress_from_native(p));
            }));
        }
        opts
    }
}

fn shard_progress_from_native(p: sia_storage::ShardProgress) -> ShardProgress {
    ShardProgress {
        host_key: p.host_key.to_string(),
        shard_size: p.shard_size as u32,
        shard_index: p.shard_index as u32,
        slab_index: p.slab_index as u32,
        elapsed_ms: p.elapsed.as_secs_f64() * 1000.0,
    }
}
