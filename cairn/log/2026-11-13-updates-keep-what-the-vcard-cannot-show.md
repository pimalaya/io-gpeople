---
cairn: log
change: updates-keep-what-the-vcard-cannot-show
landed: 2026-11-13
---

# Updates keep what the vCard cannot show

Two defects found by the family audit of 2026-11-13 (GPEOPLE-02, GPEOPLE-06) and one seen while fixing the stash the same day. A third audit finding of the same package, GPEOPLE-05, is not done.

**Merge.** `people.updateContact` replaces each masked field whole, and the documented flow (person from the edited vCard, mask from `changed_fields`, send) put in the body only what the vCard shows: the spouse and child relations, the first organization and occupation, the home, work and cell types, no sub-field without a vCard slot. Renaming a spouse deleted the mother and the assistant; changing TITLE deleted the second organization and the first one's job description and dates; adding an email cleared the custom type and display name of the others. `GpeoplePerson::merge_unprojected(current, fields)` now rebuilds each masked field: a vCard entry is paired with the server entry it shows (same projection, else same key, else the only one left on each side) and becomes that entry with the sub-fields the vCard changed; a server entry the vCard hides is kept after them; a shown entry the vCard no longer holds is removed; names, birthdays and biographies keep one entry; the `clientData` entries of other clients are kept. What a server entry shows is computed by projecting it alone through `to_vcard` and `from_vcard`, so the merge follows the projection without a second description of it. The consumers call it between `changed_fields` and the update, and can drop their own `clientData` merge.

**Dropped lines.** `from_vcard` returns a `GpeoplePersonVcard { person, dropped }`: `dropped` names the properties whose logical line is over 8 KiB and that the stash therefore lacks. A breaking change, every caller reading `.person`.

**Empty FN.** A person without a name projects the mandatory FN empty, which `from_vcard` stashed as `FN:`; once stored, each round trip added one. An empty FN is now consumed, and `to_vcard` skips a stashed `FN:`, which the stashes written before still hold.

Choices made under the rule "never erase what the vCard could not express", for the maintainer to confirm: one entry replaced by another, alone on each side, is an edit and inherits the hidden sub-fields; a hidden sub-field survives a change of the value it qualifies (a birthday's free text, an organization's dates); the vCard's entries come first and the hidden ones after; server entries go back as read, metadata and output-only fields included.

Capabilities moved: projection (six requirements added).

Verified: in src/v1/rest/people/vcard.rs, `a_nameless_person_stashes_no_empty_fn` and the six flow tests (`an_update_keeps_the_entries_the_vcard_cannot_show`, `an_update_keeps_the_sub_fields_the_vcard_cannot_show`, `an_update_keeps_a_type_the_vcard_cannot_show`, `an_update_takes_the_type_the_vcard_changed`, `an_update_removes_what_the_vcard_removed`, `the_one_entry_edited_keeps_its_sub_fields`) fail against the previous code and pass; `an_unedited_vcard_erases_nothing_under_the_widest_mask` holds every managed field of a rich person through the merge under the widest mask, twice. `cargo test --all-features` passed (30 unit, 28 integration, the 2 live tests ignored and not run), `cargo clippy --all-features --all-targets -- -D warnings` clean, `cargo build --no-default-features` builds.

Left open:

- Not checked against the live API: that People accepts entries sent back with their metadata and output-only fields, and ignores the ones another source owns. The live test in tests/people.rs now sends its update through the merge, and was not run.
- GPEOPLE-05: the read projection stays as narrow as before (no birthday without a year, anniversary, gender, photo URI, other relation types, further organizations and occupations). It needs the projection spec amended first, which lives in cardamum and says events and photos stay out of the projection.
- An organization or occupation after the first becomes the first, and shows in the vCard, once the first is removed.
- A dropped PHOTO is named, not routed to `people.updateContactPhoto`.
