//! Delete a batch of People contacts (`people.batchDeleteContacts`).
//!
//! <https://developers.google.com/people/api/rest/v1/people/batchDeleteContacts>

use alloc::string::String;

use io_http::rfc6750::bearer::HttpAuthBearer;
use log::{debug, trace};
use serde::Serialize;
use url::Url;

use crate::{
    coroutine::*,
    gpeople_try,
    v1::send::{
        GPEOPLE_API_BASE, GpeopleNoResponse, GpeopleSend, GpeopleSendError, GpeopleSendOutput,
    },
};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Request<'a> {
    resource_names: &'a [String],
}

/// People REST contacts batch deletion (500 max), by full resource names.
pub struct GpeopleContactsBatchDelete {
    send: GpeopleSend<GpeopleNoResponse>,
}

impl GpeopleContactsBatchDelete {
    /// Build a new contacts batch deletion coroutine (500 max).
    ///
    /// `resource_names` must be non-empty; each entry must identify an
    /// existing contact resource.
    pub fn new(auth: &HttpAuthBearer, resource_names: &[String]) -> Result<Self, GpeopleSendError> {
        debug!("prepare people contacts batch deletion");
        trace!("resource_names: {resource_names:?}");

        if resource_names.is_empty() {
            let err = GpeopleSendError::InvalidRequest("Resource names cannot be empty".into());
            return Err(err);
        }

        let url = Url::parse(GPEOPLE_API_BASE)?.join("./people:batchDeleteContacts")?;
        let send = GpeopleSend::post_json(auth, url, &Request { resource_names })?;

        Ok(Self { send })
    }
}

impl GpeopleCoroutine for GpeopleContactsBatchDelete {
    type Yield = GpeopleYield;
    type Return = Result<GpeopleSendOutput<GpeopleNoResponse>, GpeopleSendError>;

    fn resume(&mut self, arg: Option<&[u8]>) -> GpeopleCoroutineState<Self::Yield, Self::Return> {
        let out = gpeople_try!(&mut self.send, arg);
        debug!("people contacts batch deleted");
        trace!("out: {out:?}");
        GpeopleCoroutineState::Complete(Ok(out))
    }
}
