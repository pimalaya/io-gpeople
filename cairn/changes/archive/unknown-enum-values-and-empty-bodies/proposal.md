---
cairn: change
id: unknown-enum-values-and-empty-bodies
status: landed
created: 2026-11-13
---

# An unknown enum value and an empty body no longer fail a response

## Why

Three findings of the family audit of 2026-11-13 (GPEOPLE-04, GPEOPLE-08 and the documentation half of GPEOPLE-07), re-checked against the code the same day.

**One unknown enum value failed a whole page.** Seven enums read from responses were closed: `GpeopleSourceType`, `GpeopleObjectType`, `GpeopleAgeRange`, `GpeopleContentType`, `GpeopleMiscKeywordType`, `GpeopleNicknameType` and `GpeopleContactGroupType`. The first sits on `metadata.source.type` of every field of every person, so the day the API adds a source type, each listing page holding one such person fails with `ParseResponse` ("unknown variant"), and the address book can no longer be listed or synchronised until a release. A new group type breaks `contactGroups.list` the same way.

**An empty success body failed every struct response.** A 2xx without a body was read as `null`, which only `GpeopleNoResponse` accepts: a photo deletion, a batch or a listing answered without a body completed with `ParseResponse` ("invalid type: null") although the call succeeded. The three sibling crates read `{}`.

**Four statements in the docs disagreed with the code.** src/lib.rs said types are `People`-prefixed (they are `Gpeople`-prefixed) and that an expired sync token surfaces as HTTP 410 (the predicate also takes the 400 carrying `EXPIRED_SYNC_TOKEN`); src/v1/rest/people/vcard.rs cited `project::full_date` and two spec files that are not in this repository; the example's header ran `--example std_people` for an example named `std_gpeople`.

## What

- An `Unknown` variant with `#[serde(other)]` on the seven enums, and on each field of these types a `skip_serializing_if` that leaves `Unknown` out of written bodies, as it leaves `None` out. The enums a caller alone fills are untouched.
- `GpeoplePerson::to_vcard` mints a keyword of unknown type without a TYPE. `GpeoplePerson::merge_unprojected` needed no change: an unknown value sits in a sub-field the vCard does not show, which the merge already keeps; a test now holds it there.
- An empty 2xx body reads as `{}`.
- The four statements corrected, plus the src/lib.rs sentence on empty bodies.

## Not in this change

- `is_etag_mismatch`, the other half of GPEOPLE-07: the predicate must match the status and reasons the live API returns for an etag mismatch, which have to be collected from a real exchange and recorded in cairn/spec/etags.md first.
- Keeping the unknown value itself: `Unknown` is a unit variant, so the string the server gave is not kept. A list rewritten by an update therefore sends an entry of unknown type without a type, and the server gives it its default. Keeping the string would need a variant holding it, which costs these enums `Copy`.
