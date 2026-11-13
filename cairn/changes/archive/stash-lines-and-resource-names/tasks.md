---
cairn: tasks
change: stash-lines-and-resource-names
---

# Tasks

- [x] Unit tests in src/v1/rest/people/vcard.rs failing against the previous code: a folded property (`stash_keeps_a_folded_property_whole`), a folded UID (`a_folded_uid_rides_the_stash_whole`), a stash stored with its folds (`a_stash_stored_with_its_folds_reads_back_whole`, `a_stored_blank_line_ends_the_line_before_it`), and the round trip of any property name, length, fold position and non-ASCII text over 2000 generated cards (`stash_round_trips_any_property`)
- [x] src/v1/rest/people/vcard.rs: `from_vcard` stashes logical lines, `stash_lines` joins the folded rows of an older stash
- [x] Unit tests failing against the previous code: a line break in a URL (`a_line_break_in_a_url_adds_no_vcard_line`) and in an IM client (`a_line_break_in_an_im_client_adds_no_vcard_line`)
- [x] src/v1/rest/people/vcard.rs: `uri_value` percent-encodes control characters for URL and IMPP
- [x] Integration tests in tests/coroutines.rs failing against the previous code: `a_resource_name_adds_no_query`, `a_resource_id_is_one_path_segment` (the ten constructors), `rejects_a_resource_name_outside_its_collection`
- [x] src/v1/send.rs: `resource_url`, used by the ten constructors taking a resource name
- [x] CHANGELOG under [Unreleased]
- [x] Fold the delta into cairn/spec/projection.md and cairn/spec/requests.md, log, land
