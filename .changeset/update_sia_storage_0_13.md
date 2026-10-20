---
default: minor
---

# Update `sia_storage` to 0.13.1

Uploads and downloads pick up the SDK's transfer improvements: per-RPC idle timeouts in place of fixed deadlines, a shared connection pool, more parallel WebTransport transfers, CBOR indexer responses, and automatic reuploads of slabs the indexer rejects as too old.
