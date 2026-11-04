---
cairn: log
change: cairn-adoption
landed: 2026-11-04
---

# Adopt Cairn, retiring docs/

docs/ held a README index and one document, etags.md. This repository now keeps cairn/spec/ (current truth, one file per capability), cairn/changes/ (in-flight proposals) and cairn/log/ (dated history), with the cairn/ directory as the root marker, AGENTS.md (plus CLAUDE.md) as the activation stanza and cairn/verify.sh, vendored from io-gmail, as the optional checker. docs/etags.md became cairn/spec/etags.md, its closing "why the library does not do this" section recast as the requirement it describes. docs/ was deleted, and CONTRIBUTING.md, the src/lib.rs header and the `GpeopleContactUpdate` module doc now point at cairn/.

The rest of the spec grows by accretion: a capability gets a file the first time a change touches it. History before this entry lives in CHANGELOG.md and the git log, up to the dependency bump (io-http 0.6, vcard-rs 0.6, Rust 1.89) and the switch from base64 to base64-codec, both of 2026-11-04.

Capabilities moved: etags (ADDED, moved from docs/etags.md). This change moved documentation, not behaviour.
