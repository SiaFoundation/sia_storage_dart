import 'dart:typed_data';

import 'package:sia_storage/sia_storage.dart';
import 'package:test/test.dart';

void main() {
  setUpAll(() async {
    await Sia.ready();
  });

  test('encodedSize matches manual erasure calculation', () {
    // 10 data + 20 parity → encoded = ceil(size / 10) * 30 (rough, depends on shard size)
    final encoded = encodedSize(size: BigInt.from(0), dataShards: 10, parityShards: 20);
    expect(encoded, BigInt.zero);
    final encoded1MB = encodedSize(
      size: BigInt.from(1 << 20),
      dataShards: 10,
      parityShards: 20,
    );
    // Encoded must be at least 3x the original (1 data + 2 parity per shard).
    expect(encoded1MB, greaterThan(BigInt.from(3 << 20)));
  });

  test('fresh PinnedObject has stable id and zero size', () {
    final a = PinnedObject();
    final b = PinnedObject();
    expect(a.size(), BigInt.zero);
    expect(a.encodedSize(), BigInt.zero);
    expect(a.slabs(), isEmpty);
    expect(a.metadata(), isEmpty);
    // Two empty objects with the same default contents share an id.
    expect(a.id(), b.id());
    expect(a.id(), matches(RegExp(r'^[0-9a-f]+$')));
  });

  test('update_metadata round-trips bytes', () {
    final obj = PinnedObject();
    final payload = Uint8List.fromList([1, 2, 3, 4, 5]);
    obj.updateMetadata(metadata: payload);
    expect(obj.metadata(), equals(payload));
  });

  test('seal then open round-trips with the same AppKey', () {
    final ak = AppKey(key: List<int>.filled(32, 9));
    final obj = PinnedObject();
    obj.updateMetadata(metadata: Uint8List.fromList([42, 43]));

    final sealed = obj.seal(appKey: ak);
    expect(sealed.id, obj.id());
    expect(sealed.encryptedMetadata, isNotEmpty);

    final opened = PinnedObject.open(appKey: ak, sealed: sealed);
    expect(opened.id(), obj.id());
    expect(opened.metadata(), equals([42, 43]));
  });

  test('seal cannot be opened with the wrong AppKey', () {
    final ak1 = AppKey(key: List<int>.filled(32, 1));
    final ak2 = AppKey(key: List<int>.filled(32, 2));
    final obj = PinnedObject();
    obj.updateMetadata(metadata: Uint8List.fromList([1]));
    final sealed = obj.seal(appKey: ak1);
    expect(
      () => PinnedObject.open(appKey: ak2, sealed: sealed),
      throwsA(anything),
    );
  });
}
