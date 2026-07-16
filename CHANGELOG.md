# Changelog

## 0.3.0 (2026-07-16)

### Breaking Changes

#### Rename `maxInflight` options to `maxBufferedSlabs`/`maxBufferedChunks`

sia_storage 0.10 replaces the concurrency-based `max_inflight` knob with memory-based buffering limits, and the option classes follow suit: `UploadOptions.maxInflight` is now `maxBufferedSlabs` (maximum slabs held in memory) and `DownloadOptions.maxInflight` is now `maxBufferedChunks` (maximum ~1 MiB chunks held in memory). Both default to 10% of system memory when unset.

```dart
// before
UploadOptions(dataShards: 10, parityShards: 20, maxInflight: 15);
DownloadOptions(maxInflight: 80);

// after
UploadOptions(dataShards: 10, parityShards: 20, maxBufferedSlabs: 15);
DownloadOptions(maxBufferedChunks: 80);
```

### Features

#### Update `sia_storage` to 0.10.0

Also updates the Dart dependencies (`hooks` 2.x, `code_assets` 1.2) and bumps the pinned Rust toolchain to 1.96.1, required by sia_storage 0.10's MSRV — consumer hook builds fetch it automatically via `rust-toolchain.toml`.

### Fixes

#### Fix Flutter Web bundle shipping with a mismatched content hash

0.2.0 shipped a stale `web/pkg` wasm whose `frb_get_rust_content_hash` did not match the Dart bindings, so flutter_rust_bridge refused to initialize in browsers ("content hash doesn't match Rust"). `tool/build_web.sh` now rebuilds the bundle from a clean slate — stale output is removed and the bindings are regenerated with `flutter_rust_bridge_codegen` pinned to the runtime version — and the publish workflow builds from the clean tag checkout and verifies the regenerated bindings match the release commit before publishing.

## 0.2.0 (2026-05-29)

### Breaking Changes

#### Rename `uploadStream`/`downloadStream` to `upload`/`download`

The stream-based helpers on `Sdk` are renamed to match the underlying SDK: `uploadStream` is now `upload` and `downloadStream` is now `download`. The raw callback/`StreamSink` bindings they wrapped are no longer exposed on the public `Sdk` type.

Migrate by dropping the `Stream` suffix:

```dart
// before
final upload = sdk.uploadStream(object: obj, source: file.openRead());
final download = sdk.downloadStream(object: obj);

// after
final upload = sdk.upload(object: obj, source: file.openRead());
final download = sdk.download(object: obj);
```

### Features

#### Add packed upload support

Adds `Sdk.uploadPacked`, which batches multiple small objects into shared slabs to avoid the per-object padding of a regular upload. Add objects to the returned session via `PackedUpload.add` (streaming bytes), then `finalize` to obtain the pinned objects. Per-shard progress is reported across the whole session.

#### Update `sia_storage` to 0.9.1

Bumps the bundled `sia_storage` crate from 0.8 to 0.9.1.

Upstream `sia_storage` changelog:

###### 0.9.1

- Bump sia_core_derive.

###### 0.9.0

Switch to `sia_reed_solomon` for erasure-coding. Replaced `reed-solomon-erasure` with `sia_reed_solomon`. Encoded parity bytes are compatible with existing `indexd` slabs.

120 MiB upload against the mock cluster (`cargo bench -p sia_storage --all-features`):

| | Before | After | Δ |
|---|---|---|---|
| `upload/90 inflight` | ~285 MiB/s | **611 MiB/s** | **+114%** |
| `upload/10 inflight` | ~175 MiB/s | **595 MiB/s** | **+240%** |
| `upload/default`     | ~184 MiB/s | **609 MiB/s** | **+231%** |

- Measure RPCAverage in throughput instead of latency to take into account transmitted bytes.

## 0.1.0

Initial release.

- Dart bindings for the [Sia Storage SDK](https://crates.io/crates/sia_storage) (`sia_storage` 0.8).
- Native targets compiled via Dart build hooks ([`native_toolchain_rust`](https://pub.dev/packages/native_toolchain_rust)) — no per-platform plugin scaffolding.
- Web/wasm support via prebuilt artifacts shipped under `web/pkg/`.
- Pure-Dart compatible — usable from Flutter, Dart server, or Dart CLI.
- API surface mirrors the Sia Storage NAPI bindings (`AppKey`, `Builder`, `Sdk`, `PinnedObject`, recovery phrases) with idiomatic Dart adaptations: streaming uploads via `Stream<List<int>>`, downloads as `Stream<Uint8List>`, and per-shard progress as `Stream<ShardProgress>`.
- Lazy native runtime initialization through the `Sia` facade — no manual `init()` for typical flows.
