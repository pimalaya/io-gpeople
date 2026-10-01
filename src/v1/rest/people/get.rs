//! Get a People person (`people.get`).
//!
//! <https://developers.google.com/people/api/rest/v1/people/get>

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

/// People REST person retrieval, by full resource name.
pub struct GpeoplePersonGet {
    send: GpeopleSend<GpeoplePerson>,
}

impl GpeoplePersonGet {
    /// Build a new person retrieval coroutine.
    ///
    /// `resource_name` (e.g. `people/me`) and `person_fields` must both be
    /// non-empty.
    pub fn new(
        auth: &HttpAuthBearer,
        resource_name: &str,
        person_fields: &[GpeoplePersonField],
        sources: &[GpeopleReadSourceType],
    ) -> Result<Self, GpeopleSendError> {
        debug!("prepare people person retrieval");
        trace!("resource_name: {resource_name:?}");
        trace!("person_fields: {person_fields:?}");
        trace!("sources: {sources:?}");

        if resource_name.trim().is_empty() {
            let err =
                GpeopleSendError::InvalidRequest("Person resource name cannot be empty".into());
            return Err(err);
        }

        if person_fields.is_empty() {
            let err = GpeopleSendError::InvalidRequest("Person fields cannot be empty".into());
            return Err(err);
        }

        let mut url = Url::parse(GPEOPLE_API_BASE)?.join(resource_name)?;

        {
            let mut pairs = url.query_pairs_mut();
            pairs.append_pair("personFields", &to_field_mask(person_fields));
            pairs.extend_pairs(to_field_pairs("sources", sources));
        }

        let send = GpeopleSend::get(auth, url);

        Ok(Self { send })
    }
}

impl GpeopleCoroutine for GpeoplePersonGet {
    type Yield = GpeopleYield;
    type Return = Result<GpeopleSendOutput<GpeoplePerson>, GpeopleSendError>;

    fn resume(&mut self, arg: Option<&[u8]>) -> GpeopleCoroutineState<Self::Yield, Self::Return> {
        let out = gpeople_try!(&mut self.send, arg);
        debug!("people person retrieved");
        trace!("out: {out:?}");
        GpeopleCoroutineState::Complete(Ok(out))
    }
}
