//! Get a People contact group (`contactGroups.get`).
//!
//! <https://developers.google.com/people/api/rest/v1/contactGroups/get>

use alloc::string::ToString;

use io_http::rfc6750::bearer::HttpAuthBearer;
use log::{debug, trace};
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

/// People REST contact group retrieval, by full resource name.
pub struct GpeopleContactGroupGet {
    send: GpeopleSend<GpeopleContactGroup>,
}

impl GpeopleContactGroupGet {
    /// Build a single contact-group retrieval coroutine for the given
    /// resource name, optional member cap, and field mask.
    pub fn new(
        auth: &HttpAuthBearer,
        resource_name: &str,
        max_members: Option<u32>,
        group_fields: &[GpeopleGroupField],
    ) -> Result<Self, GpeopleSendError> {
        debug!("prepare people contact group retrieval");
        trace!("resource_name: {resource_name:?}");
        trace!("max_members: {max_members:?}");
        trace!("group_fields: {group_fields:?}");

        if resource_name.trim().is_empty() {
            let err =
                GpeopleSendError::InvalidRequest("Group resource name cannot be empty".into());
            return Err(err);
        }

        let mut url = Url::parse(GPEOPLE_API_BASE)?.join(resource_name)?;

        {
            let mut pairs = url.query_pairs_mut();
            if let Some(max_members) = max_members {
                pairs.append_pair("maxMembers", &max_members.to_string());
            }
            if !group_fields.is_empty() {
                pairs.append_pair("groupFields", &to_field_mask(group_fields));
            }
        }

        let send = GpeopleSend::get(auth, url);

        Ok(Self { send })
    }
}

impl GpeopleCoroutine for GpeopleContactGroupGet {
    type Yield = GpeopleYield;
    type Return = Result<GpeopleSendOutput<GpeopleContactGroup>, GpeopleSendError>;

    fn resume(&mut self, arg: Option<&[u8]>) -> GpeopleCoroutineState<Self::Yield, Self::Return> {
        let out = gpeople_try!(&mut self.send, arg);
        debug!("people contact group retrieved");
        trace!("out: {out:?}");
        GpeopleCoroutineState::Complete(Ok(out))
    }
}
