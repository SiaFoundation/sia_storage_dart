---
default: major
---

#### Rename `uploadStream`/`downloadStream` to `upload`/`download`

The stream-based helpers on `Sdk` are renamed to match the underlying SDK: `uploadStream` is now `upload` and `downloadStream` is now `download`. The raw callback/`StreamSink` bindings they wrapped are no longer exposed on the public `Sdk` type.

Migrate by dropping the `Stream` suffix:

```dart
// before
final upload = sdk.uploadStream(object: obj, source: file.openRead());
final download = sdk.downloadStream(object: obj);

// after
final upload = sdk.upload(object: obj, source: file.openRead());
final download = sdk.download(object: obj);
```
