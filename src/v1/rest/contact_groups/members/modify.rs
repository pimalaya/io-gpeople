//! Modify the members of a People contact group
//! (`contactGroups.members.modify`).
//!
//! Contacts can be removed from any group but can only be added to a
//! user group.
//!
//! <https://developers.google.com/people/api/rest/v1/contactGroups.members/modify>

use alloc::{format, string::String, vec::Vec};

use io_http::rfc6750::bearer::HttpAuthBearer;
use log::{debug, trace};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::{
    coroutine::*,
    gpeople_try,
    v1::send::{GPEOPLE_API_BASE, GpeopleSend, GpeopleSendError, GpeopleSendOutput},
};

/// People REST contact group members modification response (the person
/// resource names that could not be processed).
#[derive(Debug, Clone, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GpeopleContactGroupMembersModifyResponse {
    /// Person resource names from the request that were not found.
    #[serde(default)]
    pub not_found_resource_names: Vec<String>,
    /// Person resource names that could not be removed because they belong
    /// to no other contact group.
    #[serde(default)]
    pub can_not_remove_last_contact_group_resource_names: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Request<'a> {
    #[serde(skip_serializing_if = "<[String]>::is_empty")]
    resource_names_to_add: &'a [String],
    #[serde(skip_serializing_if = "<[String]>::is_empty")]
    resource_names_to_remove: &'a [String],
}

/// People REST contact group members modification, adding and/or
/// removing person resource names.
pub struct GpeopleContactGroupMembersModify {
    send: GpeopleSend<GpeopleContactGroupMembersModifyResponse>,
}

impl GpeopleContactGroupMembersModify {
    /// Build a group-members modification coroutine. At least one of the
    /// add/remove slices must be non-empty; additions require a user group.
    pub fn new(
        auth: &HttpAuthBearer,
        resource_name: &str,
        resource_names_to_add: &[String],
        resource_names_to_remove: &[String],
    ) -> Result<Self, GpeopleSendError> {
        debug!("prepare people contact group members modification");
        trace!("resource_name: {resource_name:?}");
        trace!("resource_names_to_add: {resource_names_to_add:?}");
        trace!("resource_names_to_remove: {resource_names_to_remove:?}");

        if resource_name.trim().is_empty() {
            let err =
                GpeopleSendError::InvalidRequest("Group resource name cannot be empty".into());
            return Err(err);
        }

        if resource_names_to_add.is_empty() && resource_names_to_remove.is_empty() {
            let err = GpeopleSendError::InvalidRequest(
                "Resource names to add and to remove cannot both be empty".into(),
            );
            return Err(err);
        }

        let url = Url::parse(GPEOPLE_API_BASE)?.join(&format!("{resource_name}/members:modify"))?;

        let request = Request {
            resource_names_to_add,
            resource_names_to_remove,
        };

        let send = GpeopleSend::post_json(auth, url, &request)?;

        Ok(Self { send })
    }
}

impl GpeopleCoroutine for GpeopleContactGroupMembersModify {
    type Yield = GpeopleYield;
    type Return =
        Result<GpeopleSendOutput<GpeopleContactGroupMembersModifyResponse>, GpeopleSendError>;

    fn resume(&mut self, arg: Option<&[u8]>) -> GpeopleCoroutineState<Self::Yield, Self::Return> {
        let out = gpeople_try!(&mut self.send, arg);
        debug!("people contact group members modified");
        trace!("out: {out:?}");
        GpeopleCoroutineState::Complete(Ok(out))
    }
}
