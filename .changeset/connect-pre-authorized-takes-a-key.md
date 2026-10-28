---
default: major
---

# `Builder.connectPreAuthorized` takes a `PreAuthorizedKey`

`preAuthorizedKey` is now a `PreAuthorizedKey` handle instead of a raw 32-byte seed, matching sia_storage's typed pre-authorized key. Pass a generated key directly, or restore a saved seed with `Sia.preAuthorizedKeyFromSeed`.

```dart
// before
final sdk = await builder.connectPreAuthorized(
  preAuthorizedKey: seed,
  mnemonic: mnemonic,
);

// after
final sdk = await builder.connectPreAuthorized(
  preAuthorizedKey: await Sia.preAuthorizedKeyFromSeed(seed),
  mnemonic: mnemonic,
);
```
