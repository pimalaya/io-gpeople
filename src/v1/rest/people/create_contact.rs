//! Create a People contact (`people.createContact`).
//!
//! <https://developers.google.com/people/api/rest/v1/people/createContact>

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

/// People REST contact creation, from a whole person resource.
pub struct PeopleContactCreate {
    send: PeopleSend<PeoplePerson>,
}

impl PeopleContactCreate {
    /// Build a new contact creation coroutine.
    ///
    /// `person_fields` is optional; when non-empty it controls which fields
    /// are populated on the returned person after creation.
    pub fn new(
        auth: &HttpAuthBearer,
        person: &PeoplePerson,
        person_fields: &[PeoplePersonField],
        sources: &[PeopleReadSourceType],
    ) -> Result<Self, PeopleSendError> {
        debug!("prepare people contact for creation");
        trace!("person: {person:?}");
        trace!("person_fields: {person_fields:?}");
        trace!("sources: {sources:?}");

        let mut url = Url::parse(PEOPLE_API_BASE)?.join("./people:createContact")?;

        {
            let mut pairs = url.query_pairs_mut();
            if !person_fields.is_empty() {
                pairs.append_pair("personFields", &to_field_mask(person_fields));
            }
            pairs.extend_pairs(to_field_pairs("sources", sources));
        }

        let send = PeopleSend::post_json(auth, url, person)?;

        Ok(Self { send })
    }
}

impl PeopleCoroutine for PeopleContactCreate {
    type Yield = PeopleYield;
    type Return = Result<PeopleSendOutput<PeoplePerson>, PeopleSendError>;

    fn resume(&mut self, arg: Option<&[u8]>) -> PeopleCoroutineState<Self::Yield, Self::Return> {
        let out = people_try!(&mut self.send, arg);
        debug!("people contact created");
        trace!("out: {out:?}");
        PeopleCoroutineState::Complete(Ok(out))
    }
}
