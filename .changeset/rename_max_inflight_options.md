---
default: major
---

# Rename `maxInflight` options to `maxBufferedSlabs`/`maxBufferedChunks`

sia_storage 0.10 replaces the concurrency-based `max_inflight` knob with memory-based buffering limits, and the option classes follow suit: `UploadOptions.maxInflight` is now `maxBufferedSlabs` (maximum slabs held in memory) and `DownloadOptions.maxInflight` is now `maxBufferedChunks` (maximum ~1 MiB chunks held in memory). Both default to 10% of system memory when unset.

```dart
// before
UploadOptions(dataShards: 10, parityShards: 20, maxInflight: 15);
DownloadOptions(maxInflight: 80);

// after
UploadOptions(dataShards: 10, parityShards: 20, maxBufferedSlabs: 15);
DownloadOptions(maxBufferedChunks: 80);
```
