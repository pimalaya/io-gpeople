//! Update a People contact's photo (`people.updateContactPhoto`).
//!
//! Takes the raw photo bytes (JPEG or PNG) and base64-encodes them into
//! the request.
//!
//! <https://developers.google.com/people/api/rest/v1/people/updateContactPhoto>

use alloc::{format, string::String};

use base64::{Engine, engine::general_purpose::STANDARD};
use io_http::rfc6750::bearer::HttpAuthBearer;
use log::{debug, trace};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::{
    coroutine::*,
    gpeople_try,
    v1::{
        query::to_field_mask,
        rest::people::{GpeoplePerson, GpeoplePersonField, GpeopleReadSourceType},
        send::{GPEOPLE_API_BASE, GpeopleSend, GpeopleSendError, GpeopleSendOutput},
    },
};

/// People REST contact photo update response.
#[derive(Debug, Clone, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GpeopleContactPhotoUpdateResponse {
    /// The updated person, populated when a `person_fields` mask was given.
    #[serde(default)]
    pub person: Option<GpeoplePerson>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Request<'a> {
    photo_bytes: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    person_fields: String,
    #[serde(skip_serializing_if = "<[_]>::is_empty")]
    sources: &'a [GpeopleReadSourceType],
}

/// People REST contact photo update, from raw JPEG or PNG bytes.
pub struct GpeopleContactPhotoUpdate {
    send: GpeopleSend<GpeopleContactPhotoUpdateResponse>,
}

impl GpeopleContactPhotoUpdate {
    /// Build a new contact photo update coroutine.
    ///
    /// `photo` must be non-empty JPEG or PNG bytes; they are base64-encoded
    /// into the request body. `person_fields` controls the returned person.
    pub fn new(
        auth: &HttpAuthBearer,
        resource_name: &str,
        photo: &[u8],
        person_fields: &[GpeoplePersonField],
        sources: &[GpeopleReadSourceType],
    ) -> Result<Self, GpeopleSendError> {
        debug!("prepare people contact photo for update");
        trace!("resource_name: {resource_name:?}");
        trace!("person_fields: {person_fields:?}");
        trace!("sources: {sources:?}");

        if resource_name.trim().is_empty() {
            let err =
                GpeopleSendError::InvalidRequest("Person resource name cannot be empty".into());
            return Err(err);
        }

        if photo.is_empty() {
            let err = GpeopleSendError::InvalidRequest("Photo bytes cannot be empty".into());
            return Err(err);
        }

        let url =
            Url::parse(GPEOPLE_API_BASE)?.join(&format!("{resource_name}:updateContactPhoto"))?;

        let request = Request {
            photo_bytes: STANDARD.encode(photo),
            person_fields: to_field_mask(person_fields),
            sources,
        };

        let send = GpeopleSend::patch_json(auth, url, &request)?;

        Ok(Self { send })
    }
}

impl GpeopleCoroutine for GpeopleContactPhotoUpdate {
    type Yield = GpeopleYield;
    type Return = Result<GpeopleSendOutput<GpeopleContactPhotoUpdateResponse>, GpeopleSendError>;

    fn resume(&mut self, arg: Option<&[u8]>) -> GpeopleCoroutineState<Self::Yield, Self::Return> {
        let out = gpeople_try!(&mut self.send, arg);
        debug!("people contact photo updated");
        trace!("out: {out:?}");
        GpeopleCoroutineState::Complete(Ok(out))
    }
}
