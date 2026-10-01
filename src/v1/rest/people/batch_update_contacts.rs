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
    gpeople_try,
    v1::{
        query::to_field_mask,
        rest::people::{
            GpeoplePerson, GpeoplePersonField, GpeoplePersonResponse, GpeopleReadSourceType,
        },
        send::{GPEOPLE_API_BASE, GpeopleSend, GpeopleSendError, GpeopleSendOutput},
    },
};

/// People REST contacts batch update response, keyed by resource name.
#[derive(Debug, Clone, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GpeopleContactsBatchUpdateResponse {
    /// Updated persons keyed by their resource name.
    #[serde(default)]
    pub update_result: BTreeMap<String, GpeoplePersonResponse>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Request<'a> {
    contacts: BTreeMap<&'a str, &'a GpeoplePerson>,
    update_mask: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    read_mask: String,
    #[serde(skip_serializing_if = "<[_]>::is_empty")]
    sources: &'a [GpeopleReadSourceType],
}

/// People REST contacts batch update (200 max), keyed by each person's
/// resource name.
pub struct GpeopleContactsBatchUpdate {
    send: GpeopleSend<GpeopleContactsBatchUpdateResponse>,
}

impl GpeopleContactsBatchUpdate {
    /// Build a new contacts batch update coroutine (200 max).
    ///
    /// Each person must have a non-empty `resource_name` and a valid etag.
    /// `update_mask` must be non-empty; `read_mask` controls the response.
    pub fn new(
        auth: &HttpAuthBearer,
        persons: &[GpeoplePerson],
        update_mask: &[GpeoplePersonField],
        read_mask: &[GpeoplePersonField],
        sources: &[GpeopleReadSourceType],
    ) -> Result<Self, GpeopleSendError> {
        debug!("prepare people contacts batch update");
        trace!("persons: {persons:?}");
        trace!("update_mask: {update_mask:?}");
        trace!("read_mask: {read_mask:?}");
        trace!("sources: {sources:?}");

        if persons.is_empty() {
            let err = GpeopleSendError::InvalidRequest("Contacts cannot be empty".into());
            return Err(err);
        }

        if persons
            .iter()
            .any(|person| person.resource_name.trim().is_empty())
        {
            let err =
                GpeopleSendError::InvalidRequest("Person resource name cannot be empty".into());
            return Err(err);
        }

        if update_mask.is_empty() {
            let err = GpeopleSendError::InvalidRequest("Update mask cannot be empty".into());
            return Err(err);
        }

        let url = Url::parse(GPEOPLE_API_BASE)?.join("./people:batchUpdateContacts")?;

        let request = Request {
            contacts: persons
                .iter()
                .map(|person| (person.resource_name.as_str(), person))
                .collect(),
            update_mask: to_field_mask(update_mask),
            read_mask: to_field_mask(read_mask),
            sources,
        };

        let send = GpeopleSend::post_json(auth, url, &request)?;

        Ok(Self { send })
    }
}

impl GpeopleCoroutine for GpeopleContactsBatchUpdate {
    type Yield = GpeopleYield;
    type Return = Result<GpeopleSendOutput<GpeopleContactsBatchUpdateResponse>, GpeopleSendError>;

    fn resume(&mut self, arg: Option<&[u8]>) -> GpeopleCoroutineState<Self::Yield, Self::Return> {
        let out = gpeople_try!(&mut self.send, arg);
        debug!("people contacts batch updated");
        trace!("out: {out:?}");
        GpeopleCoroutineState::Complete(Ok(out))
    }
}
