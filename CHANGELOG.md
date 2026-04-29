# Changelog

## 0.1.0

Initial release.

- Dart bindings for the [Sia Storage SDK](https://crates.io/crates/sia_storage) (`sia_storage` 0.8).
- Native targets compiled via Dart build hooks ([`native_toolchain_rust`](https://pub.dev/packages/native_toolchain_rust)) — no per-platform plugin scaffolding.
- Web/wasm support via prebuilt artifacts shipped under `web/pkg/`.
- Pure-Dart compatible — usable from Flutter, Dart server, or Dart CLI.
- API surface mirrors the Sia Storage NAPI bindings (`AppKey`, `Builder`, `Sdk`, `PinnedObject`, recovery phrases) with idiomatic Dart adaptations: streaming uploads via `Stream<List<int>>`, downloads as `Stream<Uint8List>`, and per-shard progress as `Stream<ShardProgress>`.
- Lazy native runtime initialization through the `Sia` facade — no manual `init()` for typical flows.
