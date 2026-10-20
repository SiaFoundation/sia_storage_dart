---
default: major
---

# Take a cutoff on `pruneSlabs`

`Sdk.pruneSlabs` takes an optional `before` timestamp. Only slabs pinned before it are pruned. Passing `null` leaves the cutoff to the indexer, which spares recently pinned slabs so an upload in progress is not pruned.

```dart
// before
await sdk.pruneSlabs();

// after
await sdk.pruneSlabs(before: null);
```
