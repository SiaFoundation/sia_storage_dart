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

  test('AppKey save is 32 bytes and round-trips', () {
    final seed = List<int>.filled(32, 7);
    final ak = AppKey(key: seed);
    final saved = ak.save();
    expect(saved.length, 32);
    expect(saved, equals(seed));
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

  test('PreAuthorizedKey save is 32 bytes and round-trips', () {
    final generated = PreAuthorizedKey.generate();
    final seed = generated.save();
    expect(seed.length, 32);
    final imported = PreAuthorizedKey(seed: seed);
    expect(imported.publicKey(), generated.publicKey());
  });

  test('PreAuthorizedKey rejects non-32-byte seeds', () {
    expect(() => PreAuthorizedKey(seed: List.filled(31, 0)), throwsA(anything));
    expect(() => PreAuthorizedKey(seed: List.filled(33, 0)), throwsA(anything));
  });

  test('PreAuthorizedKey signs and verifies messages', () {
    final key = PreAuthorizedKey(seed: List<int>.filled(32, 2));
    final msg = [1, 2, 3];
    final sig = key.sign(message: msg);
    expect(sig.length, 64);
    expect(key.verifySignature(message: msg, signature: sig), isTrue);
    expect(key.verifySignature(message: [4], signature: sig), isFalse);
  });

  test('signRequest echoes the request and signs it', () {
    final key = PreAuthorizedKey(seed: List<int>.filled(32, 2));
    final expiration = DateTime.utc(2030, 1, 2, 3, 4, 5);
    final appId = '03' * 32;
    final signed = key.signRequest(
      request: PreAuthorizedKeyRequest(
        connectKey: 'connect-key',
        expiration: expiration,
        totalUses: 5,
        allowedAppId: appId,
      ),
    );
    expect(signed.publicKey, key.publicKey());
    expect(signed.connectKey, 'connect-key');
    expect(signed.expiration.toUtc(), expiration);
    expect(signed.totalUses, 5);
    expect(signed.allowedAppId, appId);
    expect(signed.signature, matches(RegExp(r'^[0-9a-fA-F]{128}$')));
  });

  test('signRequest allows a null app ID and rejects an invalid one', () {
    final key = PreAuthorizedKey(seed: List<int>.filled(32, 2));
    final request = PreAuthorizedKeyRequest(
      connectKey: 'connect-key',
      expiration: DateTime.utc(2030),
      totalUses: 1,
    );
    expect(key.signRequest(request: request).allowedAppId, isNull);
    expect(
      () => key.signRequest(
        request: PreAuthorizedKeyRequest(
          connectKey: 'connect-key',
          expiration: DateTime.utc(2030),
          totalUses: 1,
          allowedAppId: 'not-a-hash',
        ),
      ),
      throwsA(anything),
    );
  });
}
