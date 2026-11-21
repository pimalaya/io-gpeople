//! People API v1.
//!
//! `rest` mirrors the REST resource tree (`people`, `contactGroups`,
//! `otherContacts`); `send` and `query` are the shared request
//! primitives, and `client` the optional std driver.

#[cfg(feature = "std")]
pub mod client;
#[cfg(feature = "client")]
pub mod query;
pub mod rest;
#[cfg(feature = "client")]
pub mod send;
