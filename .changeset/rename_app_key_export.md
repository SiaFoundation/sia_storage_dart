---
default: major
---

# Rename `AppKey.export_` to `AppKey.save`

The generated `export_()` name came from `export` being reserved in Dart. `AppKey.save()` returns the same 32-byte buffer. Pass it to `Sia.appKey` to restore the key.

```dart
// before
final bytes = appKey.export_();

// after
final bytes = appKey.save();
```
