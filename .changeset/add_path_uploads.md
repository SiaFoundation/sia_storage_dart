---
default: minor
---

# Add path-based uploads

`Sdk.uploadPath` and `PackedUpload.addPath` upload a file by path. The file is read natively instead of being streamed through Dart chunk by chunk. They aren't available on web, where both throw.

```dart
final upload = sdk.uploadPath(object: PinnedObject(), path: '/path/to/file');
final object = await upload.result;
```
