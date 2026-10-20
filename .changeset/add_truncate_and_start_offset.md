---
default: minor
---

# Add `PinnedObject.truncate` and `UploadOptions.startOffset`

`truncate` returns a copy of the object cut to the given length. `startOffset` makes an upload overwrite the object from that byte offset instead of appending, so only the slabs covering the rewritten range are uploaded again. Re-pin the object after either change.
