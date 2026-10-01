//! List the People contact groups (`contactGroups.list`).
//!
//! <https://developers.google.com/people/api/rest/v1/contactGroups/list>

use alloc::{string::String, vec::Vec};

use io_http::rfc6750::bearer::HttpAuthBearer;
use log::{debug, trace};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::{
    coroutine::*,
    gpeople_try,
    v1::{
        query::{to_field_mask, to_query_pairs},
        rest::contact_groups::{GpeopleContactGroup, GpeopleGroupField},
        send::{GPEOPLE_API_BASE, GpeopleSend, GpeopleSendError, GpeopleSendOutput},
    },
};

/// Optional query parameters for listing contact groups
/// (`contactGroups.list`).
#[derive(Debug, Clone, Default, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GpeopleContactGroupsListParams<'a> {
    /// Maximum number of groups to return per page (server default: 30,
    /// max: 1000).
    pub page_size: Option<u32>,
    /// Token from a previous response to retrieve the next page.
    pub page_token: Option<&'a str>,
    /// Token from a previous response to perform incremental sync.
    pub sync_token: Option<&'a str>,
}

/// People REST contact groups listing response (one page of groups).
#[derive(Debug, Clone, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GpeopleContactGroupsListResponse {
    /// Contact groups returned for this page of results.
    #[serde(default)]
    pub contact_groups: Vec<GpeopleContactGroup>,
    /// Token to pass as `page_token` to retrieve the next page.
    #[serde(default)]
    pub next_page_token: Option<String>,
    /// Token to pass as `sync_token` on the next incremental sync.
    #[serde(default)]
    pub next_sync_token: Option<String>,
    /// Total number of groups across all pages.
    #[serde(default)]
    pub total_items: Option<u32>,
}

/// People REST contact groups listing, wrapping a page of groups.
pub struct GpeopleContactGroupsList {
    send: GpeopleSend<GpeopleContactGroupsListResponse>,
}

impl GpeopleContactGroupsList {
    /// Build a contact-groups listing coroutine for the given field mask
    /// and optional pagination/sync parameters.
    pub fn new(
        auth: &HttpAuthBearer,
        group_fields: &[GpeopleGroupField],
        params: &GpeopleContactGroupsListParams,
    ) -> Result<Self, GpeopleSendError> {
        debug!("prepare people contact groups listing");
        trace!("group_fields: {group_fields:?}");
        trace!("params: {params:?}");

        let mut url = Url::parse(GPEOPLE_API_BASE)?.join("contactGroups")?;

        {
            let mut pairs = url.query_pairs_mut();
            if !group_fields.is_empty() {
                pairs.append_pair("groupFields", &to_field_mask(group_fields));
            }
            pairs.extend_pairs(to_query_pairs(params));
        }

        let send = GpeopleSend::get(auth, url);

        Ok(Self { send })
    }
}

impl GpeopleCoroutine for GpeopleContactGroupsList {
    type Yield = GpeopleYield;
    type Return = Result<GpeopleSendOutput<GpeopleContactGroupsListResponse>, GpeopleSendError>;

    fn resume(&mut self, arg: Option<&[u8]>) -> GpeopleCoroutineState<Self::Yield, Self::Return> {
        let out = gpeople_try!(&mut self.send, arg);
        debug!("people contact groups listed");
        trace!("out: {out:?}");
        GpeopleCoroutineState::Complete(Ok(out))
    }
}
