//! Create a People contact group (`contactGroups.create`).
//!
//! <https://developers.google.com/people/api/rest/v1/contactGroups/create>

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
    read_group_fields: String,
}

/// People REST contact group creation, from a whole group resource.
pub struct GpeopleContactGroupCreate {
    send: GpeopleSend<GpeopleContactGroup>,
}

impl GpeopleContactGroupCreate {
    /// Build a contact-group creation coroutine; `group.name` must be
    /// non-empty. `read_group_fields` controls the fields in the response.
    pub fn new(
        auth: &HttpAuthBearer,
        group: &GpeopleContactGroup,
        read_group_fields: &[GpeopleGroupField],
    ) -> Result<Self, GpeopleSendError> {
        debug!("prepare people contact group for creation");
        trace!("group: {group:?}");
        trace!("read_group_fields: {read_group_fields:?}");

        let name_is_empty = group
            .name
            .as_deref()
            .is_none_or(|name| name.trim().is_empty());

        if name_is_empty {
            let err = GpeopleSendError::InvalidRequest("Group name cannot be empty".into());
            return Err(err);
        }

        let url = Url::parse(GPEOPLE_API_BASE)?.join("contactGroups")?;

        let request = Request {
            contact_group: group,
            read_group_fields: to_field_mask(read_group_fields),
        };

        let send = GpeopleSend::post_json(auth, url, &request)?;

        Ok(Self { send })
    }
}

impl GpeopleCoroutine for GpeopleContactGroupCreate {
    type Yield = GpeopleYield;
    type Return = Result<GpeopleSendOutput<GpeopleContactGroup>, GpeopleSendError>;

    fn resume(&mut self, arg: Option<&[u8]>) -> GpeopleCoroutineState<Self::Yield, Self::Return> {
        let out = gpeople_try!(&mut self.send, arg);
        debug!("people contact group created");
        trace!("out: {out:?}");
        GpeopleCoroutineState::Complete(Ok(out))
    }
}
