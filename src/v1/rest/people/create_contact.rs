//! Create a People contact (`people.createContact`).
//!
//! <https://developers.google.com/people/api/rest/v1/people/createContact>

use io_http::rfc6750::bearer::HttpAuthBearer;
use log::{debug, trace};
use url::Url;

use crate::{
    coroutine::*,
    gpeople_try,
    v1::{
        query::{to_field_mask, to_field_pairs},
        rest::people::{GpeoplePerson, GpeoplePersonField, GpeopleReadSourceType},
        send::{GPEOPLE_API_BASE, GpeopleSend, GpeopleSendError, GpeopleSendOutput},
    },
};

/// People REST contact creation, from a whole person resource.
pub struct GpeopleContactCreate {
    send: GpeopleSend<GpeoplePerson>,
}

impl GpeopleContactCreate {
    /// Build a new contact creation coroutine.
    ///
    /// `person_fields` is optional; when non-empty it controls which fields
    /// are populated on the returned person after creation.
    pub fn new(
        auth: &HttpAuthBearer,
        person: &GpeoplePerson,
        person_fields: &[GpeoplePersonField],
        sources: &[GpeopleReadSourceType],
    ) -> Result<Self, GpeopleSendError> {
        debug!("prepare people contact for creation");
        trace!("person: {person:?}");
        trace!("person_fields: {person_fields:?}");
        trace!("sources: {sources:?}");

        let mut url = Url::parse(GPEOPLE_API_BASE)?.join("./people:createContact")?;

        {
            let mut pairs = url.query_pairs_mut();
            if !person_fields.is_empty() {
                pairs.append_pair("personFields", &to_field_mask(person_fields));
            }
            pairs.extend_pairs(to_field_pairs("sources", sources));
        }

        let send = GpeopleSend::post_json(auth, url, person)?;

        Ok(Self { send })
    }
}

impl GpeopleCoroutine for GpeopleContactCreate {
    type Yield = GpeopleYield;
    type Return = Result<GpeopleSendOutput<GpeoplePerson>, GpeopleSendError>;

    fn resume(&mut self, arg: Option<&[u8]>) -> GpeopleCoroutineState<Self::Yield, Self::Return> {
        let out = gpeople_try!(&mut self.send, arg);
        debug!("people contact created");
        trace!("out: {out:?}");
        GpeopleCoroutineState::Complete(Ok(out))
    }
}
