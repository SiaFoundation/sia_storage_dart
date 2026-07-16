---
default: patch
---

# Restore resolvability with Flutter stable

`hooks` is now constrained to `>=2.0.2 <2.1.0`: hooks 2.1.0 requires `meta ^1.19.0`, which conflicts with the `meta 1.17.0` pinned by Flutter stable (3.41), making 0.3.0 unresolvable in Flutter apps. CI now resolves the package from a scratch Flutter app against the stable channel to catch pin conflicts before release.
