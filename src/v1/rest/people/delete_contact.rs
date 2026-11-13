//! Delete a People contact (`people.deleteContact`).
//!
//! <https://developers.google.com/people/api/rest/v1/people/deleteContact>

use io_http::rfc6750::bearer::HttpAuthBearer;
use log::{debug, trace};

use crate::{
    coroutine::*,
    gpeople_try,
    v1::send::{GpeopleNoResponse, GpeopleSend, GpeopleSendError, GpeopleSendOutput, resource_url},
};

/// People REST contact deletion, by full resource name.
pub struct GpeopleContactDelete {
    send: GpeopleSend<GpeopleNoResponse>,
}

impl GpeopleContactDelete {
    /// Build a new contact deletion coroutine.
    ///
    /// `resource_name` must be non-empty and identify an existing contact.
    pub fn new(auth: &HttpAuthBearer, resource_name: &str) -> Result<Self, GpeopleSendError> {
        debug!("prepare people contact for deletion");
        trace!("resource_name: {resource_name:?}");

        if resource_name.trim().is_empty() {
            let err =
                GpeopleSendError::InvalidRequest("Person resource name cannot be empty".into());
            return Err(err);
        }

        let url = resource_url("people", resource_name, ":deleteContact")?;
        let send = GpeopleSend::delete(auth, url);

        Ok(Self { send })
    }
}

impl GpeopleCoroutine for GpeopleContactDelete {
    type Yield = GpeopleYield;
    type Return = Result<GpeopleSendOutput<GpeopleNoResponse>, GpeopleSendError>;

    fn resume(&mut self, arg: Option<&[u8]>) -> GpeopleCoroutineState<Self::Yield, Self::Return> {
        let out = gpeople_try!(&mut self.send, arg);
        debug!("people contact deleted");
        trace!("out: {out:?}");
        GpeopleCoroutineState::Complete(Ok(out))
    }
}
