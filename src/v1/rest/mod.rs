//! People API v1 resources: each method is a file and each resource a
//! module, mirroring the reference tree.
//!
//! <https://developers.google.com/people/api/rest>

pub mod contact_groups;
#[cfg(feature = "client")]
pub mod other_contacts;
pub mod people;
