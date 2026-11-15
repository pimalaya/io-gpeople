---
cairn: delta
change: unremovable-only-without-a-stash
---

## ADDED Requirements

### Requirement: Only a stash left out of an update stands

An update carrying a stash replaces the stored one whole, and an update carrying none leaves the stored one standing, whatever the mask says. `GpeoplePerson::unremovable_properties` SHALL name the property of each line of `base`'s stash when the person carries no stash, and SHALL name none when it carries one, whatever lines the two stashes share.

#### Scenario: A stashed line dropped beside another

- **WHEN** the base stash holds `UID` and `X-CUSTOM-FIELD`, and the person's stash holds `UID` alone
- **THEN** `unremovable_properties` names nothing

#### Scenario: Every stashed line dropped

- **WHEN** the base stash holds `UID` and `X-CUSTOM-FIELD`, and the person carries no stash
- **THEN** `unremovable_properties` names `UID` and `X-CUSTOM-FIELD`

## MODIFIED Requirements

## REMOVED Requirements
