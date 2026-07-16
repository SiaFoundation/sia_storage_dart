# sia_storage_flutter

Dart bindings for the [Sia Storage SDK](https://crates.io/crates/sia_storage). Upload, download, and pin objects on the Sia network from Flutter and pure Dart applications.

Powered by [`flutter_rust_bridge`](https://pub.dev/packages/flutter_rust_bridge) and Dart build hooks ([`native_toolchain_rust`](https://pub.dev/packages/native_toolchain_rust)) — no per-platform plugin scaffolding required.

## Installation

```yaml
dependencies:
  sia_storage_flutter: ^0.3.0
```

The native library is compiled at consumer build time via a Dart build hook, so all consumers need a working Rust toolchain (`rustup`) on their build machine or in CI.

## Quick start

```dart
import 'package:sia_storage/sia_storage.dart';

const appMeta = AppMetadata(
  id: <32 bytes>,
  name: 'My App',
  description: 'Demo app',
  serviceUrl: 'https://example.com',
);

// First call auto-initializes the native runtime.
final builder = await Sia.builder(
  indexerUrl: 'https://indexer.example.com',
  appMeta: appMeta,
);

// Either reconnect with an existing key…
final existingKey = await Sia.appKey(savedSeed);
final sdk = await builder.connected(appKey: existingKey);

// …or onboard a fresh app (request connection, await approval, register).
// final phrase = await Sia.generateRecoveryPhrase();
// await builder.requestConnection();
// final approvalUrl = builder.responseUrl();
// await builder.waitForApproval();
// final sdk = await builder.register(mnemonic: phrase);

final hosts = await sdk.hosts();
```

## Streaming uploads and downloads

Upload from any `Stream<List<int>>` (e.g. `File.openRead()`); per-shard progress is co-emitted on a separate stream:

```dart
final obj = await Sia.pinnedObject();
final upload = sdk.uploadStream(
  object: obj,
  source: file.openRead(),
  options: const UploadOptions(dataShards: 10, parityShards: 20),
);
upload.progress.listen((p) => print('shard ${p.shardIndex}: ${p.elapsedMs}ms'));
final pinned = await upload.result;
await sdk.pinObject(object: pinned);
```

Download:

```dart
final dl = sdk.downloadStream(object: pinned);
dl.progress.listen((p) => /* progress UI */);
await for (final chunk in dl.data) {
  // write chunk to disk, network, etc.
}
```

## Platform support

| Platform | Status | Notes |
|----------|--------|-------|
| iOS | ✅ | hook-built per consumer |
| Android | ✅ | hook-built per consumer |
| macOS | ✅ | hook-built per consumer |
| Windows | ✅ | hook-built per consumer |
| Linux | ✅ | hook-built per consumer |
| Flutter Web | ✅ | prebuilt wasm shipped in package; see below |
| Pure Dart (server / CLI / `webdev`) | ✅ | native consumers only; `webdev` users handle wasm asset copy themselves |

### Flutter Web setup

Flutter Web has two requirements beyond the standard `pub add`. **Both are required** — skip either and the app will throw on the first SDK call.

**1. Copy the wasm bundle into your app's `web/` folder.** Flutter Web only serves files from the app's own `web/`, so the prebuilt bundle the package ships at `web/pkg/` has to be copied in:

```bash
# from your Flutter app's root, after `flutter pub add sia_storage_flutter`
mkdir -p web/pkg
cp -r ~/.pub-cache/hosted/pub.dev/sia_storage-*/web/pkg/* web/pkg/
```

**2. Set cross-origin isolation headers.** flutter_rust_bridge's runtime spins up Web Workers and shares the wasm `Memory` between them via `SharedArrayBuffer`, which browsers gate behind cross-origin isolation. Without these headers the app panics with `DataCloneError: ... #<Memory> could not be cloned`.

For local dev:
```bash
flutter run -d chrome \
  --web-header=Cross-Origin-Opener-Policy=same-origin \
  --web-header=Cross-Origin-Embedder-Policy=require-corp
```

For production (set on every response from your web server):
```
Cross-Origin-Opener-Policy: same-origin
Cross-Origin-Embedder-Policy: require-corp
```

These two together put the page in cross-origin-isolated mode, which `SharedArrayBuffer` requires post-Spectre. See [MDN: COOP/COEP](https://developer.mozilla.org/en-US/docs/Web/HTTP/Cross-Origin_Opener_Policy) for details.

## API entry points

- [`Sia.builder`](lib/sia_storage.dart) — onboarding flow
- [`Sia.appKey`](lib/sia_storage.dart) — reconnect with a saved key
- [`Sia.generateRecoveryPhrase`](lib/sia_storage.dart) — BIP-39 12-word phrase
- [`Sia.validateRecoveryPhrase`](lib/sia_storage.dart) — phrase validation
- [`Sia.ready`](lib/sia_storage.dart) — explicit pre-warm (rarely needed)

Once a `Builder`, `Sdk`, `PinnedObject`, etc. is in hand, all subsequent methods are direct sync or async calls on those handles.

## License

MIT — see [LICENSE](LICENSE).
