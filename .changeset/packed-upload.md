---
default: minor
---

#### Add packed upload support

Adds `Sdk.uploadPacked`, which batches multiple small objects into shared slabs to avoid the per-object padding of a regular upload. Add objects to the returned session via `PackedUpload.add` (streaming bytes), then `finalize` to obtain the pinned objects. Per-shard progress is reported across the whole session.
