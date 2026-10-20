---
default: minor
---

# Add `Builder.reconnecting` and `Builder.matchesExistingAppKey`

After `waitForApproval`, `reconnecting` reports whether the approved connect key already has an account for the application. `matchesExistingAppKey` checks whether a recovery phrase derives an app key that is already registered. `register` now registers a new app key when the recovery phrase differs from the existing account's, instead of failing.
