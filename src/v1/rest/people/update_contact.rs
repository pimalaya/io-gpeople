//! Update a People contact (`people.updateContact`).
//!
//! The person must carry its server-assigned resource name and an etag;
//! every field named in the update mask is fully replaced.
//!
//! The etag must come from a `people.get` or a prior create/update
//! response: a `connections.list` etag is rejected with `HTTP 400`, so the
//! first edit after a pull needs the etag re-read. See `docs/etags.md`.
//!
//! <https://developers.google.com/people/api/rest/v1/people/updateContact>

use alloc::format;

use io_http::rfc6750::bearer::HttpAuthBearer;
use log::{debug, trace};
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

/// People REST contact update, replacing the masked fields.
pub struct PeopleContactUpdate {
    send: PeopleSend<PeoplePerson>,
}

impl PeopleContactUpdate {
    /// Build a new contact update coroutine.
    ///
    /// `person` must carry a non-empty `resource_name` and a valid etag from
    /// a `people.get` response. `update_person_fields` must be non-empty and
    /// lists the fields to fully replace; `person_fields` controls the
    /// response.
    pub fn new(
        auth: &HttpAuthBearer,
        person: &PeoplePerson,
        update_person_fields: &[PeoplePersonField],
        person_fields: &[PeoplePersonField],
        sources: &[PeopleReadSourceType],
    ) -> Result<Self, PeopleSendError> {
        debug!("prepare people contact for update");
        trace!("person: {person:?}");
        trace!("update_person_fields: {update_person_fields:?}");
        trace!("person_fields: {person_fields:?}");
        trace!("sources: {sources:?}");

        if person.resource_name.trim().is_empty() {
            let err =
                PeopleSendError::InvalidRequest("Person resource name cannot be empty".into());
            return Err(err);
        }

        if update_person_fields.is_empty() {
            let err =
                PeopleSendError::InvalidRequest("Update person fields cannot be empty".into());
            return Err(err);
        }

        let mut url = Url::parse(PEOPLE_API_BASE)?
            .join(&format!("{}:updateContact", person.resource_name))?;

        {
            let mut pairs = url.query_pairs_mut();
            pairs.append_pair("updatePersonFields", &to_field_mask(update_person_fields));
            if !person_fields.is_empty() {
                pairs.append_pair("personFields", &to_field_mask(person_fields));
            }
            pairs.extend_pairs(to_field_pairs("sources", sources));
        }

        let send = PeopleSend::patch_json(auth, url, person)?;

        Ok(Self { send })
    }
}

impl PeopleCoroutine for PeopleContactUpdate {
    type Yield = PeopleYield;
    type Return = Result<PeopleSendOutput<PeoplePerson>, PeopleSendError>;

    fn resume(&mut self, arg: Option<&[u8]>) -> PeopleCoroutineState<Self::Yield, Self::Return> {
        let out = people_try!(&mut self.send, arg);
        debug!("people contact updated");
        trace!("out: {out:?}");
        PeopleCoroutineState::Complete(Ok(out))
    }
}
