import 'package:sia_storage/sia_storage.dart';
import 'package:test/test.dart';

void main() {
  setUpAll(() async {
    await Sia.ready();
  });

  test('recovery phrase generates 12 words and validates', () async {
    final phrase = await Sia.generateRecoveryPhrase();
    expect(phrase.split(' ').length, 12);
    await Sia.validateRecoveryPhrase(phrase);
  });

  test('invalid recovery phrase throws', () async {
    await expectLater(
      Sia.validateRecoveryPhrase('not a real phrase'),
      throwsA(anything),
    );
  });

  test('AppKey export is 32 bytes and round-trips', () {
    final seed = List<int>.filled(32, 7);
    final ak = AppKey(key: seed);
    final exported = ak.export_();
    expect(exported.length, 32);
    expect(exported, equals(seed));
  });

  test('AppKey rejects non-32-byte input', () {
    expect(() => AppKey(key: List.filled(31, 0)), throwsA(anything));
    expect(() => AppKey(key: List.filled(33, 0)), throwsA(anything));
  });

  test('sign produces a 64-byte signature that verifies', () {
    final ak = AppKey(key: List<int>.filled(32, 1));
    final msg = [1, 2, 3, 4, 5];
    final sig = ak.sign(message: msg);
    expect(sig.length, 64);
    expect(ak.verifySignature(message: msg, signature: sig), isTrue);
    expect(ak.verifySignature(message: [9, 9], signature: sig), isFalse);
  });

  test('public key is hex and stable across instances with the same seed', () {
    final seed = List<int>.filled(32, 42);
    final a = AppKey(key: seed);
    final b = AppKey(key: seed);
    expect(a.publicKey(), b.publicKey());
    expect(a.publicKey(), matches(RegExp(r'^[0-9a-fA-F:]+$')));
  });
}
