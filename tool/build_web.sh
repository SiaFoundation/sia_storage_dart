#!/usr/bin/env bash
# Rebuilds the prebuilt wasm bundle shipped under web/pkg/.
#
# web/pkg/ is gitignored but ships in the pub.dev tarball (.pubignore),
# so this script must run before `dart pub publish`. It always starts from
# a clean slate — stale output is removed and the bindings are regenerated —
# so the generated Dart, Rust, JS, and wasm all come from the same codegen
# pass. (0.2.0 shipped a stale web/pkg whose frb content hash didn't match
# the committed Dart bindings, breaking every web consumer.)
#
# The threaded-wasm RUSTFLAGS below are required for frb's worker pool
# (SharedArrayBuffer-backed Memory). Browsers additionally require COOP/COEP
# headers to actually expose SharedArrayBuffer at runtime — see README.
#
# `--cfg=web_sys_unstable_apis` is needed because sia_storage's wasm path
# uses WebTransport (gated behind that cfg in web-sys).

set -euo pipefail

cd "$(dirname "$0")/.."

# Codegen must match the flutter_rust_bridge runtime pinned in pubspec.yaml
# and rust/Cargo.toml — a skewed codegen writes bindings with a different
# content hash and frb refuses to initialize at runtime.
FRB_VERSION=2.12.0

if [[ "$(flutter_rust_bridge_codegen --version 2>/dev/null || true)" != "flutter_rust_bridge_codegen $FRB_VERSION" ]]; then
  cargo install flutter_rust_bridge_codegen --version "$FRB_VERSION" --locked --force
fi

# wasm-pack needs a wasm-bindgen CLI matching the version in Cargo.lock. If
# none is installed it cargo-installs one itself *inside* the build's
# RUSTFLAGS environment, where the wasm link args break host build scripts —
# install it here with a clean environment instead.
WASM_BINDGEN_VERSION=$(grep -A1 '^name = "wasm-bindgen"$' rust/Cargo.lock | sed -n 's/^version = "\(.*\)"$/\1/p')

if [[ "$(wasm-bindgen --version 2>/dev/null || true)" != "wasm-bindgen $WASM_BINDGEN_VERSION" ]]; then
  cargo install wasm-bindgen-cli --version "$WASM_BINDGEN_VERSION" --locked --force
fi

rm -rf web/pkg

# frb formats the generated bindings with rustfmt, resolved against the
# toolchain pinned in rust/rust-toolchain.toml. When rustfmt is missing frb
# only warns and emits unformatted code that won't match the committed
# bindings — fail loudly here instead.
(cd rust && rustfmt --version >/dev/null)

flutter_rust_bridge_codegen generate

# Current rust-lld no longer implies `--import-memory` under
# `--shared-memory` and no longer auto-exports the heap/TLS bookkeeping
# symbols; wasm-bindgen's threading transform requires all of them.
# Shared memories can't grow past their declared max, and lld defaults max
# to the initial size — pin it at 1 GiB (16384 pages, what earlier
# toolchains defaulted to).
flutter_rust_bridge_codegen build-web --release --wasm-pack-rustflags \
  '--cfg getrandom_backend="wasm_js" -C target-feature=+atomics,+bulk-memory,+mutable-globals,+simd128 -C link-args=--shared-memory -C link-arg=--import-memory -C link-arg=--max-memory=1073741824 -C link-arg=--export=__heap_base -C link-arg=--export=__wasm_init_tls -C link-arg=--export=__tls_size -C link-arg=--export=__tls_align -C link-arg=--export=__tls_base --cfg=web_sys_unstable_apis'

# wasm-pack drops a `.gitignore: *` into pkg/ that would also exclude the
# bundle from `dart pub publish`. We need these files in the published
# tarball so consumers don't have to rebuild — drop the file.
rm -f web/pkg/.gitignore
