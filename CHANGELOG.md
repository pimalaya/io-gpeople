# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.4.1] - 2026-10-01

### Added

- Added the `vcard` feature: `GpeoplePerson::to_vcard` and `from_vcard` project a person onto a vCard 4.0 document and back, and `changed_fields` and `unremovable_properties` shape an update against the base person.

  Google-scoped fields ride as read-only `X-GOOGLE-*` properties, every other line round-trips through a `clientData` stash. A read meant for the projection passes `GPEOPLE_PERSON_VCARD_FIELDS`. The projection moved here from Cardamum, stash key included, so persons Cardamum already stashed still read back.

- Added `GpeoplePerson::id`, the bare id behind the `people/<id>` resource name.

## [0.4.0] - 2026-10-01

### Added

- **BREAKING** Added `GpeopleClientStdConnectOptions::proxy`, routing the connection through a SOCKS5 or HTTP proxy. The default resolves it from the environment, as before.

### Changed

- **BREAKING** Renamed every `People*` type to `Gpeople*`, the `people_try!` macro to `gpeople_try!` and `PEOPLE_API_BASE` to `GPEOPLE_API_BASE`, matching io-gcal and io-gmail. The `rest::people` module keeps the API's resource name.

### Fixed

- Fixed the `client` feature failing to build without a TLS feature.

## [0.3.1] - 2026-09-28

### Changed

- Renamed the crate from io-people to io-gpeople.

  The library path moved from `io_people` to `io_gpeople`, types are unchanged.

### Fixed

- Fixed `no_std` builds pulling in `std` ([io-gmail#2]).

  The `serde_variant` dependency was dropped, enum query parameters now go through the in-crate query serializer.

## [0.3.0] - 2026-08-15

### Changed

- Bumped pimalaya-stream to 0.3. **Behaviour change.**

  `StreamStd` became `stream::Stream`, with per-transport connect options. The transport now retries a spurious `EAGAIN` for a minute before failing with `TimedOut`, and arms a read deadline at connect time.

- Bumped io-http to 0.5.

  The coroutines take and yield its types, so consumers must bump in step.

- Raised the minimum supported Rust version from 1.87 to 1.88.

### Fixed

- Fixed non-JSON error responses surfacing their whole body.

  `parse_api_error` now keeps the HTML `title`, or else the stripped text capped at 200 characters.

## [0.2.0] - 2026-07-16

### Changed

- Moved each resource's types out of its private `types` submodule into the resource module.

  Public paths such as `people::PeoplePerson` are unchanged.

- Documented every public item.

## [0.1.0] - 2026-07-16

### Added

- Added the I/O-free coroutines for the Google People API v1: people, connections, contact groups and other contacts.
- Added `PeopleClientStd`, a std blocking client behind the `client` feature.

[unreleased]: https://github.com/pimalaya/io-gpeople/compare/v0.4.1..HEAD
[0.4.1]: https://github.com/pimalaya/io-gpeople/compare/v0.4.0..v0.4.1
[0.4.0]: https://github.com/pimalaya/io-gpeople/compare/v0.3.1..v0.4.0
[0.3.1]: https://github.com/pimalaya/io-gpeople/compare/v0.3.0..v0.3.1
[0.3.0]: https://github.com/pimalaya/io-gpeople/compare/v0.2.0..v0.3.0
[0.2.0]: https://github.com/pimalaya/io-gpeople/compare/v0.1.0..v0.2.0
[0.1.0]: https://github.com/pimalaya/io-gpeople/compare/root..v0.1.0

[io-gmail#2]: https://github.com/pimalaya/io-gmail/issues/2
