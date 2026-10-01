//! Update a People contact group (`contactGroups.update`).
//!
//! The group must carry its server-assigned resource name and the etag
//! from the latest read; every field named in the update mask is fully
//! replaced.
//!
//! <https://developers.google.com/people/api/rest/v1/contactGroups/update>

use alloc::string::String;

use io_http::rfc6750::bearer::HttpAuthBearer;
use log::{debug, trace};
use serde::Serialize;
use url::Url;

use crate::{
    coroutine::*,
    gpeople_try,
    v1::{
        query::to_field_mask,
        rest::contact_groups::{GpeopleContactGroup, GpeopleGroupField},
        send::{GPEOPLE_API_BASE, GpeopleSend, GpeopleSendError, GpeopleSendOutput},
    },
};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Request<'a> {
    contact_group: &'a GpeopleContactGroup,
    #[serde(skip_serializing_if = "String::is_empty")]
    update_group_fields: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    read_group_fields: String,
}

/// People REST contact group update, replacing the masked fields (only
/// `name` and `clientData` can be updated).
pub struct GpeopleContactGroupUpdate {
    send: GpeopleSend<GpeopleContactGroup>,
}

impl GpeopleContactGroupUpdate {
    /// Build a contact-group update coroutine. `update_group_fields` names
    /// the fields to replace; `read_group_fields` controls the response.
    pub fn new(
        auth: &HttpAuthBearer,
        group: &GpeopleContactGroup,
        update_group_fields: &[GpeopleGroupField],
        read_group_fields: &[GpeopleGroupField],
    ) -> Result<Self, GpeopleSendError> {
        debug!("prepare people contact group for update");
        trace!("group: {group:?}");
        trace!("update_group_fields: {update_group_fields:?}");
        trace!("read_group_fields: {read_group_fields:?}");

        if group.resource_name.trim().is_empty() {
            let err =
                GpeopleSendError::InvalidRequest("Group resource name cannot be empty".into());
            return Err(err);
        }

        let url = Url::parse(GPEOPLE_API_BASE)?.join(&group.resource_name)?;

        let request = Request {
            contact_group: group,
            update_group_fields: to_field_mask(update_group_fields),
            read_group_fields: to_field_mask(read_group_fields),
        };

        let send = GpeopleSend::put_json(auth, url, &request)?;

        Ok(Self { send })
    }
}

impl GpeopleCoroutine for GpeopleContactGroupUpdate {
    type Yield = GpeopleYield;
    type Return = Result<GpeopleSendOutput<GpeopleContactGroup>, GpeopleSendError>;

    fn resume(&mut self, arg: Option<&[u8]>) -> GpeopleCoroutineState<Self::Yield, Self::Return> {
        let out = gpeople_try!(&mut self.send, arg);
        debug!("people contact group updated");
        trace!("out: {out:?}");
        GpeopleCoroutineState::Complete(Ok(out))
    }
}
