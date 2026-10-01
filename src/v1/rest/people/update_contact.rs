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
    gpeople_try,
    v1::{
        query::{to_field_mask, to_field_pairs},
        rest::people::{GpeoplePerson, GpeoplePersonField, GpeopleReadSourceType},
        send::{GPEOPLE_API_BASE, GpeopleSend, GpeopleSendError, GpeopleSendOutput},
    },
};

/// People REST contact update, replacing the masked fields.
pub struct GpeopleContactUpdate {
    send: GpeopleSend<GpeoplePerson>,
}

impl GpeopleContactUpdate {
    /// Build a new contact update coroutine.
    ///
    /// `person` must carry a non-empty `resource_name` and a valid etag from
    /// a `people.get` response. `update_person_fields` must be non-empty and
    /// lists the fields to fully replace; `person_fields` controls the
    /// response.
    pub fn new(
        auth: &HttpAuthBearer,
        person: &GpeoplePerson,
        update_person_fields: &[GpeoplePersonField],
        person_fields: &[GpeoplePersonField],
        sources: &[GpeopleReadSourceType],
    ) -> Result<Self, GpeopleSendError> {
        debug!("prepare people contact for update");
        trace!("person: {person:?}");
        trace!("update_person_fields: {update_person_fields:?}");
        trace!("person_fields: {person_fields:?}");
        trace!("sources: {sources:?}");

        if person.resource_name.trim().is_empty() {
            let err =
                GpeopleSendError::InvalidRequest("Person resource name cannot be empty".into());
            return Err(err);
        }

        if update_person_fields.is_empty() {
            let err =
                GpeopleSendError::InvalidRequest("Update person fields cannot be empty".into());
            return Err(err);
        }

        let mut url = Url::parse(GPEOPLE_API_BASE)?
            .join(&format!("{}:updateContact", person.resource_name))?;

        {
            let mut pairs = url.query_pairs_mut();
            pairs.append_pair("updatePersonFields", &to_field_mask(update_person_fields));
            if !person_fields.is_empty() {
                pairs.append_pair("personFields", &to_field_mask(person_fields));
            }
            pairs.extend_pairs(to_field_pairs("sources", sources));
        }

        let send = GpeopleSend::patch_json(auth, url, person)?;

        Ok(Self { send })
    }
}

impl GpeopleCoroutine for GpeopleContactUpdate {
    type Yield = GpeopleYield;
    type Return = Result<GpeopleSendOutput<GpeoplePerson>, GpeopleSendError>;

    fn resume(&mut self, arg: Option<&[u8]>) -> GpeopleCoroutineState<Self::Yield, Self::Return> {
        let out = gpeople_try!(&mut self.send, arg);
        debug!("people contact updated");
        trace!("out: {out:?}");
        GpeopleCoroutineState::Complete(Ok(out))
    }
}
