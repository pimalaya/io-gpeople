---
cairn: spec
capability: projection
status: current
---

# vCard projection

What `GpeoplePerson::from_vcard` and `GpeoplePerson::to_vcard` keep of a vCard that People has no field for, what they never write, and what `GpeoplePerson::merge_unprojected` keeps of a person that its vCard does not show.

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

### Requirement: An update keeps what the vCard cannot show

An update replaces each masked field whole, and a managed field holds more than its vCard shows. Before an update built from a vCard is sent, `GpeoplePerson::merge_unprojected` SHALL be given the server's current person and the update mask, and SHALL rebuild each masked managed field so that nothing the vCard could not express is erased:

- a server entry the vCard hides (a relation other than spouse or child, an organization or occupation after the first, an entry without a value) SHALL be kept as the server holds it, after the vCard's entries;
- a vCard entry paired with a server entry SHALL be that server entry, each sub-field the vCard shows differently from the server entry's own projection taking the vCard's value, every other sub-field (a custom type, a display name, dates, a job description, a country code, phonetic names, metadata) keeping the server's;
- a vCard entry paired with no server entry is a new entry;
- a server entry the vCard shows and that no vCard entry is paired with SHALL be removed, its sub-fields with it: the vCard removed it.

A field outside the mask SHALL be left as the vCard gave it. The mask is computed first, from `changed_fields`, the merged person no longer comparing equal to a projection.

#### Scenario: Relations

- **WHEN** the server holds a spouse, a mother and an assistant, and the vCard renames the spouse
- **THEN** the mask is `relations` and the merged list holds the renamed spouse, the mother and the assistant

#### Scenario: Sub-fields of an organization

- **WHEN** the server's first organization holds a job description and a start date, a second organization follows, and the vCard changes TITLE
- **THEN** the merged list holds the first organization with the new title, its job description and its start date, then the second one

#### Scenario: A type the vCard cannot show

- **WHEN** a server email is typed `School` with a display name, and the vCard adds another email
- **THEN** the first email keeps its type and display name, and the new one follows

#### Scenario: A type the vCard changed

- **WHEN** the vCard gives `TYPE=home` to the email the server types `School`
- **THEN** the merged email is typed `home` and keeps its display name

#### Scenario: A removed entry

- **WHEN** the vCard drops an EMAIL line the server holds
- **THEN** the merged list does not hold that email

### Requirement: A vCard entry is paired by projection, key, then elimination

A vCard entry SHALL be paired with a server entry not yet paired, trying in this order: the one whose own projection equals it; the one of the same key (the address of an email, the number of a phone, the value of a URL or a nickname, the username of an IM client, the name of a related person; an address, an organization and an occupation have no key); the only shown server entry left unpaired, when the vCard entry is the only one left unpaired too. The last rule reads one entry replaced by another as an edit, so the new value keeps the old one's hidden sub-fields.

#### Scenario: The one URL edited

- **WHEN** the server holds one URL typed `blog` and the vCard changes its value
- **THEN** the merged URL holds the new value and the type `blog`

#### Scenario: Two URLs edited at once

- **WHEN** the server holds two typed URLs and the vCard changes both values
- **THEN** the merged list holds two new URLs without a type

### Requirement: A field of one entry stays of one entry

For names, birthdays and biographies, of which a contact holds one, the merged field SHALL hold at most one entry: the vCard's entry laid over the server's (the first name, the first birthday, the first biography that is not HTML), or the server's entry alone when the vCard hides it (a birthday without a day) and gives none. Further server entries, which another source owns and an update does not modify, are not sent.

#### Scenario: Phonetic names

- **WHEN** the server name holds a phonetic given name and the vCard changes FN
- **THEN** the merged name holds the new unstructured name and the phonetic given name

### Requirement: The client data of others survives

With `clientData` in the mask, `merge_unprojected` SHALL keep the server entries under another key than `GPEOPLE_PERSON_STASH_KEY`, after the stash.

### Requirement: A person without a name stashes no FN

`GpeoplePerson::to_vcard` writes an empty FN for a person without a name, the property being mandatory. `GpeoplePerson::from_vcard` SHALL consume an empty FN, and `to_vcard` SHALL NOT restore a stashed `FN:` line, which older stashes hold.

#### Scenario: Round trip

- **WHEN** a person without a name goes through `to_vcard`, then `from_vcard`
- **THEN** the result carries no stash

### Requirement: A line left out of the stash is named

A logical line longer than 8 KiB is not stashed. `GpeoplePerson::from_vcard` SHALL return, next to the person, the name of each such property (`GpeoplePersonVcard::dropped`), uppercased, its group prefix included, so that the caller knows the server copy lacks it.

#### Scenario: An inline photo

- **WHEN** a vCard holds a PHOTO line of 10,000 octets and `X-FOO:bar`
- **THEN** `dropped` is `PHOTO` and the stash holds `X-FOO:bar` alone

### Requirement: A year-less birthday is a People birthday

`GpeoplePerson::from_vcard` SHALL read the first `BDAY` holding a full date (`YYYY-MM-DD` or `YYYYMMDD`) or a year-less one (`--MMDD`, RFC 6350 §4.3.1) into a People birthday, the year-less one with no year. `GpeoplePerson::to_vcard` SHALL write a People birthday whose year is absent or 0 as `--MMDD`. Any other `BDAY` (a reduced precision such as `1985-04`, a value carrying a time, a month or a day out of range) SHALL ride the stash.

#### Scenario: A year-less birthday

- **WHEN** a vCard holding `BDAY:--0412` goes through `from_vcard`, then `to_vcard`
- **THEN** the person's birthday is April 12 with no year, nothing is stashed, and the output holds `BDAY:--0412`

#### Scenario: A reduced-precision birthday

- **WHEN** a vCard holding `BDAY:1985-04` goes through `from_vcard`
- **THEN** the person holds no birthday and the stash holds `BDAY:1985-04`

### Requirement: An unknown enum value stays on its entry

An enum value the crate does not know (cairn/spec/responses.md) is server data no vCard shows. `GpeoplePerson::to_vcard` SHALL NOT write it: a keyword of unknown type is minted without a TYPE, and a biography of unknown content type is shown as plain text. `GpeoplePerson::merge_unprojected` SHALL keep it on its entry, as a sub-field the vCard cannot show, whether the entry is unchanged, edited or hidden.

#### Scenario: A nickname of unknown type, edited

- **WHEN** the server's only nickname has an unknown type and the vCard changes its value
- **THEN** the merged nickname holds the new value and the unknown type
