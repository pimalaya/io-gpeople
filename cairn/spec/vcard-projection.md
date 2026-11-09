---
cairn: spec
capability: vcard-projection
status: current
---

# vCard projection

`GpeoplePerson::to_vcard` and `GpeoplePerson::from_vcard` project a person onto a vCard 4.0 document and back. This file grows by accretion; the src/v1/rest/people/vcard.rs header describes the rest of the projection.

## Birthdays

The first People birthday projects onto `BDAY`, and a `BDAY` line projects onto a People birthday when the person has none yet:

- a full date (`YYYY-MM-DD`, or `YYYYMMDD` on read) maps to a People date with year, month and day;
- a year-less date (`--MMDD`, RFC 6350 section 4.3.1) maps to a People date with month and day and no year. On write, a People date whose year is absent or 0 projects as `--MMDD`;
- any other partial date (reduced precision such as `1985-04`, a month-less `---12`, a value carrying a time) and a year-less date with an out-of-range month or day have no People slot: the line rides the stash and is spliced back verbatim on read.
