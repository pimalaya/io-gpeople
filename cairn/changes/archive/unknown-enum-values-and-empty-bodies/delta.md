---
cairn: delta
change: unknown-enum-values-and-empty-bodies
---

## ADDED Requirements

### Requirement: An unknown enum value reads as unknown

Every enum a response fills (`GpeopleSourceType`, `GpeopleObjectType`, `GpeopleAgeRange`, `GpeopleContentType`, `GpeopleMiscKeywordType`, `GpeopleNicknameType`, `GpeopleContactGroupType`) SHALL read a value the crate does not know as its `Unknown` variant rather than fail the whole response. A field holding `Unknown` SHALL be left out of every body it is written in, so a write never states a value the server did not give. The enums a caller alone fills (field masks, sources, sort orders) have no such variant.

#### Scenario: A source type to come

- **WHEN** a listing page holds a person whose `metadata.sources[].type` is `FUTURE_SOURCE`
- **THEN** the page parses, and that source type is `GpeopleSourceType::Unknown`

#### Scenario: Written back

- **WHEN** a person read with a nickname of type `FUTURE_NICKNAME` is sent in an update
- **THEN** the body holds the nickname's value and no `type` for it

### Requirement: An empty success body reads as an empty object

A 2xx response without a body SHALL be read as `{}`: `GpeopleNoResponse` ignores it, and a struct response takes the default of each of its fields.

#### Scenario: A photo deletion answered without a body

- **WHEN** `GpeopleContactPhotoDelete` receives a 200 with an empty body
- **THEN** it completes with a response whose `person` is `None`

### Requirement: An unknown enum value stays on its entry

An enum value the crate does not know (cairn/spec/responses.md) is server data no vCard shows. `GpeoplePerson::to_vcard` SHALL NOT write it: a keyword of unknown type is minted without a TYPE, and a biography of unknown content type is shown as plain text. `GpeoplePerson::merge_unprojected` SHALL keep it on its entry, as a sub-field the vCard cannot show, whether the entry is unchanged, edited or hidden.

#### Scenario: A nickname of unknown type, edited

- **WHEN** the server's only nickname has an unknown type and the vCard changes its value
- **THEN** the merged nickname holds the new value and the unknown type

## MODIFIED Requirements

## REMOVED Requirements
