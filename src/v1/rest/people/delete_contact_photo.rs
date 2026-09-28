//! Delete a People contact's photo (`people.deleteContactPhoto`).
//!
//! <https://developers.google.com/people/api/rest/v1/people/deleteContactPhoto>

use alloc::format;

use io_http::rfc6750::bearer::HttpAuthBearer;
use log::{debug, trace};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::{
    coroutine::*,
    people_try,
    v1::{
        query::{to_field_mask, to_field_pairs},
        rest::people::{PeoplePerson, PeoplePersonField, PeopleReadSourceType},
        send::{PEOPLE_API_BASE, PeopleSend, PeopleSendError, PeopleSendOutput},
    },
};

/// People REST contact photo deletion response (the person after the
/// mutation, when a person fields mask was given).
#[derive(Debug, Clone, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PeopleContactPhotoDeleteResponse {
    /// The person after photo removal, populated when `person_fields` was
    /// given.
    #[serde(default)]
    pub person: Option<PeoplePerson>,
}

/// People REST contact photo deletion, by full resource name.
pub struct PeopleContactPhotoDelete {
    send: PeopleSend<PeopleContactPhotoDeleteResponse>,
}

impl PeopleContactPhotoDelete {
    /// Build a new contact photo deletion coroutine.
    ///
    /// `person_fields` is optional; when non-empty the response includes the
    /// updated person with the specified fields populated.
    pub fn new(
        auth: &HttpAuthBearer,
        resource_name: &str,
        person_fields: &[PeoplePersonField],
        sources: &[PeopleReadSourceType],
    ) -> Result<Self, PeopleSendError> {
        debug!("prepare people contact photo for deletion");
        trace!("resource_name: {resource_name:?}");
        trace!("person_fields: {person_fields:?}");
        trace!("sources: {sources:?}");

        if resource_name.trim().is_empty() {
            let err =
                PeopleSendError::InvalidRequest("Person resource name cannot be empty".into());
            return Err(err);
        }

        let mut url =
            Url::parse(PEOPLE_API_BASE)?.join(&format!("{resource_name}:deleteContactPhoto"))?;

        {
            let mut pairs = url.query_pairs_mut();
            if !person_fields.is_empty() {
                pairs.append_pair("personFields", &to_field_mask(person_fields));
            }
            pairs.extend_pairs(to_field_pairs("sources", sources));
        }

        let send = PeopleSend::delete(auth, url);

        Ok(Self { send })
    }
}

impl PeopleCoroutine for PeopleContactPhotoDelete {
    type Yield = PeopleYield;
    type Return = Result<PeopleSendOutput<PeopleContactPhotoDeleteResponse>, PeopleSendError>;

    fn resume(&mut self, arg: Option<&[u8]>) -> PeopleCoroutineState<Self::Yield, Self::Return> {
        let out = people_try!(&mut self.send, arg);
        debug!("people contact photo deleted");
        trace!("out: {out:?}");
        PeopleCoroutineState::Complete(Ok(out))
    }
}
