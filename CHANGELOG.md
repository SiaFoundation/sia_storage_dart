# Changelog

## 0.4.1 (2026-10-28)

### Features

- Host performance metrics and the price cache are now shared across all SDK instances.

#### List object events without slabs

Add `Sdk.objectEventSummaries`, which lists the same object events as `Sdk.objectEvents` without the objects' slabs, making the response much smaller. Each `ObjectEventSummary` has an `ObjectSummary` with the object's id, size, decrypted metadata and timestamps, or `null` when the object was deleted, and paginates with the same `ObjectsCursor`. Fetch the full object with `Sdk.object` to download it.

```dart
final events = await sdk.objectEventSummaries(cursor: null, limit: 100);
```

### Fixes

- Stop sending a chunk's trailing frame in a packet of its own
- Fixed an Android crash on the first HTTPS request in apps that load React Native.
- Appending data now efficiently repacks the last slab when it is not full.

#### Fix connecting to hosts that announce an IPv6 address

An announced address is `host:port` with an IPv6 host bracketed. The siamux client split it on the last colon, which left the brackets attached to the host, and `getaddrinfo` rejects those, so every IPv6 host failed to resolve and no shard could reach it. The address is now resolved whole.

#### Fix a crash reading a slab with a host the SDK does not know

A slab whose sectors include a host missing from the SDK's host list, such as one an indexer stopped listing, crashed the download with "index out of bounds: the len is 29 but the index is 29". Sectors on unknown hosts are dropped before reading, and the array the recovered shards are stored in was sized from the sectors left rather than from the slab's shard count, while each shard is stored at its index in the slab. A read from the host at the slab's last index then wrote past the end. On web the panic stops the WebAssembly module, so every stream in the page froze with no error. The array is now sized from the slab's shard count.

#### Take a read's account token once the stream to the host is open

A sector read took its account token before reaching the host. When reaching the host took a long time, as in a browser delaying WebTransport connections after failed handshakes, the token expired on the way and the host rejected the read with "account token expired". The transports now take the token from the token source after the stream to the host is open.

#### Upgrade reqwest and update TLS security dependencies

Upgrade reqwest to 0.13.5, using platform certificate verification on native platforms except Android. Android uses a preconfigured Rustls client with bundled roots to avoid requiring JNI initialization. Web builds continue using browser TLS.

Update `webpki-roots` to 1.0.10 and `rustls-webpki` to 0.103.16 in the lockfile to address RUSTSEC-2026-0381 and RUSTSEC-2026-0382.

#### Close WebTransport streams whose open was abandoned

When opening a bidirectional stream timed out or the read waiting on it was cancelled, a stream the browser opened afterwards was never used or closed. These streams piled up on the host's session until it stopped accepting new ones, and reads from that host stalled. A stream that opens after its caller gave up is now closed right away.

#### Stop redialing hosts that refused a WebTransport handshake

Chrome counts every failed WebTransport handshake against the whole browser for five minutes, and while any are counted it delays each new connection by 10 ms doubled per failure, up to a minute. The web client dialed a host again on the next read after its handshake failed, so a few unreachable hosts kept adding failures, and within minutes dials to healthy hosts took 35 to 60 seconds. An address that refuses a handshake is now not dialed again for five minutes, and a host whose addresses have all refused fails at once, so the download moves to other hosts. A dial that only times out is not counted, since Chrome can hold a healthy host back for up to a minute while it is counting failures.

## 0.4.0 (2026-10-20)

### Breaking Changes

#### Add `version` to `Slab`

`Slab` has a new required `version` field, matching sia_storage's slab versions. Code that constructs `Slab` or `SealedObject` values by hand must pass it.

#### Remove `Sdk.slab` and `PinnedSlab`

sia_storage 0.13 removed the slab lookup, so `Sdk.slab` and the `PinnedSlab` type are gone.

#### Rename `AppKey.export_` to `AppKey.save`

The generated `export_()` name came from `export` being reserved in Dart. `AppKey.save()` returns the same 32-byte buffer. Pass it to `Sia.appKey` to restore the key.

```dart
// before
final bytes = appKey.export_();

// after
final bytes = appKey.save();
```

#### Rename the share URL methods on `Sdk`

`Sdk.shareObject` and `Sdk.sharedObject` are now `Sdk.objectShareUrl` and `Sdk.objectFromShareUrl`. `shareObject` now attaches an object to a sharing key.

```dart
// before
final url = sdk.shareObject(object: obj, validUntil: expiry);
final shared = await sdk.sharedObject(sharedUrl: url);

// after
final url = sdk.objectShareUrl(object: obj, validUntil: expiry);
final shared = await sdk.objectFromShareUrl(shareUrl: url);
```

### Features

#### Add `Builder.connectPreAuthorized`

Connects with a pre-authorized key that the indexer operator provisions out of band. It runs the connect, approval, and registration steps in one call and returns a ready `Sdk`, without the interactive approval flow.

#### Add path-based uploads

`Sdk.uploadPath` and `PackedUpload.addPath` upload a file by path. The file is read natively instead of being streamed through Dart chunk by chunk. They aren't available on web, where both throw.

```dart
final upload = sdk.uploadPath(object: PinnedObject(), path: '/path/to/file');
final object = await upload.result;
```

#### Add `Builder.reconnecting` and `Builder.matchesExistingAppKey`

After `waitForApproval`, `reconnecting` reports whether the approved connect key already has an account for the application. `matchesExistingAppKey` checks whether a recovery phrase derives an app key that is already registered. `register` now registers a new app key when the recovery phrase differs from the existing account's, instead of failing.

#### Add sharing keys for read-only access to objects

A sharing key grants read-only access to a chosen set of objects. The recipient doesn't need an account, an app registration, or an approval flow. Owners create a key with `Sdk.createSharingKey`, attach objects with `Sdk.shareObject`, and hand out the key's seed from `SharingKey.save`. They manage keys with `sharingKeys`, `sharingKey`, `sharedObjects`, `unshareObject`, and `revokeSharingKey`.

Recipients connect with `Sia.sharedSdk` and get a `SharedSdk` with `stats`, `object`, `objects`, `objectSummaries`, `hosts`, and `download`. Reads are paid from the owner's account.

```dart
final key = await sdk.createSharingKey(description: 'photos', expiresAt: null);
await sdk.shareObject(key: key, object: obj);

final shared = await Sia.sharedSdk(indexerUrl: url, sharingKey: key.save());
final objects = await shared.objects(offset: null, limit: null);
```

#### Add `PinnedObject.truncate` and `UploadOptions.startOffset`

`truncate` returns a copy of the object cut to the given length. `startOffset` makes an upload overwrite the object from that byte offset instead of appending, so only the slabs covering the rewritten range are uploaded again. Re-pin the object after either change.

#### Take a cutoff on `pruneSlabs`

`Sdk.pruneSlabs` takes an optional `before` timestamp. Only slabs pinned before it are pruned. Passing `null` leaves the cutoff to the indexer, which spares recently pinned slabs so an upload in progress is not pruned.

```dart
// before
await sdk.pruneSlabs();

// after
await sdk.pruneSlabs(before: null);
```

#### Update `sia_storage` to 0.13.1

Uploads and downloads pick up the SDK's transfer improvements: per-RPC idle timeouts in place of fixed deadlines, a shared connection pool, more parallel WebTransport transfers, CBOR indexer responses, and automatic reuploads of slabs the indexer rejects as too old.

## 0.3.1 (2026-07-16)

### Fixes

#### Restore resolvability with Flutter stable

`hooks` is now constrained to `>=2.0.2 <2.1.0`: hooks 2.1.0 requires `meta ^1.19.0`, which conflicts with the `meta 1.17.0` pinned by Flutter stable (3.41), making 0.3.0 unresolvable in Flutter apps. CI now resolves the package from a scratch Flutter app against the stable channel to catch pin conflicts before release.

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
