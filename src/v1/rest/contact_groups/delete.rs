//! Delete a People contact group (`contactGroups.delete`).
//!
//! <https://developers.google.com/people/api/rest/v1/contactGroups/delete>

use io_http::rfc6750::bearer::HttpAuthBearer;
use log::{debug, trace};
use url::Url;

use crate::{
    coroutine::*,
    gpeople_try,
    v1::send::{
        GPEOPLE_API_BASE, GpeopleNoResponse, GpeopleSend, GpeopleSendError, GpeopleSendOutput,
    },
};

/// People REST contact group deletion, optionally deleting its contacts
/// too.
pub struct GpeopleContactGroupDelete {
    send: GpeopleSend<GpeopleNoResponse>,
}

impl GpeopleContactGroupDelete {
    /// Build a contact-group deletion coroutine. Pass `delete_contacts:
    /// true` to also delete all contacts that belong only to this group.
    pub fn new(
        auth: &HttpAuthBearer,
        resource_name: &str,
        delete_contacts: bool,
    ) -> Result<Self, GpeopleSendError> {
        debug!("prepare people contact group for deletion");
        trace!("resource_name: {resource_name:?}");
        trace!("delete_contacts: {delete_contacts:?}");

        if resource_name.trim().is_empty() {
            let err =
                GpeopleSendError::InvalidRequest("Group resource name cannot be empty".into());
            return Err(err);
        }

        let mut url = Url::parse(GPEOPLE_API_BASE)?.join(resource_name)?;

        if delete_contacts {
            url.query_pairs_mut().append_pair("deleteContacts", "true");
        }

        let send = GpeopleSend::delete(auth, url);

        Ok(Self { send })
    }
}

impl GpeopleCoroutine for GpeopleContactGroupDelete {
    type Yield = GpeopleYield;
    type Return = Result<GpeopleSendOutput<GpeopleNoResponse>, GpeopleSendError>;

    fn resume(&mut self, arg: Option<&[u8]>) -> GpeopleCoroutineState<Self::Yield, Self::Return> {
        let out = gpeople_try!(&mut self.send, arg);
        debug!("people contact group deleted");
        trace!("out: {out:?}");
        GpeopleCoroutineState::Complete(Ok(out))
    }
}
