---
cairn: change
id: the-live-night-findings
status: active
created: 2026-11-24
---

# The live night findings

The live test night of 2026-11-23 (workspace .lanes/live/report.md, cardamum lane): `contact-groups list --sync-token <fresh>` fails with "invalid value: integer `-1`, expected u32". Google answers `"totalItems": -1` on a sync-token listing, and io-gpeople 0.9.0 types it `Option<u32>` (src/v1/rest/contact_groups/list.rs:51; connections/list.rs:67 the same).

## What

Read a negative `totalItems` as none, through a deserializer on the field, keeping `Option<u32>` so the change is a patch release rather than a breaking one. The alternative, `Option<i64>`, exposes a value Google uses only as "unknown".
