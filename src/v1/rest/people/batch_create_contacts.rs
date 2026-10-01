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
    gpeople_try,
    v1::{
        query::to_field_mask,
        rest::people::{
            GpeoplePerson, GpeoplePersonField, GpeoplePersonResponse, GpeopleReadSourceType,
        },
        send::{GPEOPLE_API_BASE, GpeopleSend, GpeopleSendError, GpeopleSendOutput},
    },
};

/// People REST contacts batch creation response.
#[derive(Debug, Clone, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GpeopleContactsBatchCreateResponse {
    /// Person responses for each newly created contact, in request order.
    #[serde(default)]
    pub created_people: Vec<GpeoplePersonResponse>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Request<'a> {
    contacts: Vec<Contact<'a>>,
    read_mask: String,
    #[serde(skip_serializing_if = "<[_]>::is_empty")]
    sources: &'a [GpeopleReadSourceType],
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Contact<'a> {
    contact_person: &'a GpeoplePerson,
}

/// People REST contacts batch creation (200 max).
pub struct GpeopleContactsBatchCreate {
    send: GpeopleSend<GpeopleContactsBatchCreateResponse>,
}

impl GpeopleContactsBatchCreate {
    /// Build a new contacts batch creation coroutine (200 max).
    ///
    /// `persons` and `read_mask` must be non-empty; `read_mask` controls
    /// which fields are populated on the returned persons.
    pub fn new(
        auth: &HttpAuthBearer,
        persons: &[GpeoplePerson],
        read_mask: &[GpeoplePersonField],
        sources: &[GpeopleReadSourceType],
    ) -> Result<Self, GpeopleSendError> {
        debug!("prepare people contacts batch creation");
        trace!("persons: {persons:?}");
        trace!("read_mask: {read_mask:?}");
        trace!("sources: {sources:?}");

        if persons.is_empty() {
            let err = GpeopleSendError::InvalidRequest("Contacts cannot be empty".into());
            return Err(err);
        }

        if read_mask.is_empty() {
            let err = GpeopleSendError::InvalidRequest("Read mask cannot be empty".into());
            return Err(err);
        }

        let url = Url::parse(GPEOPLE_API_BASE)?.join("./people:batchCreateContacts")?;

        let request = Request {
            contacts: persons
                .iter()
                .map(|contact_person| Contact { contact_person })
                .collect(),
            read_mask: to_field_mask(read_mask),
            sources,
        };

        let send = GpeopleSend::post_json(auth, url, &request)?;

        Ok(Self { send })
    }
}

impl GpeopleCoroutine for GpeopleContactsBatchCreate {
    type Yield = GpeopleYield;
    type Return = Result<GpeopleSendOutput<GpeopleContactsBatchCreateResponse>, GpeopleSendError>;

    fn resume(&mut self, arg: Option<&[u8]>) -> GpeopleCoroutineState<Self::Yield, Self::Return> {
        let out = gpeople_try!(&mut self.send, arg);
        debug!("people contacts batch created");
        trace!("out: {out:?}");
        GpeopleCoroutineState::Complete(Ok(out))
    }
}
