//! Create a batch of People contacts (`people.batchCreateContacts`).
//!
//! <https://developers.google.com/people/api/rest/v1/people/batchCreateContacts>

use alloc::{string::String, vec::Vec};

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

/// People REST contacts batch creation response.
#[derive(Debug, Clone, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PeopleContactsBatchCreateResponse {
    /// Person responses for each newly created contact, in request order.
    #[serde(default)]
    pub created_people: Vec<PeoplePersonResponse>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Request<'a> {
    contacts: Vec<Contact<'a>>,
    read_mask: String,
    #[serde(skip_serializing_if = "<[_]>::is_empty")]
    sources: &'a [PeopleReadSourceType],
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Contact<'a> {
    contact_person: &'a PeoplePerson,
}

/// People REST contacts batch creation (200 max).
pub struct PeopleContactsBatchCreate {
    send: PeopleSend<PeopleContactsBatchCreateResponse>,
}

impl PeopleContactsBatchCreate {
    /// Build a new contacts batch creation coroutine (200 max).
    ///
    /// `persons` and `read_mask` must be non-empty; `read_mask` controls
    /// which fields are populated on the returned persons.
    pub fn new(
        auth: &HttpAuthBearer,
        persons: &[PeoplePerson],
        read_mask: &[PeoplePersonField],
        sources: &[PeopleReadSourceType],
    ) -> Result<Self, PeopleSendError> {
        debug!("prepare people contacts batch creation");
        trace!("persons: {persons:?}");
        trace!("read_mask: {read_mask:?}");
        trace!("sources: {sources:?}");

        if persons.is_empty() {
            let err = PeopleSendError::InvalidRequest("Contacts cannot be empty".into());
            return Err(err);
        }

        if read_mask.is_empty() {
            let err = PeopleSendError::InvalidRequest("Read mask cannot be empty".into());
            return Err(err);
        }

        let url = Url::parse(PEOPLE_API_BASE)?.join("./people:batchCreateContacts")?;

        let request = Request {
            contacts: persons
                .iter()
                .map(|contact_person| Contact { contact_person })
                .collect(),
            read_mask: to_field_mask(read_mask),
            sources,
        };

        let send = PeopleSend::post_json(auth, url, &request)?;

        Ok(Self { send })
    }
}

impl PeopleCoroutine for PeopleContactsBatchCreate {
    type Yield = PeopleYield;
    type Return = Result<PeopleSendOutput<PeopleContactsBatchCreateResponse>, PeopleSendError>;

    fn resume(&mut self, arg: Option<&[u8]>) -> PeopleCoroutineState<Self::Yield, Self::Return> {
        let out = people_try!(&mut self.send, arg);
        debug!("people contacts batch created");
        trace!("out: {out:?}");
        PeopleCoroutineState::Complete(Ok(out))
    }
}
