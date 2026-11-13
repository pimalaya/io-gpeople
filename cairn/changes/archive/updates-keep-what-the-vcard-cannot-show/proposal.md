---
cairn: change
id: updates-keep-what-the-vcard-cannot-show
status: landed
created: 2026-11-13
---

# Updates keep what the vCard cannot show

## Why

Three defects, each re-checked against the code on 2026-11-13 with a test failing before the fix: two from the family audit of that day (GPEOPLE-02, GPEOPLE-06) and one seen while fixing the stash earlier the same day.

**An update erased what the vCard does not show.** `people.updateContact` replaces every masked field whole. The flow the crate documents builds the person from the edited vCard, takes `changed_fields` against the projection of the server copy, and sends that. But a managed field holds more than its vCard: `to_vcard` shows the spouse and child relations only, the first organization and occupation only, the home, work and cell types only, and no sub-field without a vCard slot. With a server contact holding a spouse, a mother and an assistant, renaming the spouse sent `relations: [spouse]` under the mask `relations`: mother and assistant were deleted. Changing TITLE deleted the second organization and the first one's job description and dates; adding an email cleared the custom type and display name of every other email; touching a phone reset `workFax`. Three consumers share the flow and each merged back one thing only, the `clientData` entries of other clients.

**A line left out of the stash left no trace.** A logical line over 8 KiB (an inline PHOTO, a KEY) is not stashed, by design, and `from_vcard` returned `Ok(person)` without saying so: the caller could not tell that the server copy would lack the property, nor keep it out of a later comparison.

**A person without a name grew a stash.** `to_vcard` writes the mandatory FN empty for a person without a name; `from_vcard` did not consume an empty FN, so it was stashed as `FN:`, and once stored the stash gained one more `FN:` line at each round trip.

## What

- `GpeoplePerson::merge_unprojected(&mut self, current, fields)`: for each managed field of the mask, pairs each vCard entry with the server entry it shows and keeps what that entry holds beyond the vCard, keeps the server entries the vCard hides, and keeps the `clientData` entries of other clients. What a server entry shows is found by projecting it alone through `to_vcard` and `from_vcard`, so the merge cannot drift from the projection.
- `GpeoplePerson::from_vcard` returns a `GpeoplePersonVcard { person, dropped }`, `dropped` naming the properties too long for the stash. A breaking change, chosen so that no caller can ignore the list by accident.
- `from_vcard` consumes an empty FN, and `to_vcard` skips a stashed `FN:`.

## Choices made, the maintainer's to confirm

The rule followed is the conservative one: never erase what the vCard could not express.

- One entry replaced by another is read as an edit when it is the only unpaired entry on each side, so a new email inherits the custom type and display name of the one it replaced.
- A sub-field the vCard does not show survives a change of the value it qualifies: a birthday's free text survives a new date, an organization's dates a new name.
- An entry the vCard showed and no longer holds is removed with its hidden sub-fields.
- The vCard's entries come first, the hidden server entries after them, so a hidden entry that came first no longer does.
- Server entries are sent back as read, metadata and output-only fields included, the way a read, modify, write sequence does.
- `clientData` is merged here too, which makes the merge each consumer does by hand redundant.

## Not in this change

- GPEOPLE-05, widening the read projection (a birthday without a year, ANNIVERSARY, GENDER, PHOTO as a URI, every RELATED type, repeated ORG and ROLE). It contradicts requirements written in cardamum's projection spec, which this code was ported from, and each mapping needs a decision. The merge removes the data loss at the server meanwhile; the export stays as narrow as before.
- Routing a dropped PHOTO to `people.updateContactPhoto`.
- A cap on the stash as a whole.
