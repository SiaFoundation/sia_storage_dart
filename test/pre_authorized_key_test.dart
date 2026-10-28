import 'package:sia_storage/sia_storage.dart';
import 'package:test/test.dart';

void main() {
  setUpAll(() async {
    await Sia.ready();
  });

  test('generated keys are random and round-trip through their seed', () async {
    final key = await Sia.preAuthorizedKey();
    final seed = key.save();
    expect(seed.length, 32);

    final imported = await Sia.preAuthorizedKeyFromSeed(seed);
    expect(imported.publicKey(), key.publicKey());

    final other = await Sia.preAuthorizedKey();
    expect(other.publicKey(), isNot(key.publicKey()));
  });

  test('rejects seeds that are not 32 bytes', () {
    expect(
      () => PreAuthorizedKey.fromSeed(seed: List.filled(31, 0)),
      throwsA(anything),
    );
    expect(
      () => PreAuthorizedKey.fromSeed(seed: List.filled(33, 0)),
      throwsA(anything),
    );
  });

  test('sign produces a 64-byte signature that verifies', () {
    final key = PreAuthorizedKey.fromSeed(seed: List<int>.filled(32, 2));
    final msg = [1, 2, 3, 4, 5];
    final sig = key.sign(message: msg);
    expect(sig.length, 64);
    expect(key.verifySignature(message: msg, signature: sig), isTrue);
    expect(key.verifySignature(message: [9, 9], signature: sig), isFalse);
  });

  test('signRequest returns the signed creation request', () {
    final key = PreAuthorizedKey.fromSeed(seed: List<int>.filled(32, 2));
    final expiration = DateTime.utc(1970, 1, 1, 0, 3, 3, 123);
    final signed = key.signRequest(
      request: PreAuthorizedKeyRequest(
        connectKey: 'connect-key',
        expiration: expiration,
        totalUses: 5,
        allowedAppId: '03' * 32,
      ),
    );

    expect(signed.publicKey, key.publicKey());
    expect(signed.connectKey, 'connect-key');
    expect(signed.expiration.toUtc(), expiration);
    expect(signed.totalUses, 5);
    expect(signed.allowedAppId, '03' * 32);
    expect(signed.signature, matches(RegExp(r'^[0-9a-fA-F]{128}$')));
  });

  test('signRequest allows any application when allowedAppId is null', () {
    final key = PreAuthorizedKey.fromSeed(seed: List<int>.filled(32, 2));
    final signed = key.signRequest(
      request: PreAuthorizedKeyRequest(
        connectKey: 'connect-key',
        expiration: DateTime.utc(2030),
        totalUses: 1,
      ),
    );
    expect(signed.allowedAppId, isNull);
  });

  test('signRequest rejects a malformed application ID', () {
    final key = PreAuthorizedKey.fromSeed(seed: List<int>.filled(32, 2));
    expect(
      () => key.signRequest(
        request: PreAuthorizedKeyRequest(
          connectKey: 'connect-key',
          expiration: DateTime.utc(2030),
          totalUses: 1,
          allowedAppId: 'not-an-id',
        ),
      ),
      throwsA(anything),
    );
  });
}
