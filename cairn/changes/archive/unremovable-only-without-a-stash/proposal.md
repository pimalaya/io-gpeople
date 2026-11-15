---
cairn: change
id: unremovable-only-without-a-stash
status: landed
created: 2026-11-15
---

# A stashed property is unremovable only when the update carries no stash

## Why

Found by the live testing campaign of 2026-11-15 (low severity). A card update dropping its `X-CUSTOM-FIELD` line, its `UID` line staying, landed and removed the property from the server: read-backs over 20 seconds no longer held it, and the same update run again named nothing. Yet the first run warned that the server would keep `X-CUSTOM-FIELD`.

People treats the stash, a `clientData` entry, as one value: an update carrying a stash replaces the stored one whole, an update carrying none leaves the stored one standing, whatever the mask says (an explicit `"clientData": []` included). `GpeoplePerson::unremovable_properties` states that rule in its doc but compared the two stashes line name by line name: it named every property of the base stash missing from the new one, even when the new stash, holding a `UID` or any other line, replaced the stored one and took the dropped line with it. A card nearly always stashes its `UID`, so the warning was wrong for nearly every dropped property.

## What

- `unremovable_properties` names nothing when the person carries a stash, and every property of the base stash when it carries none, which is the only case People keeps them.
- A unit test, failing on the old code, pins both cases on persons built by `from_vcard`, the way consumers build them.

## Not in this change

- Whether People keeps the stored stash when the update carries no stash but carries the `clientData` entries of other clients (which `merge_unprojected` keeps): not observed, the live evidence covers an empty list only. The function assumes it stands, as before.
- Duplicate names: a base stash holding the same property twice names it twice, as before.
