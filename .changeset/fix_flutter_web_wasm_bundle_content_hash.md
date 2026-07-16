---
default: patch
---

# Fix Flutter Web bundle shipping with a mismatched content hash

0.2.0 shipped a stale `web/pkg` wasm whose `frb_get_rust_content_hash` did not match the Dart bindings, so flutter_rust_bridge refused to initialize in browsers ("content hash doesn't match Rust"). `tool/build_web.sh` now rebuilds the bundle from a clean slate — stale output is removed and the bindings are regenerated with `flutter_rust_bridge_codegen` pinned to the runtime version — and the publish workflow builds from the clean tag checkout and verifies the regenerated bindings match the release commit before publishing.
