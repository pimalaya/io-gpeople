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
    people_try,
    v1::{
        query::to_field_mask,
        rest::people::{PeoplePerson, PeoplePersonField, PeopleReadSourceType},
        send::{PEOPLE_API_BASE, PeopleSend, PeopleSendError, PeopleSendOutput},
    },
};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Request<'a> {
    copy_mask: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    read_mask: String,
    #[serde(skip_serializing_if = "<[_]>::is_empty")]
    sources: &'a [PeopleReadSourceType],
}

/// People REST "Other contact" copy into the "myContacts" group; only
/// `emailAddresses`, `names` and `phoneNumbers` are valid in the copy
/// mask.
pub struct PeopleOtherContactCopy {
    send: PeopleSend<PeoplePerson>,
}

impl PeopleOtherContactCopy {
    /// Build a coroutine that copies the "Other contact" identified by
    /// `resource_name` into the user's "myContacts" group.
    ///
    /// `copy_mask` selects which fields to copy (limited to
    /// `emailAddresses`, `names`, `phoneNumbers`); `read_mask` and
    /// `sources` control the returned `PeoplePerson` representation.
    pub fn new(
        auth: &HttpAuthBearer,
        resource_name: &str,
        copy_mask: &[PeoplePersonField],
        read_mask: &[PeoplePersonField],
        sources: &[PeopleReadSourceType],
    ) -> Result<Self, PeopleSendError> {
        debug!("prepare people other contact for copy");
        trace!("resource_name: {resource_name:?}");
        trace!("copy_mask: {copy_mask:?}");
        trace!("read_mask: {read_mask:?}");
        trace!("sources: {sources:?}");

        if resource_name.trim().is_empty() {
            let err =
                PeopleSendError::InvalidRequest("Person resource name cannot be empty".into());
            return Err(err);
        }

        if copy_mask.is_empty() {
            let err = PeopleSendError::InvalidRequest("Copy mask cannot be empty".into());
            return Err(err);
        }

        let url = Url::parse(PEOPLE_API_BASE)?.join(&format!(
            "{resource_name}:copyOtherContactToMyContactsGroup"
        ))?;

        let request = Request {
            copy_mask: to_field_mask(copy_mask),
            read_mask: to_field_mask(read_mask),
            sources,
        };

        let send = PeopleSend::post_json(auth, url, &request)?;

        Ok(Self { send })
    }
}

impl PeopleCoroutine for PeopleOtherContactCopy {
    type Yield = PeopleYield;
    type Return = Result<PeopleSendOutput<PeoplePerson>, PeopleSendError>;

    /// Drive the HTTP exchange one step; yields I/O wants until the
    /// response is fully received, then completes with the copied person.
    fn resume(&mut self, arg: Option<&[u8]>) -> PeopleCoroutineState<Self::Yield, Self::Return> {
        let out = people_try!(&mut self.send, arg);
        debug!("people other contact copied");
        trace!("out: {out:?}");
        PeopleCoroutineState::Complete(Ok(out))
    }
}
