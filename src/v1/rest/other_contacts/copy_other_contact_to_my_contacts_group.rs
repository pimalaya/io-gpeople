//! Copy an "Other contact" to the user's "myContacts" group
//! (`otherContacts.copyOtherContactToMyContactsGroup`).
//!
//! <https://developers.google.com/people/api/rest/v1/otherContacts/copyOtherContactToMyContactsGroup>

use alloc::{format, string::String};

use io_http::rfc6750::bearer::HttpAuthBearer;
use log::{debug, trace};
use serde::Serialize;
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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Request<'a> {
    copy_mask: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    read_mask: String,
    #[serde(skip_serializing_if = "<[_]>::is_empty")]
    sources: &'a [GpeopleReadSourceType],
}

/// People REST "Other contact" copy into the "myContacts" group; only
/// `emailAddresses`, `names` and `phoneNumbers` are valid in the copy
/// mask.
pub struct GpeopleOtherContactCopy {
    send: GpeopleSend<GpeoplePerson>,
}

impl GpeopleOtherContactCopy {
    /// Build a coroutine that copies the "Other contact" identified by
    /// `resource_name` into the user's "myContacts" group.
    ///
    /// `copy_mask` selects which fields to copy (limited to
    /// `emailAddresses`, `names`, `phoneNumbers`); `read_mask` and
    /// `sources` control the returned `GpeoplePerson` representation.
    pub fn new(
        auth: &HttpAuthBearer,
        resource_name: &str,
        copy_mask: &[GpeoplePersonField],
        read_mask: &[GpeoplePersonField],
        sources: &[GpeopleReadSourceType],
    ) -> Result<Self, GpeopleSendError> {
        debug!("prepare people other contact for copy");
        trace!("resource_name: {resource_name:?}");
        trace!("copy_mask: {copy_mask:?}");
        trace!("read_mask: {read_mask:?}");
        trace!("sources: {sources:?}");

        if resource_name.trim().is_empty() {
            let err =
                GpeopleSendError::InvalidRequest("Person resource name cannot be empty".into());
            return Err(err);
        }

        if copy_mask.is_empty() {
            let err = GpeopleSendError::InvalidRequest("Copy mask cannot be empty".into());
            return Err(err);
        }

        let url = Url::parse(GPEOPLE_API_BASE)?.join(&format!(
            "{resource_name}:copyOtherContactToMyContactsGroup"
        ))?;

        let request = Request {
            copy_mask: to_field_mask(copy_mask),
            read_mask: to_field_mask(read_mask),
            sources,
        };

        let send = GpeopleSend::post_json(auth, url, &request)?;

        Ok(Self { send })
    }
}

impl GpeopleCoroutine for GpeopleOtherContactCopy {
    type Yield = GpeopleYield;
    type Return = Result<GpeopleSendOutput<GpeoplePerson>, GpeopleSendError>;

    /// Drive the HTTP exchange one step; yields I/O wants until the
    /// response is fully received, then completes with the copied person.
    fn resume(&mut self, arg: Option<&[u8]>) -> GpeopleCoroutineState<Self::Yield, Self::Return> {
        let out = gpeople_try!(&mut self.send, arg);
        debug!("people other contact copied");
        trace!("out: {out:?}");
        GpeopleCoroutineState::Complete(Ok(out))
    }
}
