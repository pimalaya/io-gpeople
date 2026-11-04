---
cairn: spec
capability: etags
status: current
---

# Etags and `updateContact`

The People API guards writes with an etag: `people.updateContact` (and `batchUpdateContacts`, `updateContactPhoto`, `deleteContactPhoto`) require the person to carry an etag, and reject the write when it does not match the server's current one with `HTTP 400: Request person.etag is different than the current person.etag. Clear local cache and get the latest person.`

## The catch: not every read's etag is accepted

People etags are request-shaped, not just version-stamped: the same person, unchanged, is returned with a different etag by different reads. Observed behaviour is that `updateContact` only accepts an etag that came from `people.get`, `people.createContact` or a prior `people.updateContact` response; it rejects the etag returned by `people.connections.list` (and by the search endpoints). So "the etag from the latest read" is not sufficient: it must be the etag from a *get* or a prior *write*, not from a *list*.

This bites synchronising clients specifically. A card pulled from `connections.list` (a full or delta enumerate) carries a list etag. Editing that card and guarding the update with the etag the client stored from the enumerate is rejected. Edits chained from a client's own prior writes work, because each `updateContact` response returns a fresh, accepted etag; the problem is the first edit after a pull (which a two-sided conflict resolution always is).

## Recommended handling

On a `400` whose message reports the etag mismatch, re-read the person with `people.get` to obtain an accepted etag, then retry the update once:

1. `people.updateContact` with the etag on hand.
2. If it fails with the etag-mismatch `400`, `people.get` the same resource, take its etag.
3. `people.updateContact` again with that etag.

This is Google's own "clear local cache and get the latest person" guidance.

## Concurrency caveat

Step 2 drops the optimistic-concurrency guard for that write: the refetched etag is whatever the server holds right now, so retrying overwrites a concurrent remote change instead of detecting it. This is only safe when either last-writer-wins is acceptable, or a higher layer already guards concurrency independently. For example a sync engine whose enumerate re-lists (and re-flags as a conflict) any remotely-changed card before the push runs: there the retry only bridges the etag-representation gap, because a genuine remote change has already been caught upstream and the rejected card is never pushed. A client without such a guard should surface the mismatch to its caller rather than retry blindly.

### Requirement: No built-in etag retry

`GpeopleContactUpdate` SHALL send the etag the person carries and SHALL surface an etag mismatch to its caller as the API error, without re-reading the person or retrying. The coroutines are I/O-free and the refetch-and-retry above trades safety for convenience, so each consumer implements it where it can see its own concurrency guard. The std `client` feature could grow an opt-in helper for it later.

#### Scenario: Update guarded by a list etag

- GIVEN a person whose etag came from `people.connections.list`
- WHEN `GpeopleContactUpdate` sends it and the server answers the etag-mismatch `400`
- THEN the coroutine completes with `GpeopleSendError::Api`, and no `people.get` nor second update is sent
