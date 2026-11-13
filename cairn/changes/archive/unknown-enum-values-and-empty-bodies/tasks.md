---
cairn: tasks
change: unknown-enum-values-and-empty-bodies
---

# Tasks

- [x] Integration tests in tests/coroutines.rs failing against the previous code: `an_unknown_enum_value_does_not_fail_the_page`, `an_unknown_group_type_does_not_fail_the_listing`, `an_unknown_enum_value_is_never_written_back`, `an_empty_success_body_reads_as_an_empty_object`
- [x] `Unknown` with `#[serde(other)]` on the seven enums read from responses, left out of written bodies
- [x] Unit test in src/v1/rest/people/vcard.rs failing before the keyword fix: `an_unknown_enum_value_stays_on_its_entry_and_out_of_the_vcard` (the vCard, then the merge under the widest mask and on two edits)
- [x] `to_vcard` mints a keyword of unknown type without a TYPE
- [x] src/v1/send.rs: an empty 2xx body reads as `{}`
- [x] Docs: src/lib.rs (prefix, expired sync token, empty bodies), src/v1/rest/people/vcard.rs (three citations), examples/std_gpeople.rs (example name)
- [x] CHANGELOG under [Unreleased]
- [x] Fold the delta into cairn/spec/responses.md and cairn/spec/projection.md, log, land
- [ ] `is_etag_mismatch`: not done, waits for the codes of a live etag mismatch
