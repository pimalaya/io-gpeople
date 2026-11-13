---
cairn: log
change: year-less-birthday
landed: 2026-11-13
---

# Project a year-less BDAY onto a People birthday

A `BDAY:--MMDD` line used to land in the stash, so a year-less birthday set from a vCard never reached the People birthday field and Google did not see it (pimalaya/cardamum#26). It now maps both ways: `from_vcard` reads `--MMDD` into a People date with month and day and no year, and `to_vcard` writes a People date whose year is absent or 0 as `--MMDD` instead of dropping it. The other partial forms keep riding the stash, and the stash test moved to one of them (`BDAY:1985-04`).

A year-less server birthday is now shown, so `merge_unprojected` no longer keeps it through an update whose vCard drops the `BDAY`: the vCard removed it. A `BDAY:--MMDD` stashed by an earlier version is restored by `to_vcard` and moves into the birthday on the next `from_vcard`.

Contributed by jontxux (pimalaya/io-gpeople#2, written with Claude Code), who checked it against the live People API through cardamum; rebased onto 0.7.0 by the maintainer, its spec file folded into projection.

Capabilities moved: projection (one requirement added, one example reworded).

Verified: `a_year_less_birthday_round_trips_through_a_people_date` and `a_people_birthday_with_year_zero_projects_year_less` fail against the previous code and pass. `cargo test --all-features` passed, `cargo clippy --all-features --all-targets -- -D warnings` clean, `cargo build --no-default-features` builds, cairn/verify.sh conformant.
