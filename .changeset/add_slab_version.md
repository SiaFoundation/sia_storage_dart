---
default: major
---

# Add `version` to `Slab`

`Slab` has a new required `version` field, matching sia_storage's slab versions. Code that constructs `Slab` or `SealedObject` values by hand must pass it.
