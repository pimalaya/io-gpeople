---
cairn: change
id: stash-lines-and-resource-names
status: landed
created: 2026-11-13
---

# The stash holds logical lines, a projected URI adds no line, a resource name stays in its collection

## Why

Three defects found by the family audit of 2026-11-13 (GPEOPLE-01, GPEOPLE-09, GPEOPLE-03) and re-checked against the code the same day, each with a test failing before the fix.

**The stash truncated every folded property.** `GpeoplePerson::from_vcard` stashed an unmanaged line through its `Display`, which replays the folds the line arrived with, joined the lines with LF, and the reader split on LF. A property folded on the way in (anything past 75 octets, from any CardDAV server or from vcard-rs itself) was therefore read back as its first piece plus a CR, the other pieces failing to tokenise and being dropped without a word. A 120-character UID came back as its first 71 characters, so `stashed_uid` handed a sync engine a wrong identity. The pieces themselves were stored whole at People: the loss was on the way back.

**A URL or IM value could add vCard lines.** `to_vcard` writes URL and IMPP as URI values, which vcard-rs emits byte for byte, and only trimmed the ends of the People strings. A `urls[].value`, `imClients[].username` or `imClients[].protocol` holding a line break produced extra properties, which `from_vcard` then read as real fields. Whether People stores a line break in these fields was not verified: the encoding is the second line of defence either way.

**A resource name was a relative URL reference.** Ten constructors joined the caller's resource name onto the API base, checking only that it was not empty. `contactGroups/abc?deleteContacts=true` given to `GpeopleContactGroupDelete` with `delete_contacts` false deleted the contacts with the group; a `#` moved the method suffix into the fragment, which is never sent; `..`, `//host/x` and an absolute URL left the collection or changed the Host header; a bare id made `c1:deleteContact` parse as a URL of scheme `c1`.

## What

- src/v1/rest/people/vcard.rs: the stash holds each unmanaged property as its logical line (name, parameters, raw value), built from the line's parts instead of its `Display`. The reader joins a row starting with a space or a tab to the row before it, minus that row's trailing CR, so a stash stored with its folds reads back whole; a blank row ends the line before it, as it does in a card. The stash stays where it is: who owns it (this crate or vcard-rs) is a pending decision.
- Same file: control characters of a URL or IMPP value are percent-encoded before the value is pushed.
- src/v1/send.rs: `resource_url(collection, resource_name, tail)`, crate-private, accepts only `<collection>/<id>` with an id that is neither empty, `.` nor `..`, pushes the id as one percent-encoded path segment and appends the method suffix. The ten constructors use it, each with the collection its method serves.

## Not in this change

- A stash stored before this change whose line carried quoted-printable soft breaks (`=` at the end of a row) is not joined back: only folds are. New stashes hold such a line whole.
- The 8 KiB cap on a stashed line (GPEOPLE-06) is untouched; it now measures the logical line.
- The path-segment helper the family plans in io-http: `resource_url` is local until it exists.
