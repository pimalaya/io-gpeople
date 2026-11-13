---
cairn: delta
change: updates-keep-what-the-vcard-cannot-show
---

## ADDED Requirements

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

For names, birthdays and biographies, of which a contact holds one, the merged field SHALL hold at most one entry: the vCard's entry laid over the server's (the first name, the first birthday, the first biography that is not HTML), or the server's entry alone when the vCard hides it (a birthday without a year) and gives none. Further server entries, which another source owns and an update does not modify, are not sent.

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

## MODIFIED Requirements

## REMOVED Requirements
