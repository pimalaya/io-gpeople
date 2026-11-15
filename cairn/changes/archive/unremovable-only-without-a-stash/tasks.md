---
cairn: tasks
change: unremovable-only-without-a-stash
---

# Tasks

- [x] Unit test in src/v1/rest/people/vcard.rs failing against the previous code: `a_new_stash_keeps_none_of_the_lines_it_drops`
- [x] `unremovable_properties` names the base stash only when the person carries no stash
- [x] `cargo test --all-features`, `cargo clippy --all-features --all-targets -- -D warnings`, `cargo build --no-default-features`, `cargo fmt`
- [x] CHANGELOG under [Unreleased]
- [x] Fold the delta into cairn/spec/projection.md, log, land
