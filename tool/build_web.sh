#!/usr/bin/env bash
# Rebuilds the prebuilt wasm bundle shipped under web/pkg/.
#
# web/pkg/ is gitignored but ships in the pub.dev tarball (.pubignore),
# so this script must run before `dart pub publish`.
#
# The threaded-wasm RUSTFLAGS below are required for frb's worker pool
# (SharedArrayBuffer-backed Memory). Browsers additionally require COOP/COEP
# headers to actually expose SharedArrayBuffer at runtime — see README.
#
# `--cfg=web_sys_unstable_apis` is needed because sia_storage's wasm path
# uses WebTransport (gated behind that cfg in web-sys).

set -euo pipefail

cd "$(dirname "$0")/.."

flutter_rust_bridge_codegen build-web --release --wasm-pack-rustflags \
  '--cfg getrandom_backend="wasm_js" -C target-feature=+atomics,+bulk-memory,+mutable-globals,+simd128 -C link-args=--shared-memory --cfg=web_sys_unstable_apis'

# wasm-pack drops a `.gitignore: *` into pkg/ that would also exclude the
# bundle from `dart pub publish`. We need these files in the published
# tarball so consumers don't have to rebuild — drop the file.
rm -f web/pkg/.gitignore
