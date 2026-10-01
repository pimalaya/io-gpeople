//! List the authenticated user's "Other contacts"
//! (`otherContacts.list`).
//!
//! Set `request_sync_token` on the first full listing, then pass the
//! returned `next_sync_token` back as `sync_token` to fetch incremental
//! changes.
//!
//! <https://developers.google.com/people/api/rest/v1/otherContacts/list>

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
        rest::people::{GpeoplePerson, GpeoplePersonField, GpeopleReadSourceType},
        send::{GPEOPLE_API_BASE, GpeopleSend, GpeopleSendError, GpeopleSendOutput},
    },
};

/// Optional query parameters for listing "Other contacts"
/// (`otherContacts.list`).
#[derive(Debug, Clone, Default, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GpeopleOtherContactsListParams<'a> {
    /// Maximum number of contacts to return per page (1–1000; default 100).
    pub page_size: Option<u32>,
    /// Page token received from a previous response's `next_page_token`.
    pub page_token: Option<&'a str>,
    /// When `true`, a `next_sync_token` is included in the last response
    /// page for use in subsequent incremental sync requests.
    #[serde(skip_serializing_if = "crate::v1::query::is_false")]
    pub request_sync_token: bool,
    /// Token from a prior `next_sync_token`; causes the response to
    /// return only changes since the previous full sync.
    pub sync_token: Option<&'a str>,
    /// Data sources to include in the response.
    pub sources: &'a [GpeopleReadSourceType],
}

/// People REST "Other contacts" listing response (one page of persons).
#[derive(Debug, Clone, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GpeopleOtherContactsListResponse {
    /// Persons returned for this page of results.
    #[serde(default)]
    pub other_contacts: Vec<GpeoplePerson>,
    /// Token to retrieve the next page; absent on the last page.
    #[serde(default)]
    pub next_page_token: Option<String>,
    /// Token to use in a future incremental sync; present only on the
    /// last page when `request_sync_token` was set.
    #[serde(default)]
    pub next_sync_token: Option<String>,
    /// Total number of contacts in the list, without page filtering.
    #[serde(default)]
    pub total_size: Option<u32>,
}

/// People REST "Other contacts" listing, wrapping a page of persons.
pub struct GpeopleOtherContactsList {
    send: GpeopleSend<GpeopleOtherContactsListResponse>,
}

impl GpeopleOtherContactsList {
    /// Build a coroutine that lists "Other contacts" with the given
    /// `read_mask` fields and optional query `params`.
    pub fn new(
        auth: &HttpAuthBearer,
        read_mask: &[GpeoplePersonField],
        params: &GpeopleOtherContactsListParams,
    ) -> Result<Self, GpeopleSendError> {
        debug!("prepare people other contacts listing");
        trace!("read_mask: {read_mask:?}");
        trace!("params: {params:?}");

        if read_mask.is_empty() {
            let err = GpeopleSendError::InvalidRequest("Read mask cannot be empty".into());
            return Err(err);
        }

        let mut url = Url::parse(GPEOPLE_API_BASE)?.join("otherContacts")?;

        {
            let mut pairs = url.query_pairs_mut();
            pairs.append_pair("readMask", &to_field_mask(read_mask));
            pairs.extend_pairs(to_query_pairs(params));
        }

        let send = GpeopleSend::get(auth, url);

        Ok(Self { send })
    }
}

impl GpeopleCoroutine for GpeopleOtherContactsList {
    type Yield = GpeopleYield;
    type Return = Result<GpeopleSendOutput<GpeopleOtherContactsListResponse>, GpeopleSendError>;

    /// Drive the HTTP exchange one step; yields I/O wants until the
    /// response is fully received, then completes with the parsed page.
    fn resume(&mut self, arg: Option<&[u8]>) -> GpeopleCoroutineState<Self::Yield, Self::Return> {
        let out = gpeople_try!(&mut self.send, arg);
        debug!("people other contacts listed");
        trace!("out: {out:?}");
        GpeopleCoroutineState::Complete(Ok(out))
    }
}
