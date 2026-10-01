//! Search the domain directory people (`people.searchDirectoryPeople`).
//!
//! Google Workspace only: requires a domain-wide directory.
//!
//! <https://developers.google.com/people/api/rest/v1/people/searchDirectoryPeople>

use alloc::{string::String, vec::Vec};

use io_http::rfc6750::bearer::HttpAuthBearer;
use log::{debug, trace};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::{
    coroutine::*,
    gpeople_try,
    v1::{
        query::{to_field_mask, to_field_pairs, to_query_pairs},
        rest::people::{
            GpeopleDirectoryMergeSourceType, GpeopleDirectorySourceType, GpeoplePerson,
            GpeoplePersonField,
        },
        send::{GPEOPLE_API_BASE, GpeopleSend, GpeopleSendError, GpeopleSendOutput},
    },
};

/// Optional query parameters for searching directory people
/// (`people.searchDirectoryPeople`).
#[derive(Debug, Clone, Default, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GpeopleDirectorySearchParams<'a> {
    /// Additional person data to merge into each directory entry.
    pub merge_sources: &'a [GpeopleDirectoryMergeSourceType],
    /// Maximum number of people to return per page (max 100).
    pub page_size: Option<u32>,
    /// Page token from a previous response, used to retrieve the next page.
    pub page_token: Option<&'a str>,
}

/// People REST directory people search response (one page of persons).
#[derive(Debug, Clone, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GpeopleDirectorySearchResponse {
    /// Directory persons matching the query for this page.
    #[serde(default)]
    pub people: Vec<GpeoplePerson>,
    /// Token for retrieving the next page; absent on the final page.
    #[serde(default)]
    pub next_page_token: Option<String>,
    /// Total number of matching people across all pages.
    #[serde(default)]
    pub total_size: Option<u32>,
}

/// People REST directory people search, by plain-text prefix query.
pub struct GpeopleDirectorySearch {
    send: GpeopleSend<GpeopleDirectorySearchResponse>,
}

impl GpeopleDirectorySearch {
    /// Build a new directory people search coroutine.
    ///
    /// Both `read_mask` and `sources` must be non-empty. `query` is a
    /// plain-text prefix string matched against the directory entries.
    pub fn new(
        auth: &HttpAuthBearer,
        query: &str,
        read_mask: &[GpeoplePersonField],
        sources: &[GpeopleDirectorySourceType],
        params: &GpeopleDirectorySearchParams,
    ) -> Result<Self, GpeopleSendError> {
        debug!("prepare people directory search");
        trace!("query: {query:?}");
        trace!("read_mask: {read_mask:?}");
        trace!("sources: {sources:?}");
        trace!("params: {params:?}");

        if read_mask.is_empty() {
            let err = GpeopleSendError::InvalidRequest("Read mask cannot be empty".into());
            return Err(err);
        }

        if sources.is_empty() {
            let err = GpeopleSendError::InvalidRequest("Directory sources cannot be empty".into());
            return Err(err);
        }

        let mut url = Url::parse(GPEOPLE_API_BASE)?.join("./people:searchDirectoryPeople")?;

        {
            let mut pairs = url.query_pairs_mut();
            pairs.append_pair("query", query);
            pairs.append_pair("readMask", &to_field_mask(read_mask));
            pairs.extend_pairs(to_field_pairs("sources", sources));
            pairs.extend_pairs(to_query_pairs(params));
        }

        let send = GpeopleSend::get(auth, url);

        Ok(Self { send })
    }
}

impl GpeopleCoroutine for GpeopleDirectorySearch {
    type Yield = GpeopleYield;
    type Return = Result<GpeopleSendOutput<GpeopleDirectorySearchResponse>, GpeopleSendError>;

    fn resume(&mut self, arg: Option<&[u8]>) -> GpeopleCoroutineState<Self::Yield, Self::Return> {
        let out = gpeople_try!(&mut self.send, arg);
        debug!("people directory searched");
        trace!("out: {out:?}");
        GpeopleCoroutineState::Complete(Ok(out))
    }
}
