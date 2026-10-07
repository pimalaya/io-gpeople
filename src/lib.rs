#![no_std]
#![deny(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]

//! # io-gpeople
//!
//! I/O-free Google People coroutines built on [io-http]. Every
//! network exchange is a resumable state machine that emits read and
//! write requests instead of performing I/O itself: the caller owns
//! the socket and pumps the coroutine with the bytes it read, whatever
//! the runtime (blocking, async, in-memory tests). The `client`
//! feature ships a ready-made std-blocking pump for callers who just
//! want a working client.
//!
//! [io-http]: https://docs.rs/io-http
//!
//! ## Layout: everything under `v1`
//!
//! The People API is versioned (`/v1/`), so the crate is too: the
//! version-agnostic primitives stay at the crate root ([`coroutine`])
//! and everything v1-specific lives under [`v1`]. A future People v2
//! would slot in as a sibling `v2` without breaking `v1` consumers.
//!
//! Inside [`v1`], [`v1::rest`] mirrors the REST reference one to one:
//! each resource is a folder ([`v1::rest::people`],
//! [`v1::rest::contact_groups`], [`v1::rest::other_contacts`]) and each
//! method a file named after the API method in snake_case (`getBatchGet`
//! becomes `get_batch_get`). A reader who knows the reference knows
//! where to look.
//!
//! ## The send primitive
//!
//! Every People call is an independent HTTP request/response, so the
//! crate has a single shared primitive every coroutine delegates to:
//! [`v1::send::GpeopleSend`], wrapping io-http's `Http11Send`. It builds
//! the request (the `Authorization` header from the caller's bearer
//! token, `Accept: application/json`, an optional JSON body) and on
//! completion deserialises the 2xx body into `T`, or parses People's
//! JSON error envelope into `GpeopleSendError`. The envelope is kept
//! whole in [`v1::send::GpeopleApiError`] (status, `errors[].reason`,
//! `status` and `details[].reason`), so callers match rate limits and
//! an expired sync token on Google's codes rather than on the message
//! text. A 3xx surfaces as an unexpected-redirect error; redirects are
//! never followed. Empty 2xx
//! bodies (delete, batch, stop) deserialise into the `GpeopleNoResponse`
//! unit marker.
//!
//! ## The coroutine contract
//!
//! [`coroutine`] holds the crate-local, version-agnostic shape: the
//! [`coroutine::GpeopleCoroutine`] trait with `resume(Option<&[u8]>)`,
//! the [`coroutine::GpeopleCoroutineState`] (`Yielded` or `Complete`)
//! and the [`coroutine::GpeopleYield`] (`WantsRead` / `WantsWrite`, a
//! People call being I/O-only: no clock, randomness or filesystem). The
//! `gpeople_try!` macro is the coroutine `?`. Because each call is a
//! single I/O step, every REST coroutine is a thin wrapper holding the
//! send directly, with no multi-variant `State` enum.
//!
//! ## Types
//!
//! Domain types are `People`-prefixed
//! ([`v1::rest::people::GpeoplePerson`], ...), never re-exported at the
//! crate root, and mirror the REST schema fully. A full-resource request
//! body takes the whole resource by reference; enum-valued wire fields
//! are typed `People`-prefixed enums, while free-form `type` labels stay
//! `Option<String>` since the API accepts custom values there. Field
//! masks take `&[GpeoplePersonField]` / `&[GpeopleGroupField]` joined by
//! [`v1::query::to_field_mask`]; list methods take a borrowed `*Params`
//! struct flattened into query pairs by [`v1::query`]'s tiny no_std serde
//! serializer.
//!
//! ## Incremental sync
//!
//! The People API has no push channel and no history log: incremental
//! sync is a token loop on the list methods. `people.connections.list`
//! and `otherContacts.list` return a sync token on the last page;
//! passing it back returns only what changed (deletions come back
//! flagged `metadata.deleted`). An expired token surfaces as HTTP 410,
//! which callers handle by re-baselining with a full listing. The etag
//! gotcha `updateContact` exposes is documented in
//! [docs/etags.md](https://github.com/pimalaya/io-gpeople/blob/master/docs/etags.md).
//!
//! ## The std client
//!
//! The optional [`v1::client`] module (`client` feature) is the
//! std-blocking [`v1::client::GpeopleClientStd`]: a light client wrapping
//! any stream you opened yourself, or a full client opening the TCP/TLS
//! connection itself when a TLS feature (`rustls-ring` default,
//! `rustls-aws`, `native-tls`) is enabled. It offers one convenience
//! method per first-class verb and drives the photo, batch and directory
//! coroutines through its generic `run`.
//!
//! ## Authentication and intentional omissions
//!
//! The credential is exactly a bare OAuth 2.0 bearer token, the only
//! thing People accepts: the client stores an `HttpAuthBearer` and
//! coroutines take `&HttpAuthBearer`; minting and refreshing tokens is
//! the caller's job. `Person` fields the reference marks as returning no
//! data (`ageRange`, `residences`, ...) and the deprecated
//! `requestMask.includeField` query parameter are deliberately omitted.

extern crate alloc;
#[cfg(feature = "client")]
extern crate std;

pub mod coroutine;
pub mod v1;
