---
cairn: log
change: unknown-enum-values-and-empty-bodies
landed: 2026-11-13
---

# An unknown enum value and an empty body no longer fail a response

Three findings of the family audit of 2026-11-13: GPEOPLE-04, GPEOPLE-08 and the documentation half of GPEOPLE-07.

**Enums.** The seven enums a response fills (`GpeopleSourceType`, `GpeopleObjectType`, `GpeopleAgeRange`, `GpeopleContentType`, `GpeopleMiscKeywordType`, `GpeopleNicknameType`, `GpeopleContactGroupType`) were closed, and the first sits on the metadata of every field: one value added by the API failed each page holding it with "unknown variant", so a whole address book stopped listing. Each now has an `Unknown` variant taking any other value, and each field of these types leaves `Unknown` out of written bodies the way it leaves `None` out. The six enums a caller alone fills are untouched. In the vCard projection, a keyword of unknown type is minted without a TYPE (it was minted `TYPE=UNKNOWN` once the variant existed), and the merge keeps an unknown value on its entry, being a sub-field no vCard shows.

**Empty body.** A 2xx without a body was read as `null`, which failed every response that is a struct ("invalid type: null"). It reads as `{}`, as in the sibling crates.

**Docs.** src/lib.rs: types are `Gpeople`-prefixed, an expired sync token is a 410 or a 400 carrying `EXPIRED_SYNC_TOKEN`, an empty body reads as `{}`. src/v1/rest/people/vcard.rs: the citations of `project::full_date` and of two spec files absent from this repository are gone. examples/std_gpeople.rs: the command names the example.

Capabilities moved: responses (new spec file, two requirements), projection (one requirement added).

Verified: in tests/coroutines.rs, `an_unknown_enum_value_does_not_fail_the_page`, `an_unknown_group_type_does_not_fail_the_listing`, `an_unknown_enum_value_is_never_written_back` and `an_empty_success_body_reads_as_an_empty_object` fail against the previous code and pass; in src/v1/rest/people/vcard.rs, `an_unknown_enum_value_stays_on_its_entry_and_out_of_the_vcard` failed on the minted `TYPE=UNKNOWN` and passes. `cargo test --all-features` passed (31 unit, 32 integration, the 2 live tests ignored and not run), `cargo clippy --all-features --all-targets -- -D warnings` clean, `cargo build --no-default-features` builds.

Left open:

- `is_etag_mismatch`: the predicate needs the status and reasons People returns for an etag mismatch, to be collected from a live exchange and written in cairn/spec/etags.md first.
- `Unknown` does not keep the string the server gave. An update that rewrites a list sends an entry of unknown nickname type or content type without it, and the server gives its default. Keeping the string needs a variant holding it, at the cost of `Copy` on these enums.
