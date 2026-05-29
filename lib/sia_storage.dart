/// Dart bindings for the Sia Storage SDK.
///
/// Use [Sia.builder] (or [Sia.appKey] when reconnecting) as your first call —
/// these auto-initialize the native runtime. Everything downstream is sync
/// or async per its method signature.
///
/// ```dart
/// final builder = await Sia.builder(
///   indexerUrl: 'https://indexer.example.com',
///   appMeta: const AppMetadata(...),
/// );
/// final sdk = await builder.connected(appKey: existingKey);
/// final hosts = await sdk.hosts();
/// ```
library;

import 'dart:async';
import 'dart:typed_data';

import 'src/rust/api/builder.dart';
import 'src/rust/api/keys.dart' hide generateRecoveryPhrase, validateRecoveryPhrase;
import 'src/rust/api/keys.dart' as raw_keys;
import 'src/rust/api/object.dart';
import 'src/rust/api/options.dart' as raw_options;
import 'src/rust/api/sdk.dart';
import 'src/rust/api/sdk.dart' as raw_sdk;
import 'src/rust/api/types.dart';
import 'src/rust/api/upload.dart';
import 'src/rust/frb_generated.dart' show RustLib;

export 'src/rust/api/builder.dart' show Builder;
export 'src/rust/api/keys.dart' show AppKey;
export 'src/rust/api/object.dart' show ObjectEvent, PinnedObject, encodedSize;
export 'src/rust/api/sdk.dart' show Sdk;
export 'src/rust/api/upload.dart' show PackedUpload;
export 'src/rust/api/types.dart'
    show
        Account,
        AddressProtocol,
        App,
        AppMetadata,
        Host,
        NetAddress,
        ObjectsCursor,
        PinnedSector,
        PinnedSlab,
        SealedObject,
        ShardProgress,
        Slab;

/// Top-level entry points that auto-initialize the native runtime.
///
/// Use these for the *first* call from your application. Subsequent
/// operations on the resulting handles call regular sync/async methods on
/// those handles directly.
abstract final class Sia {
  static Future<void>? _initFuture;

  /// Resolves once the native runtime is loaded. Implicitly awaited by every
  /// other [Sia] entry point; call it directly only to pre-warm the library.
  static Future<void> ready() => _initFuture ??= RustLib.init();

  /// Releases the native runtime. Mainly useful in tests.
  static void dispose() {
    RustLib.dispose();
    _initFuture = null;
  }

  /// Constructs a [Builder] for onboarding the application to an indexer.
  static Future<Builder> builder({
    required String indexerUrl,
    required AppMetadata appMeta,
  }) async {
    await ready();
    return Builder(indexerUrl: indexerUrl, appMeta: appMeta);
  }

  /// Imports an [AppKey] from a 32-byte buffer. Use this when reconnecting
  /// with a previously saved key.
  static Future<AppKey> appKey(List<int> key) async {
    await ready();
    return AppKey(key: key);
  }

  /// Generates a new BIP-39 12-word recovery phrase.
  static Future<String> generateRecoveryPhrase() async {
    await ready();
    return raw_keys.generateRecoveryPhrase();
  }

  /// Validates a BIP-39 recovery phrase. Throws on invalid input.
  static Future<void> validateRecoveryPhrase(String phrase) async {
    await ready();
    raw_keys.validateRecoveryPhrase(phrase: phrase);
  }
}

/// Upload configuration. `const`-constructible.
class UploadOptions {
  /// The number of data shards per slab. Defaults to 10 when null.
  final int? dataShards;

  /// The number of parity shards per slab. Defaults to 20 when null.
  final int? parityShards;

  /// Maximum number of concurrent shard uploads. Defaults to 15 when null.
  final int? maxInflight;

  const UploadOptions({
    this.dataShards,
    this.parityShards,
    this.maxInflight,
  });
}

/// Download configuration. `const`-constructible.
class DownloadOptions {
  /// Maximum number of concurrent chunk downloads. Defaults to 80 when null.
  final int? maxInflight;

  /// Byte offset to start downloading from. Defaults to 0 when null.
  final BigInt? offset;

  /// Number of bytes to download. Downloads to EOF when null.
  final BigInt? length;

  const DownloadOptions({
    this.maxInflight,
    this.offset,
    this.length,
  });
}

/// Handle for an in-flight upload. Subscribe to [progress] before awaiting
/// [result] to avoid missing early events.
class Upload {
  /// Resolves with the [PinnedObject] when the upload completes.
  final Future<PinnedObject> result;

  /// Per-shard progress events. Closes when the upload finishes.
  final Stream<ShardProgress> progress;

  Upload._(this.result, this.progress);
}

/// Handle for an in-flight download. Subscribe to [progress] before reading
/// from [data] to avoid missing early events.
class Download {
  /// The downloaded byte stream.
  final Stream<Uint8List> data;

  /// Per-shard progress events. Closes when the download finishes.
  final Stream<ShardProgress> progress;

  Download._(this.data, this.progress);
}

/// Handle for an in-flight packed upload. Subscribe to [progress] before
/// adding objects to avoid missing early events. Add objects to [upload] via
/// [PackedUploadStreams.add], then await [PackedUpload.finalize] to
/// obtain the pinned objects.
class PackedUploadSession {
  /// The packed upload to add objects to and finalize.
  final PackedUpload upload;

  /// Per-shard progress events across all packed objects. Closes when the
  /// upload is finalized.
  final Stream<ShardProgress> progress;

  PackedUploadSession._(this.upload, this.progress);
}

/// Stream-friendly upload and download helpers.
extension SdkStreams on Sdk {
  /// Uploads an object by streaming bytes from `source` (e.g.
  /// `File.openRead()`).
  Upload upload({
    required PinnedObject object,
    required Stream<List<int>> source,
    UploadOptions options = const UploadOptions(),
  }) {
    final raw = raw_options.UploadOptions(
      dataShards: options.dataShards,
      parityShards: options.parityShards,
      maxInflight: options.maxInflight,
    );
    final progress = raw.shardProgress();
    final iterator = StreamIterator<List<int>>(source);
    final result = raw_sdk.upload(
      sdk: this,
      object: object,
      source: () async {
        if (!await iterator.moveNext()) return null;
        final chunk = iterator.current;
        return chunk is Uint8List ? chunk : Uint8List.fromList(chunk);
      },
      options: raw,
    );
    return Upload._(result, progress);
  }

  /// Downloads an object's bytes.
  Download download({
    required PinnedObject object,
    DownloadOptions options = const DownloadOptions(),
  }) {
    final raw = raw_options.DownloadOptions(
      maxInflight: options.maxInflight,
      offset: options.offset,
      length: options.length,
    );
    final progress = raw.shardProgress();
    final data = raw_sdk.download(sdk: this, object: object, options: raw);
    return Download._(data, progress);
  }

  /// Begins a packed upload, batching multiple small objects into shared slabs
  /// to avoid the per-object padding of [upload]. Add objects to the
  /// returned [PackedUploadSession.upload], then finalize it to obtain the
  /// pinned objects.
  PackedUploadSession uploadPacked({
    UploadOptions options = const UploadOptions(),
  }) {
    final raw = raw_options.UploadOptions(
      dataShards: options.dataShards,
      parityShards: options.parityShards,
      maxInflight: options.maxInflight,
    );
    final progress = raw.shardProgress();
    final upload = raw_sdk.uploadPacked(sdk: this, options: raw);
    return PackedUploadSession._(upload, progress);
  }
}

/// Stream-friendly helper for adding objects to a [PackedUpload].
extension PackedUploadStreams on PackedUpload {
  /// Adds an object by streaming bytes from `source` (e.g.
  /// `File.openRead()`). Returns the number of bytes consumed.
  Future<BigInt> add(Stream<List<int>> source) {
    final iterator = StreamIterator<List<int>>(source);
    return packedUploadAdd(
      upload: this,
      source: () async {
        if (!await iterator.moveNext()) return null;
        final chunk = iterator.current;
        return chunk is Uint8List ? chunk : Uint8List.fromList(chunk);
      },
    );
  }
}
