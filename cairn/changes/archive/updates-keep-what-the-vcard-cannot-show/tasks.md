---
cairn: tasks
change: updates-keep-what-the-vcard-cannot-show
---

# Tasks

- [x] Unit test in src/v1/rest/people/vcard.rs failing against the previous code: `a_nameless_person_stashes_no_empty_fn`
- [x] `from_vcard` consumes an empty FN, `to_vcard` skips a stashed `FN:`
- [x] Unit tests failing against the previous flow (projection, mask, send): `an_update_keeps_the_entries_the_vcard_cannot_show`, `an_update_keeps_the_sub_fields_the_vcard_cannot_show`, `an_update_keeps_a_type_the_vcard_cannot_show`, `an_update_takes_the_type_the_vcard_changed`, `an_update_removes_what_the_vcard_removed`, `the_one_entry_edited_keeps_its_sub_fields`
- [x] `GpeoplePerson::merge_unprojected`, with `merge_list`, `merge_one` and `shown`
- [x] Unit tests of the merge alone: `two_entries_edited_at_once_are_new_entries`, `an_update_keeps_the_client_data_of_others`, `an_unedited_vcard_erases_nothing_under_the_widest_mask` (every managed field masked, the merge applied twice), `a_field_outside_the_mask_is_left_alone`
- [x] `GpeoplePersonVcard`, returned by `from_vcard`, with `from_vcard_names_the_properties_it_drops`
- [x] tests/people.rs: the live update goes through `merge_unprojected` (not run here)
- [x] CHANGELOG under [Unreleased]
- [x] Fold the delta into cairn/spec/projection.md, log, land
- [ ] GPEOPLE-05, the wider read projection: not done, waits for a decision on the projection spec
