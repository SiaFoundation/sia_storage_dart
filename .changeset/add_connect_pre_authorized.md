---
default: minor
---

# Add `Builder.connectPreAuthorized`

Connects with a pre-authorized key that the indexer operator provisions out of band. It runs the connect, approval, and registration steps in one call and returns a ready `Sdk`, without the interactive approval flow.
