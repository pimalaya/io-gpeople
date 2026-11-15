---
cairn: log
change: unremovable-only-without-a-stash
landed: 2026-11-15
---

# A stashed property is unremovable only when the update carries no stash

Found by the live testing campaign of 2026-11-15 on the Google test account, through cardamum (low severity). `cardamum -a google card update <id> card.vcf`, card.vcf being the card minus its `X-CUSTOM-FIELD` line, exited 0 with the warning "except X-CUSTOM-FIELD, which the server will not let go". Read-backs over 20 seconds showed the property gone from Google, and the same update run again reported `keptProperties: []`: the write had removed it, the warning was wrong.

**Cause.** People treats the stash `clientData` entry as one value: an update carrying a stash replaces the stored one whole, an update carrying none (an explicit `"clientData": []` included, cardamum's live finding P8) leaves the stored one standing. `GpeoplePerson::unremovable_properties` stated that rule in its doc but compared the two stashes line name by line name, naming each property of the base stash missing from the new one. With `UID` left in the new stash, as nearly every card has it, the new stash replaced the stored one and took `X-CUSTOM-FIELD` with it, yet the function named `X-CUSTOM-FIELD`. The suspected cause was the right one.

**Fix.** `unremovable_properties` names nothing when the person carries a stash, and every property of the base stash when it carries none. The second case behaves as before.

Capabilities moved: projection (one requirement added, its intro now naming `unremovable_properties`).

Verified: `a_new_stash_keeps_none_of_the_lines_it_drops` in src/v1/rest/people/vcard.rs, persons built by `from_vcard` as consumers build them, failed against the previous code (`left: ["X-CUSTOM-FIELD"]`, `right: []`) and passes. `cargo test --all-features` passed (35 unit, 34 integration, the 2 live tests ignored and not run), `cargo clippy --all-features --all-targets -- -D warnings` clean, `cargo build --no-default-features` builds, `cargo fmt` applied. Not re-run against the live API.

Left open:

- Whether People keeps the stored stash when an update carries no stash but carries the `clientData` entries of other clients, which `merge_unprojected` keeps: not observed. The function assumes it stands, as before.
- A base stash holding one property twice names it twice, as before.
