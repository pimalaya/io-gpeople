---
cairn: delta
change: stash-lines-and-resource-names
---

## ADDED Requirements

### Requirement: The stash holds logical lines

A property `GpeoplePerson::from_vcard` neither projects nor mints SHALL be stashed in the `clientData` entry keyed `GPEOPLE_PERSON_STASH_KEY` as its logical line: name, parameters and raw value, without fold, soft break or line ending. The lines of a stash are separated by one LF. `GpeoplePerson::to_vcard` SHALL restore each stashed line whole, whatever its name, its length and the places it was folded at.

#### Scenario: A folded property

- **WHEN** a vCard whose `X-CUSTOM-FIELD` line is 165 octets long and folded twice goes through `from_vcard`, then `to_vcard`
- **THEN** the stash holds the 165 octets on one row, and the output, unfolded, holds the same line

#### Scenario: A folded UID

- **WHEN** a vCard whose UID is 120 characters long and folded once goes through `from_vcard`
- **THEN** `stashed_uid` returns the 120 characters

### Requirement: A stash stored with its folds reads back whole

A stash written before 2026-11-13 holds each line as it travelled, folds included. Reading a stash, a row that starts with a space or a tab SHALL continue the row before it, that row's trailing CR and its own first character dropped. A blank row SHALL end the line before it: a row after it is a line of its own, its leading spaces and tabs dropped. A stash stored whole reads the same under this rule, a logical line never starting with a space or a tab.

#### Scenario: A stored fold

- **WHEN** the stash holds `X-FOO:abc`, CR, LF, a space, `def`, LF, `GENDER:F`
- **THEN** its lines are `X-FOO:abcdef` and `GENDER:F`

#### Scenario: A stored blank row

- **WHEN** the stash holds `GENDER:F`, LF, CR, LF, a space, `X-FOO:a`
- **THEN** its lines are `GENDER:F` and `X-FOO:a`

### Requirement: A projected URI adds no line

`GpeoplePerson::to_vcard` SHALL percent-encode every control character of a URL or IMPP value, these two being written without escaping. A line break held by a People url, IM username or IM protocol SHALL stay inside the value of its own property.

#### Scenario: A line break in a URL

- **WHEN** a person's url is `https://doe.org/`, CR, LF, `TEL:+339999`
- **THEN** `to_vcard` writes `URL:https://doe.org/%0D%0ATEL:+339999` on one line, and `from_vcard` of the output reads no phone number from it

### Requirement: A resource name stays in its collection

A coroutine addressing one person, contact group or other contact SHALL accept only a resource name `<collection>/<id>`, the collection being the one its method serves (`people`, `contactGroups` or `otherContacts`) and the id being neither empty, `.` nor `..`. Any other name SHALL fail with `GpeopleSendError::InvalidRequest` before a request is built.

#### Scenario: A bare id

- **WHEN** `GpeopleContactDelete` is built for `c1`
- **THEN** it fails with `InvalidRequest`

#### Scenario: Another collection

- **WHEN** `GpeopleContactGroupDelete` is built for `people/c1`
- **THEN** it fails with `InvalidRequest`

### Requirement: A resource id is one path segment

The id of a resource name SHALL be percent-encoded as a single path segment under `GPEOPLE_API_BASE`, the method suffix (`:deleteContact`, `/members:modify`) following it. An id holding `/`, `?`, `#` or `\` SHALL reach the resource it names verbatim, never another path, a query or a fragment.

#### Scenario: A group name carrying a query

- **WHEN** `GpeopleContactGroupDelete` is built for `contactGroups/abc?deleteContacts=true` with `delete_contacts` false
- **THEN** its request line is `DELETE /v1/contactGroups/abc%3FdeleteContacts=true HTTP/1.1`

#### Scenario: A person name carrying a fragment

- **WHEN** `GpeopleContactDelete` is built for `people/c1#x`
- **THEN** its request path is `/v1/people/c1%23x:deleteContact`

## MODIFIED Requirements

## REMOVED Requirements
