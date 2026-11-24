//! People API v1 resources: each method is a file and each resource a
//! module, mirroring the reference tree.
//!
//! <https://developers.google.com/people/api/rest>

#[cfg(feature = "client")]
use serde::{Deserialize, Deserializer};

pub mod contact_groups;
#[cfg(feature = "client")]
pub mod other_contacts;
pub mod people;

/// Reads a listing total as a count, none when the server does not know
/// it: a sync-token listing answers `-1`.
#[cfg(feature = "client")]
pub(crate) fn deserialize_total<'de, D>(deserializer: D) -> Result<Option<u32>, D::Error>
where
    D: Deserializer<'de>,
{
    let total = Option::<i64>::deserialize(deserializer)?;
    Ok(total.and_then(|total| u32::try_from(total).ok()))
}
