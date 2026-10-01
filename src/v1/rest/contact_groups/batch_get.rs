//! Get a batch of People contact groups (`contactGroups.batchGet`).
//!
//! <https://developers.google.com/people/api/rest/v1/contactGroups/batchGet>

use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use io_http::rfc6750::bearer::HttpAuthBearer;
use log::{debug, trace};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::{
    coroutine::*,
    gpeople_try,
    v1::{
        query::to_field_mask,
        rest::contact_groups::{GpeopleContactGroupResponse, GpeopleGroupField},
        send::{GPEOPLE_API_BASE, GpeopleSend, GpeopleSendError, GpeopleSendOutput},
    },
};

/// People REST contact groups batch retrieval response (one entry per
/// requested resource name).
#[derive(Debug, Clone, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GpeopleContactGroupsBatchGetResponse {
    /// One entry per requested resource name, in the same order.
    #[serde(default)]
    pub responses: Vec<GpeopleContactGroupResponse>,
}

/// People REST contact groups batch retrieval, by full resource names
/// (200 max).
pub struct GpeopleContactGroupsBatchGet {
    send: GpeopleSend<GpeopleContactGroupsBatchGetResponse>,
}

impl GpeopleContactGroupsBatchGet {
    /// Build a batch contact-group retrieval coroutine for up to 200
    /// resource names, an optional member cap, and a field mask.
    pub fn new(
        auth: &HttpAuthBearer,
        resource_names: &[String],
        max_members: Option<u32>,
        group_fields: &[GpeopleGroupField],
    ) -> Result<Self, GpeopleSendError> {
        debug!("prepare people contact groups batch retrieval");
        trace!("resource_names: {resource_names:?}");
        trace!("max_members: {max_members:?}");
        trace!("group_fields: {group_fields:?}");

        if resource_names.is_empty() {
            let err = GpeopleSendError::InvalidRequest("Resource names cannot be empty".into());
            return Err(err);
        }

        let mut url = Url::parse(GPEOPLE_API_BASE)?.join("./contactGroups:batchGet")?;

        {
            let mut pairs = url.query_pairs_mut();
            for resource_name in resource_names {
                pairs.append_pair("resourceNames", resource_name);
            }
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

impl GpeopleCoroutine for GpeopleContactGroupsBatchGet {
    type Yield = GpeopleYield;
    type Return = Result<GpeopleSendOutput<GpeopleContactGroupsBatchGetResponse>, GpeopleSendError>;

    fn resume(&mut self, arg: Option<&[u8]>) -> GpeopleCoroutineState<Self::Yield, Self::Return> {
        let out = gpeople_try!(&mut self.send, arg);
        debug!("people contact groups batch retrieved");
        trace!("out: {out:?}");
        GpeopleCoroutineState::Complete(Ok(out))
    }
}
