---
default: patch
---

#### Update sia_storage to v0.13.3

Picks up fixes to the browser WebTransport host client. Bidirectional streams
whose open timed out or was cancelled are now closed instead of piling up on a
host's session until it stopped accepting new streams, which stalled reads from
that host. Addresses that refuse a WebTransport handshake are no longer redialed
for five minutes, and a host whose addresses have all refused now fails
immediately so a download moves on to other hosts, avoiding the multi-second
connection delays Chrome applies after repeated handshake failures.

There are no public Dart API changes.
