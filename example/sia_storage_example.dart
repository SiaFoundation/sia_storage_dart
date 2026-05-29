/// End-to-end example: authorize an app, upload a stream of random bytes,
/// pin the object, then download and verify it — reporting throughput.
///
/// Run from the package root:
///
/// ```sh
/// dart run example/sia_storage_example.dart --size 125829120
/// ```
library;

import 'dart:io';
import 'dart:math';
import 'dart:typed_data';

import 'package:sia_storage/sia_storage.dart';

final appMeta = AppMetadata(
  id: _hexToBytes(
    '5c0b1af28e6ac76395b2087ea987297b9c496f90d2ab3e3d3d07980ae4c43633',
  ),
  name: 'My Example App',
  description: 'My Example App Description',
  serviceUrl: 'https://myexampleapp.com',
);

Future<void> main(List<String> args) async {
  // Size of the data to upload and download in bytes (default: 120 MiB).
  final size = _parseSize(args);

  // Authorize the app to access the user's storage.
  final builder = await Sia.builder(
    indexerUrl: 'https://sia.storage',
    appMeta: appMeta,
  );

  await builder.requestConnection();
  stdout.writeln(
    'Visit the following URL to authorize the application: '
    '${builder.responseUrl()}',
  );

  await builder.waitForApproval();
  stdout.writeln('Connection approved!');

  stdout.write('Enter recovery phrase: ');
  final phrase = stdin.readLineSync()?.trim();
  if (phrase == null || phrase.isEmpty) {
    stderr.writeln('no recovery phrase provided');
    exit(1);
  }

  final sdk = await builder.register(mnemonic: phrase);
  stdout.writeln('App registered successfully!');

  final seed = Random().nextInt(0x100000000);

  // Upload the data to the network.
  stdout.writeln('Uploading random data...');
  final uploadTimer = Stopwatch()..start();
  final upload = sdk.upload(
    object: PinnedObject(),
    source: _randomData(seed, size),
  );
  final obj = await upload.result;
  uploadTimer.stop();

  // Pin the object to ensure it remains available on the network.
  await sdk.pinObject(object: obj);
  stdout.writeln('Object pinned successfully!');

  // Download the object back from the network, verifying it matches the
  // bytes we uploaded as it streams in.
  stdout.writeln('Downloading object...');
  final verifier = _SeededBytes(seed);
  final downloadTimer = Stopwatch()..start();
  var verified = 0;
  Duration? ttfb;
  var gapMax = Duration.zero;
  var lastChunk = Duration.zero;
  await for (final chunk in sdk.download(object: obj).data) {
    final elapsed = downloadTimer.elapsed;
    ttfb ??= elapsed;
    final gap = elapsed - lastChunk;
    if (gap > gapMax) gapMax = gap;
    lastChunk = elapsed;

    final expected = Uint8List(chunk.length);
    verifier.fill(expected);
    for (var i = 0; i < chunk.length; i++) {
      if (chunk[i] != expected[i]) {
        throw StateError('data mismatch at byte ${verified + i}');
      }
    }
    verified += chunk.length;
  }
  downloadTimer.stop();
  if (verified != size) {
    throw StateError('expected $size bytes, got $verified');
  }

  final uploadDuration = uploadTimer.elapsed;
  final downloadDuration = downloadTimer.elapsed;
  final objSize = obj.size().toInt();
  final encodedSize = obj.encodedSize().toInt();

  stdout.writeln(
    'Object uploaded ID: ${obj.id()}'
    '\tSize: ${_formatBytes(objSize)}'
    '\tEncoded: ${_formatBytes(encodedSize)}'
    '\tElapsed: $uploadDuration'
    '\tThroughput: ${_formatBitrate(objSize, uploadDuration)}'
    '\tEncoded Throughput: ${_formatBitrate(encodedSize, uploadDuration)}',
  );
  stdout.writeln(
    'Object downloaded ID: ${obj.id()}'
    '\tSize: ${_formatBytes(objSize)}'
    '\tEncoded: ${_formatBytes(encodedSize)}'
    '\tElapsed: $downloadDuration'
    '\tTTFB: ${ttfb ?? Duration.zero}'
    '\tThroughput: ${_formatBitrate(objSize, downloadDuration)}'
    '\tMax Write Latency: $gapMax',
  );

  // Tear down the native runtime so the VM can exit on its own: this closes
  // the frb channel ports that otherwise keep the event loop alive.
  Sia.dispose();
}

/// A deterministic stream of random bytes from a seed, yielded in chunks.
Stream<List<int>> _randomData(
  int seed,
  int size, {
  int chunk = 64 * 1024,
}) async* {
  final bytes = _SeededBytes(seed);
  var remaining = size;
  while (remaining > 0) {
    final buf = Uint8List(min(chunk, remaining));
    bytes.fill(buf);
    yield buf;
    remaining -= buf.length;
  }
}

/// Produces a deterministic byte sequence from a seed. Bytes are drawn from a
/// rolling 32-bit word, so the sequence is independent of how it's chunked —
/// the uploader and the download verifier stay in lockstep regardless of
/// where chunk boundaries fall.
class _SeededBytes {
  final Random _rng;
  int _word = 0;
  int _wordBytesLeft = 0;

  _SeededBytes(int seed) : _rng = Random(seed);

  int _nextByte() {
    if (_wordBytesLeft == 0) {
      _word = _rng.nextInt(0x100000000);
      _wordBytesLeft = 4;
    }
    final b = _word & 0xff;
    _word >>= 8;
    _wordBytesLeft--;
    return b;
  }

  void fill(Uint8List out) {
    for (var i = 0; i < out.length; i++) {
      out[i] = _nextByte();
    }
  }
}

int _parseSize(List<String> args) {
  const defaultSize = 120 * 1024 * 1024;
  for (var i = 0; i < args.length; i++) {
    final arg = args[i];
    if (arg == '-s' || arg == '--size') {
      if (i + 1 >= args.length) {
        stderr.writeln('missing value for $arg');
        exit(2);
      }
      return int.parse(args[i + 1]);
    }
    if (arg.startsWith('--size=')) {
      return int.parse(arg.substring('--size='.length));
    }
  }
  return defaultSize;
}

Uint8List _hexToBytes(String hex) {
  final out = Uint8List(hex.length ~/ 2);
  for (var i = 0; i < out.length; i++) {
    out[i] = int.parse(hex.substring(i * 2, i * 2 + 2), radix: 16);
  }
  return out;
}

String _formatBytes(int bytes) {
  const units = ['B', 'KiB', 'MiB', 'GiB', 'TiB'];
  var value = bytes.toDouble();
  for (final unit in units) {
    if (value < 1024 || unit == units.last) {
      return '${value.toStringAsFixed(2)} $unit';
    }
    value /= 1024;
  }
  throw StateError('unreachable');
}

String _formatBitrate(int bytes, Duration duration) {
  const units = ['bps', 'Kbps', 'Mbps', 'Gbps', 'Tbps'];
  var value = (bytes * 8) / (duration.inMicroseconds / 1e6);
  for (final unit in units) {
    if (value < 1000 || unit == units.last) {
      return '${value.toStringAsFixed(2)} $unit';
    }
    value /= 1000;
  }
  throw StateError('unreachable');
}
