//! People contact groups (`contactGroups`): list, get, batchGet,
//! create, update, delete.
//!
//! <https://developers.google.com/people/api/rest/v1/contactGroups>

use alloc::{string::String, vec::Vec};

use serde::{Deserialize, Serialize};

pub mod batch_get;
pub mod create;
pub mod delete;
pub mod get;
pub mod list;
pub mod members;
pub mod update;

use crate::v1::rest::people::GpeopleStatus;

/// Contact group owned by the authenticated user.
#[derive(Debug, Clone, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GpeopleContactGroup {
    /// Server-assigned identifier, e.g. `contactGroups/123`.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub resource_name: String,
    /// HTTP entity tag used for optimistic concurrency control.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub etag: String,
    /// Server-managed metadata such as the last update time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<GpeopleContactGroupMetadata>,
    /// Whether this is a user-created group or a system group.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group_type: Option<GpeopleContactGroupType>,
    /// User-defined name of the contact group.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Display name translated or formatted for the viewer's locale.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub formatted_name: Option<String>,
    /// Resource names of the people that are members of the group.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub member_resource_names: Vec<String>,
    /// Total number of members in the group (may exceed the returned list).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub member_count: Option<u32>,
    /// Arbitrary key-value pairs stored by the calling application.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub client_data: Vec<GpeopleGroupClientData>,
}

/// Metadata about a contact group.
#[derive(Debug, Clone, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GpeopleContactGroupMetadata {
    /// RFC 3339 timestamp of the most recent modification.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub update_time: Option<String>,
    /// `true` when the group has been deleted; present only in sync responses.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deleted: Option<bool>,
}

/// Owner of the contact group: created by Google or by the user.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GpeopleContactGroupType {
    /// Default value; should not be used.
    GroupTypeUnspecified,
    /// Group created by the authenticated user.
    UserContactGroup,
    /// Read-only group maintained by Google (e.g. "Starred").
    SystemContactGroup,
}

/// Arbitrary client data attached to a contact group.
#[derive(Debug, Clone, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GpeopleGroupClientData {
    /// Application-defined key for this piece of client data.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    /// Application-defined value associated with the key.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

/// Response for a single requested contact group in a batch method.
#[derive(Debug, Clone, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GpeopleContactGroupResponse {
    /// The contact group that was requested, if successfully retrieved.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contact_group: Option<GpeopleContactGroup>,
    /// The resource name originally requested by the caller.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requested_resource_name: Option<String>,
    /// Error status when this particular group could not be retrieved.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<GpeopleStatus>,
}

/// Contact group field selectable in a `groupFields` field mask.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum GpeopleGroupField {
    /// Arbitrary key-value data stored by the calling application.
    ClientData,
    /// Whether the group is user-created or a system group.
    GroupType,
    /// Total number of members in the group.
    MemberCount,
    /// Server-managed metadata including the last update time.
    Metadata,
    /// User-defined display name of the group.
    Name,
}
