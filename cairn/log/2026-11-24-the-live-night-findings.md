---
cairn: log
change: the-live-night-findings
landed: 2026-11-24
---

# A negative listing total no longer fails the page

G1 of the change the-live-night-findings. On the live night of 2026-11-23, cardamum's `contact-groups list --sync-token <fresh>` failed with "invalid value: integer -1, expected u32": People answers `"totalItems": -1` on a sync-token listing, and `total_items` was a plain `Option<u32>` on `GpeopleContactGroupsListResponse` and `GpeopleConnectionsListResponse`.

Both fields now go through `v1::rest::deserialize_total`, which reads the number as an `i64` and keeps it only when it fits a `u32`, so `-1` reads as `None`. The type is unchanged, so this is a patch-level fix.

Capabilities moved: responses (one requirement added).

Verified: tests/coroutines.rs `a_negative_total_reads_as_unknown` failed against the previous code with a `ParseResponse` error ("invalid value: integer -1, expected u32") and passes. `cargo test --all-features` passed, clippy `-D warnings` clean on all features, default, `--no-default-features` (lib) and `--no-default-features --features client`.

Left open: G2, the patch release (owner). `total_people`, `total_size` (other contacts, directory search) keep the plain `Option<u32>`: no negative was seen on them.
