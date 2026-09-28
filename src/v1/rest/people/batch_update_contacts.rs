//! Update a batch of People contacts (`people.batchUpdateContacts`).
//!
//! Each person must carry its server-assigned resource name and the etag
//! from the latest read.
//!
//! <https://developers.google.com/people/api/rest/v1/people/batchUpdateContacts>

use alloc::{collections::BTreeMap, string::String};

use io_http::rfc6750::bearer::HttpAuthBearer;
use log::{debug, trace};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::{
    coroutine::*,
    people_try,
    v1::{
        query::to_field_mask,
        rest::people::{
            PeoplePerson, PeoplePersonField, PeoplePersonResponse, PeopleReadSourceType,
        },
        send::{PEOPLE_API_BASE, PeopleSend, PeopleSendError, PeopleSendOutput},
    },
};

/// People REST contacts batch update response, keyed by resource name.
#[derive(Debug, Clone, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PeopleContactsBatchUpdateResponse {
    /// Updated persons keyed by their resource name.
    #[serde(default)]
    pub update_result: BTreeMap<String, PeoplePersonResponse>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Request<'a> {
    contacts: BTreeMap<&'a str, &'a PeoplePerson>,
    update_mask: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    read_mask: String,
    #[serde(skip_serializing_if = "<[_]>::is_empty")]
    sources: &'a [PeopleReadSourceType],
}

/// People REST contacts batch update (200 max), keyed by each person's
/// resource name.
pub struct PeopleContactsBatchUpdate {
    send: PeopleSend<PeopleContactsBatchUpdateResponse>,
}

impl PeopleContactsBatchUpdate {
    /// Build a new contacts batch update coroutine (200 max).
    ///
    /// Each person must have a non-empty `resource_name` and a valid etag.
    /// `update_mask` must be non-empty; `read_mask` controls the response.
    pub fn new(
        auth: &HttpAuthBearer,
        persons: &[PeoplePerson],
        update_mask: &[PeoplePersonField],
        read_mask: &[PeoplePersonField],
        sources: &[PeopleReadSourceType],
    ) -> Result<Self, PeopleSendError> {
        debug!("prepare people contacts batch update");
        trace!("persons: {persons:?}");
        trace!("update_mask: {update_mask:?}");
        trace!("read_mask: {read_mask:?}");
        trace!("sources: {sources:?}");

        if persons.is_empty() {
            let err = PeopleSendError::InvalidRequest("Contacts cannot be empty".into());
            return Err(err);
        }

        if persons
            .iter()
            .any(|person| person.resource_name.trim().is_empty())
        {
            let err =
                PeopleSendError::InvalidRequest("Person resource name cannot be empty".into());
            return Err(err);
        }

        if update_mask.is_empty() {
            let err = PeopleSendError::InvalidRequest("Update mask cannot be empty".into());
            return Err(err);
        }

        let url = Url::parse(PEOPLE_API_BASE)?.join("./people:batchUpdateContacts")?;

        let request = Request {
            contacts: persons
                .iter()
                .map(|person| (person.resource_name.as_str(), person))
                .collect(),
            update_mask: to_field_mask(update_mask),
            read_mask: to_field_mask(read_mask),
            sources,
        };

        let send = PeopleSend::post_json(auth, url, &request)?;

        Ok(Self { send })
    }
}

impl PeopleCoroutine for PeopleContactsBatchUpdate {
    type Yield = PeopleYield;
    type Return = Result<PeopleSendOutput<PeopleContactsBatchUpdateResponse>, PeopleSendError>;

    fn resume(&mut self, arg: Option<&[u8]>) -> PeopleCoroutineState<Self::Yield, Self::Return> {
        let out = people_try!(&mut self.send, arg);
        debug!("people contacts batch updated");
        trace!("out: {out:?}");
        PeopleCoroutineState::Complete(Ok(out))
    }
}
