---
default: minor
---

# Add sharing keys for read-only access to objects

A sharing key grants read-only access to a chosen set of objects. The recipient doesn't need an account, an app registration, or an approval flow. Owners create a key with `Sdk.createSharingKey`, attach objects with `Sdk.shareObject`, and hand out the key's exported seed. They manage keys with `sharingKeys`, `sharingKey`, `sharedObjects`, `unshareObject`, and `revokeSharingKey`.

Recipients connect with `Sia.sharedSdk` and get a `SharedSdk` with `stats`, `object`, `objects`, `objectSummaries`, `hosts`, and `download`. Reads are paid from the owner's account.

```dart
final key = await sdk.createSharingKey(description: 'photos', expiresAt: null);
await sdk.shareObject(key: key, object: obj);

final shared = await Sia.sharedSdk(indexerUrl: url, sharingKey: key.export());
final objects = await shared.objects(offset: null, limit: null);
```
