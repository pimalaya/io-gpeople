---
cairn: log
change: stash-lines-and-resource-names
landed: 2026-11-13
---

# The stash holds logical lines, a projected URI adds no line, a resource name stays in its collection

Three defects found by the family audit of 2026-11-13 (GPEOPLE-01, GPEOPLE-09, GPEOPLE-03).

**Stash.** `GpeoplePerson::from_vcard` stashed each unmanaged line through its `Display`, which replays the folds the line arrived with, and the reader split the stash on LF: a folded property came back as its first piece plus a CR, the rest dropped by the `push_raw` that could not tokenise it. A 120-character UID read back as 71 characters. The stash now holds the logical line, built from the line's name, parameters and raw value, and `stash_lines` joins a row starting with a space or a tab to the row before it (minus that row's trailing CR), which reads the stashes already stored at Google whole: their pieces were all there, the loss was on the way back. A blank row ends the line before it, so an indented line after a dropped blank line stays its own, as the vCard tokeniser has it. The stash was fixed in place: whether this crate or vcard-rs should own it is a pending decision.

**URI values.** URL and IMPP are URI values, which vcard-rs 0.6.0 writes byte for byte. `uri_value` percent-encodes their control characters, so a line break in a People url, IM username or IM protocol stays inside its property. Whether People stores a line break in these fields was not checked against the live API.

**Resource names.** Ten constructors (`GpeoplePersonGet`, `GpeopleContactUpdate`, `GpeopleContactDelete`, `GpeopleContactPhotoUpdate`, `GpeopleContactPhotoDelete`, `GpeopleContactGroupGet`, `GpeopleContactGroupUpdate`, `GpeopleContactGroupDelete`, `GpeopleContactGroupMembersModify`, `GpeopleOtherContactCopy`) joined the resource name onto the API base as a relative reference. The crate-private `resource_url` in src/v1/send.rs now takes the collection the method serves, refuses a name that is not `<collection>/<id>` (id empty, `.` or `..` included) with `GpeopleSendError::InvalidRequest`, and pushes the id as one percent-encoded path segment before the method suffix. No signature changed; a name that used to build a request to the wrong place, a bare id for one, now fails at construction.

Capabilities moved: projection (new spec file, three requirements), requests (new spec file, two requirements).

Verified: in src/v1/rest/people/vcard.rs, `stash_keeps_a_folded_property_whole`, `a_folded_uid_rides_the_stash_whole`, `a_stash_stored_with_its_folds_reads_back_whole`, `a_stored_blank_line_ends_the_line_before_it`, `stash_round_trips_any_property` (2000 generated cards: any name, parameters, length, fold position, CRLF or LF, non-ASCII), `a_line_break_in_a_url_adds_no_vcard_line` and `a_line_break_in_an_im_client_adds_no_vcard_line` fail against the previous code and pass; in tests/coroutines.rs, `a_resource_name_adds_no_query`, `a_resource_id_is_one_path_segment` and `rejects_a_resource_name_outside_its_collection` likewise. `cargo test --all-features` passed (18 unit, 28 integration, the 2 live tests ignored and not run), `cargo clippy --all-features --all-targets -- -D warnings` clean, `cargo build --no-default-features` builds.

Left open:

- A stash stored before this change whose line carried quoted-printable soft breaks is not joined back, only folds are.
- A stashed line now keeps the trailing spaces of its value, which the previous code trimmed. Whether People trims a `clientData` value was not checked: if it does, a stash ending on such a line would differ on every read.
- vcard-rs 0.6.0 escapes LF in a text value but writes a bare CR as is: a parser taking a lone CR for a line break would still see one in a text field.
- A person without a name projects an empty `FN`, which `from_vcard` stashes: once stored, the stash gains one `FN:` line per round trip.
- An id holding `:` is sent as is, the colon being legal in a path segment: no method takes one today.
