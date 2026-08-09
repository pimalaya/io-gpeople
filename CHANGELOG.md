# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed

- Fixed an error response that is not a Google JSON envelope rendering as its whole body, so a 404 answered with an HTML page surfaced as the page. `parse_api_error` now summarises such a body: the HTML `title` when it carries one, else the markup stripped, the whitespace collapsed and the length capped at 200 characters.

## [0.2.0] - 2026-07-16

### Changed

- Moved each resource's data types out of its `types` catch-all submodule into the sibling resource module (people.rs, contact_groups.rs), next to the `pub mod` operation declarations.

  The public paths, for example `people::PeoplePerson` and `contact_groups::PeopleContactGroup`, are unchanged: the types were already re-exported at the resource module, so only the private `types` submodule and its re-export go away.

- Documented every public item, including struct fields, enum variants and methods, aligning with the documentation guidelines.

## [0.1.0] - 2026-07-16

### Added

- Added the I/O-free coroutines covering version 1 of the Google People API: the `people` resource (get, batch get, create, update and delete contacts, batch create, update and delete, search contacts, list and search directory people, contact photo upload and delete), its `connections` sub-resource (list, with incremental sync via a sync token), the `contactGroups` resource (list, get, batch get, create, update, delete) and its `members` sub-resource (modify), and the `otherContacts` resource (list, search, copy to the personal contacts group).
- Added the shared `PeopleSend` request primitive over io-http, the `PeopleCoroutine` contract and the `PeopleClientStd` blocking client (`client` feature), with `connect` opening the TCP and TLS connection behind a TLS feature (`rustls-ring` default, `rustls-aws`, `native-tls`).

[unreleased]: https://github.com/pimalaya/io-people/compare/v0.2.0..HEAD
[0.2.0]: https://github.com/pimalaya/io-people/compare/v0.1.0..v0.2.0
[0.1.0]: https://github.com/pimalaya/io-people/compare/root..v0.1.0
