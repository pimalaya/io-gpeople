//! Get a batch of People persons (`people.getBatchGet`).
//!
//! <https://developers.google.com/people/api/rest/v1/people/getBatchGet>

use alloc::{string::String, vec::Vec};

use io_http::rfc6750::bearer::HttpAuthBearer;
use log::{debug, trace};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::{
    coroutine::*,
    gpeople_try,
    v1::{
        query::{to_field_mask, to_field_pairs},
        rest::people::{GpeoplePersonField, GpeoplePersonResponse, GpeopleReadSourceType},
        send::{GPEOPLE_API_BASE, GpeopleSend, GpeopleSendError, GpeopleSendOutput},
    },
};

/// People REST persons batch retrieval response (one entry per requested
/// resource name).
#[derive(Debug, Clone, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GpeoplePersonsBatchGetResponse {
    /// One entry per requested resource name, in request order.
    #[serde(default)]
    pub responses: Vec<GpeoplePersonResponse>,
}

/// People REST persons batch retrieval, by full resource names (200 max).
pub struct GpeoplePersonsBatchGet {
    send: GpeopleSend<GpeoplePersonsBatchGetResponse>,
}

impl GpeoplePersonsBatchGet {
    /// Build a new persons batch retrieval coroutine.
    ///
    /// Both `resource_names` and `person_fields` must be non-empty; up to
    /// 200 resource names may be requested in a single call.
    pub fn new(
        auth: &HttpAuthBearer,
        resource_names: &[String],
        person_fields: &[GpeoplePersonField],
        sources: &[GpeopleReadSourceType],
    ) -> Result<Self, GpeopleSendError> {
        debug!("prepare people persons batch retrieval");
        trace!("resource_names: {resource_names:?}");
        trace!("person_fields: {person_fields:?}");
        trace!("sources: {sources:?}");

        if resource_names.is_empty() {
            let err = GpeopleSendError::InvalidRequest("Resource names cannot be empty".into());
            return Err(err);
        }

        if person_fields.is_empty() {
            let err = GpeopleSendError::InvalidRequest("Person fields cannot be empty".into());
            return Err(err);
        }

        let mut url = Url::parse(GPEOPLE_API_BASE)?.join("./people:batchGet")?;

        {
            let mut pairs = url.query_pairs_mut();
            pairs.append_pair("personFields", &to_field_mask(person_fields));
            for resource_name in resource_names {
                pairs.append_pair("resourceNames", resource_name);
            }
            pairs.extend_pairs(to_field_pairs("sources", sources));
        }

        let send = GpeopleSend::get(auth, url);

        Ok(Self { send })
    }
}

impl GpeopleCoroutine for GpeoplePersonsBatchGet {
    type Yield = GpeopleYield;
    type Return = Result<GpeopleSendOutput<GpeoplePersonsBatchGetResponse>, GpeopleSendError>;

    fn resume(&mut self, arg: Option<&[u8]>) -> GpeopleCoroutineState<Self::Yield, Self::Return> {
        let out = gpeople_try!(&mut self.send, arg);
        debug!("people persons batch retrieved");
        trace!("out: {out:?}");
        GpeopleCoroutineState::Complete(Ok(out))
    }
}
