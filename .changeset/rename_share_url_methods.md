---
default: major
---

# Rename the share URL methods on `Sdk`

`Sdk.shareObject` and `Sdk.sharedObject` are now `Sdk.objectShareUrl` and `Sdk.objectFromShareUrl`. `shareObject` now attaches an object to a sharing key.

```dart
// before
final url = sdk.shareObject(object: obj, validUntil: expiry);
final shared = await sdk.sharedObject(sharedUrl: url);

// after
final url = sdk.objectShareUrl(object: obj, validUntil: expiry);
final shared = await sdk.objectFromShareUrl(shareUrl: url);
```
