//! Search the People contacts (`people.searchContacts`).
//!
//! Matches prefix phrases of the fields on a contact. Recently mutated
//! data may not appear until the server-side cache is refreshed; see the
//! reference for the warmup request advice.
//!
//! <https://developers.google.com/people/api/rest/v1/people/searchContacts>

use alloc::string::ToString;

use io_http::rfc6750::bearer::HttpAuthBearer;
use log::{debug, trace};
use url::Url;

use crate::{
    coroutine::*,
    people_try,
    v1::{
        query::{to_field_mask, to_field_pairs},
        rest::people::{PeoplePersonField, PeopleReadSourceType, PeopleSearchResponse},
        send::{PEOPLE_API_BASE, PeopleSend, PeopleSendError, PeopleSendOutput},
    },
};

/// People REST contacts search, by plain-text prefix query.
pub struct PeopleContactsSearch {
    send: PeopleSend<PeopleSearchResponse>,
}

impl PeopleContactsSearch {
    /// Build a new contacts search coroutine.
    ///
    /// `read_mask` must be non-empty. `query` is matched as a prefix phrase
    /// against contact fields; results may lag recent mutations.
    pub fn new(
        auth: &HttpAuthBearer,
        query: &str,
        read_mask: &[PeoplePersonField],
        page_size: Option<u32>,
        sources: &[PeopleReadSourceType],
    ) -> Result<Self, PeopleSendError> {
        debug!("prepare people contacts search");
        trace!("query: {query:?}");
        trace!("read_mask: {read_mask:?}");
        trace!("page_size: {page_size:?}");
        trace!("sources: {sources:?}");

        if read_mask.is_empty() {
            let err = PeopleSendError::InvalidRequest("Read mask cannot be empty".into());
            return Err(err);
        }

        let mut url = Url::parse(PEOPLE_API_BASE)?.join("./people:searchContacts")?;

        {
            let mut pairs = url.query_pairs_mut();
            pairs.append_pair("query", query);
            pairs.append_pair("readMask", &to_field_mask(read_mask));
            if let Some(page_size) = page_size {
                pairs.append_pair("pageSize", &page_size.to_string());
            }
            pairs.extend_pairs(to_field_pairs("sources", sources));
        }

        let send = PeopleSend::get(auth, url);

        Ok(Self { send })
    }
}

impl PeopleCoroutine for PeopleContactsSearch {
    type Yield = PeopleYield;
    type Return = Result<PeopleSendOutput<PeopleSearchResponse>, PeopleSendError>;

    fn resume(&mut self, arg: Option<&[u8]>) -> PeopleCoroutineState<Self::Yield, Self::Return> {
        let out = people_try!(&mut self.send, arg);
        debug!("people contacts searched");
        trace!("out: {out:?}");
        PeopleCoroutineState::Complete(Ok(out))
    }
}
