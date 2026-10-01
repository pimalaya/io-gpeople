//! Search the authenticated user's "Other contacts"
//! (`otherContacts.search`).
//!
//! Matches prefix phrases of the fields on a person. Recently mutated
//! data may not appear until the server-side cache is refreshed; see the
//! reference for the warmup request advice.
//!
//! <https://developers.google.com/people/api/rest/v1/otherContacts/search>

use alloc::string::ToString;

use io_http::rfc6750::bearer::HttpAuthBearer;
use log::{debug, trace};
use url::Url;

use crate::{
    coroutine::*,
    gpeople_try,
    v1::{
        query::to_field_mask,
        rest::people::{GpeoplePersonField, GpeopleSearchResponse},
        send::{GPEOPLE_API_BASE, GpeopleSend, GpeopleSendError, GpeopleSendOutput},
    },
};

/// People REST "Other contacts" search, by plain-text prefix query.
pub struct GpeopleOtherContactsSearch {
    send: GpeopleSend<GpeopleSearchResponse>,
}

impl GpeopleOtherContactsSearch {
    /// Build a coroutine that searches "Other contacts" using a
    /// plain-text prefix `query`, returning the specified `read_mask`
    /// fields and at most `page_size` results.
    pub fn new(
        auth: &HttpAuthBearer,
        query: &str,
        read_mask: &[GpeoplePersonField],
        page_size: Option<u32>,
    ) -> Result<Self, GpeopleSendError> {
        debug!("prepare people other contacts search");
        trace!("query: {query:?}");
        trace!("read_mask: {read_mask:?}");
        trace!("page_size: {page_size:?}");

        if read_mask.is_empty() {
            let err = GpeopleSendError::InvalidRequest("Read mask cannot be empty".into());
            return Err(err);
        }

        let mut url = Url::parse(GPEOPLE_API_BASE)?.join("./otherContacts:search")?;

        {
            let mut pairs = url.query_pairs_mut();
            pairs.append_pair("query", query);
            pairs.append_pair("readMask", &to_field_mask(read_mask));
            if let Some(page_size) = page_size {
                pairs.append_pair("pageSize", &page_size.to_string());
            }
        }

        let send = GpeopleSend::get(auth, url);

        Ok(Self { send })
    }
}

impl GpeopleCoroutine for GpeopleOtherContactsSearch {
    type Yield = GpeopleYield;
    type Return = Result<GpeopleSendOutput<GpeopleSearchResponse>, GpeopleSendError>;

    /// Drive the HTTP exchange one step; yields I/O wants until the
    /// response is fully received, then completes with the search results.
    fn resume(&mut self, arg: Option<&[u8]>) -> GpeopleCoroutineState<Self::Yield, Self::Return> {
        let out = gpeople_try!(&mut self.send, arg);
        debug!("people other contacts searched");
        trace!("out: {out:?}");
        GpeopleCoroutineState::Complete(Ok(out))
    }
}
