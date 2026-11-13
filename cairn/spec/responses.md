---
cairn: spec
capability: responses
status: current
---

# Responses

How the coroutines read what the People API answers.

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
