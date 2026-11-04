---
cairn: change
id: cairn-adoption
status: landed
created: 2026-11-04
---

# Adopt Cairn, retiring docs/

## Why

docs/ held one living document (etags.md) under an ad hoc index, so the crate had no structured place stating what it does today: the src/lib.rs header describes the architecture and CHANGELOG.md records releases, but neither carries the behavioural truth a change can state a delta against.

## What

Keep the three Cairn artifacts next to the code: cairn/spec/ for current truth, cairn/changes/ for in-flight proposals and cairn/log/ for the dated history, with the cairn/ directory as the root marker, AGENTS.md (plus CLAUDE.md) as the activation stanza and cairn/verify.sh as the optional checker. Move docs/etags.md to cairn/spec/etags.md, the first capability, delete docs/ and point CONTRIBUTING.md and the source comments at cairn/.
