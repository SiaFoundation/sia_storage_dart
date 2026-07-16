---
default: minor
---

# Update `sia_storage` to 0.10.0

Also updates the Dart dependencies (`hooks` 2.x, `code_assets` 1.2) and bumps the pinned Rust toolchain to 1.96.1, required by sia_storage 0.10's MSRV — consumer hook builds fetch it automatically via `rust-toolchain.toml`.
