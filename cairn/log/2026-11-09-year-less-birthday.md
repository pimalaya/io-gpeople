---
cairn: log
change: year-less-birthday
landed: 2026-11-09
---

# Project a year-less BDAY onto a People birthday

A `BDAY:--MMDD` line used to land in the stash, so a year-less birthday set from a vCard never reached the People birthday field and Google did not see it. It now maps both ways: `from_vcard` reads `--MMDD` into a People date with month and day and no year, and `to_vcard` writes a People date whose year is absent or 0 as `--MMDD` instead of dropping it. The other partial forms keep riding the stash, and the stash test moved to one of them (`BDAY:1985-04`).

Capabilities moved: vcard-projection (ADDED, birthdays only).
