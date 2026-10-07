//! HTTP/JSON transport every People coroutine delegates to: builds the
//! authorized request and parses the JSON response, or the People error
//! envelope on failure.
//!
//! People API reference: <https://developers.google.com/people/api/rest>.

use core::{fmt, marker::PhantomData};

use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};

use io_http::{
    coroutine::{HttpCoroutine, HttpCoroutineState},
    rfc6750::bearer::HttpAuthBearer,
    rfc9110::{
        request::HttpRequest,
        send::{HttpSendOutput, HttpSendYield},
    },
    rfc9112::send::{Http11Send, Http11SendError},
};
use log::trace;
use serde::{Deserialize, Deserializer, Serialize, de::DeserializeOwned};
use thiserror::Error;
use url::Url;

use crate::coroutine::{GpeopleCoroutine, GpeopleCoroutineState, GpeopleYield};

/// Base URL for the Google People API v1.
pub const GPEOPLE_API_BASE: &str = "https://people.googleapis.com/v1/";

/// Placeholder response type for People API operations that return no body
/// (e.g. DELETE).
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq)]
pub struct GpeopleNoResponse;

impl<'de> Deserialize<'de> for GpeopleNoResponse {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let _ = serde::de::IgnoredAny::deserialize(deserializer)?;
        Ok(Self)
    }
}

/// Errors that can occur while sending a People API HTTP request.
#[derive(Debug, Error)]
pub enum GpeopleSendError {
    /// The underlying HTTP/1.1 send coroutine failed.
    #[error("People HTTP request failed: {0}")]
    Send(#[from] Http11SendError),
    /// The request body could not be serialized to JSON.
    #[error("People request serialization failed: {0}")]
    SerializeRequest(#[source] serde_json::Error),
    /// The response body could not be deserialized from JSON.
    #[error("People response parsing failed: {0}")]
    ParseResponse(#[source] serde_json::Error),
    /// A URL passed to a coroutine constructor could not be parsed.
    #[error("People URL parsing failed: {0}")]
    ParseUrl(#[from] url::ParseError),
    /// A request parameter was rejected before the HTTP round-trip.
    #[error("Invalid People request: {0}")]
    InvalidRequest(String),
    /// The API returned a non-2xx status with an error envelope.
    #[error("{0}")]
    Api(GpeopleApiError),
    /// The server issued a redirect, which the client never follows.
    #[error("People server returned an unexpected redirect")]
    UnexpectedRedirect,
}

impl GpeopleSendError {
    /// Return the API error when People answered with a non-2xx status.
    pub fn api(&self) -> Option<&GpeopleApiError> {
        match self {
            Self::Api(err) => Some(err),
            _ => None,
        }
    }

    /// Return the HTTP status code if this is an [`GpeopleSendError::Api`]
    /// error, otherwise `None`.
    pub fn status(&self) -> Option<u16> {
        self.api().map(|err| err.status)
    }

    /// Return `true` if the error is an API error worth retrying later
    /// ([`GpeopleApiError::is_retryable`]).
    pub fn is_retryable(&self) -> bool {
        self.api().is_some_and(GpeopleApiError::is_retryable)
    }

    /// Return `true` if the error is a rate limit answer
    /// ([`GpeopleApiError::is_rate_limited`]).
    pub fn is_rate_limited(&self) -> bool {
        self.api().is_some_and(GpeopleApiError::is_rate_limited)
    }

    /// Return `true` if the target of the request does not exist (404).
    pub fn is_not_found(&self) -> bool {
        self.status() == Some(404)
    }

    /// Return `true` if a sync token was refused as expired
    /// ([`GpeopleApiError::is_sync_token_expired`]): the caller must
    /// start over with a full listing, without a sync token.
    pub fn is_sync_token_expired(&self) -> bool {
        self.api()
            .is_some_and(GpeopleApiError::is_sync_token_expired)
    }
}

/// The People error envelope, read from a non-2xx answer.
///
/// Google answers errors as
/// `{"error":{"code":400,"message":"...","errors":[{"reason":"failedPrecondition",...}],"status":"FAILED_PRECONDITION","details":[{"reason":"EXPIRED_SYNC_TOKEN",...}]}}`;
/// the reasons and statuses are kept so callers match on codes rather
/// than on the message text.
#[derive(Debug, Clone, Default, Eq, PartialEq, Error)]
#[error("People API returned HTTP {status}: {message}")]
pub struct GpeopleApiError {
    /// The effective status code: `error.code` when present, the HTTP
    /// status otherwise.
    pub status: u16,
    /// The error message, from the envelope or the raw body.
    pub message: String,
    /// The reasons of `error.errors[]`, in order, such as
    /// `rateLimitExceeded` or `failedPrecondition`.
    pub reasons: Vec<String>,
    /// The canonical status of `error.status`, such as `NOT_FOUND`,
    /// `FAILED_PRECONDITION` or `RESOURCE_EXHAUSTED`.
    pub google_status: Option<String>,
    /// The reasons of `error.details[]` (`google.rpc.ErrorInfo`), such
    /// as `EXPIRED_SYNC_TOKEN` or `RATE_LIMIT_EXCEEDED`.
    pub detail_reasons: Vec<String>,
}

impl GpeopleApiError {
    /// Read the People error envelope out of a non-2xx answer, falling
    /// back to the raw body as message.
    pub fn parse(http_status: u16, body: &[u8]) -> Self {
        let (status, message) = parse_api_error(http_status, body);
        let mut err = Self {
            status,
            message,
            ..Default::default()
        };

        if let Ok(envelope) = serde_json::from_slice::<ErrorEnvelope>(body) {
            let error = envelope.error;
            err.reasons = error.errors.into_iter().filter_map(|e| e.reason).collect();
            err.google_status = error.status;
            err.detail_reasons = error.details.into_iter().filter_map(|d| d.reason).collect();
        }

        err
    }

    /// Return `true` if `reason` is one of [`Self::reasons`] or
    /// [`Self::detail_reasons`].
    pub fn has_reason(&self, reason: &str) -> bool {
        self.reasons
            .iter()
            .chain(&self.detail_reasons)
            .any(|r| r == reason)
    }

    /// Return `true` if Google throttled the request: a 429, a
    /// `RESOURCE_EXHAUSTED` status, or a 403 whose reason is
    /// `rateLimitExceeded`, `userRateLimitExceeded`, `quotaExceeded` or
    /// `RATE_LIMIT_EXCEEDED`.
    ///
    /// The daily quota (`dailyLimitExceeded`) is not a rate limit:
    /// waiting a minute does not lift it.
    pub fn is_rate_limited(&self) -> bool {
        if self.status == 429 || self.google_status.as_deref() == Some("RESOURCE_EXHAUSTED") {
            return true;
        }

        self.status == 403
            && [
                "rateLimitExceeded",
                "userRateLimitExceeded",
                "quotaExceeded",
                "RATE_LIMIT_EXCEEDED",
            ]
            .iter()
            .any(|reason| self.has_reason(reason))
    }

    /// Return `true` if the request is worth sending again later: rate
    /// limited ([`Self::is_rate_limited`]) or a transient 5xx.
    pub fn is_retryable(&self) -> bool {
        self.is_rate_limited() || matches!(self.status, 500 | 502 | 503 | 504)
    }

    /// Return `true` if the target of the request does not exist (404).
    pub fn is_not_found(&self) -> bool {
        self.status == 404
    }

    /// Return `true` if a `syncToken` was refused as expired: a 410, or
    /// a 400 carrying the `EXPIRED_SYNC_TOKEN` reason. Sync tokens expire
    /// seven days after the full listing that issued them, and the caller
    /// must then list again without one.
    ///
    /// <https://developers.google.com/people/api/rest/v1/people.connections/list>
    pub fn is_sync_token_expired(&self) -> bool {
        self.status == 410 || (self.status == 400 && self.has_reason("EXPIRED_SYNC_TOKEN"))
    }
}

/// Successful output from a [`GpeopleSend`] coroutine.
#[derive(Clone, Debug)]
pub struct GpeopleSendOutput<T> {
    /// The deserialized API response body.
    pub response: T,
    /// Whether the server indicated the connection can be reused.
    pub keep_alive: bool,
}

/// I/O-free coroutine that sends one HTTP request to the People API and
/// deserializes the JSON response into `T`.
pub struct GpeopleSend<T> {
    state: State,
    _phantom: PhantomData<T>,
}

impl<T: DeserializeOwned> GpeopleSend<T> {
    /// Build a `GET` request coroutine for the given URL.
    pub fn get(auth: &HttpAuthBearer, url: Url) -> Self {
        Self::with_method(auth, "GET", url, None, Vec::new())
    }

    /// Build a `DELETE` request coroutine for the given URL.
    pub fn delete(auth: &HttpAuthBearer, url: Url) -> Self {
        Self::with_method(auth, "DELETE", url, None, Vec::new())
    }

    /// Build a `POST` request coroutine with a JSON-serialized body.
    pub fn post_json<B: Serialize>(
        auth: &HttpAuthBearer,
        url: Url,
        body: &B,
    ) -> Result<Self, GpeopleSendError> {
        let body = serde_json::to_vec(body).map_err(GpeopleSendError::SerializeRequest)?;
        Ok(Self::with_method(
            auth,
            "POST",
            url,
            Some("application/json"),
            body,
        ))
    }

    /// Build a `PUT` request coroutine with a JSON-serialized body.
    pub fn put_json<B: Serialize>(
        auth: &HttpAuthBearer,
        url: Url,
        body: &B,
    ) -> Result<Self, GpeopleSendError> {
        let body = serde_json::to_vec(body).map_err(GpeopleSendError::SerializeRequest)?;
        Ok(Self::with_method(
            auth,
            "PUT",
            url,
            Some("application/json"),
            body,
        ))
    }

    /// Build a `PATCH` request coroutine with a JSON-serialized body.
    pub fn patch_json<B: Serialize>(
        auth: &HttpAuthBearer,
        url: Url,
        body: &B,
    ) -> Result<Self, GpeopleSendError> {
        let body = serde_json::to_vec(body).map_err(GpeopleSendError::SerializeRequest)?;
        Ok(Self::with_method(
            auth,
            "PATCH",
            url,
            Some("application/json"),
            body,
        ))
    }

    /// Build a request coroutine for an arbitrary HTTP method, optional
    /// content type, and raw body bytes.
    pub fn with_method(
        auth: &HttpAuthBearer,
        method: &str,
        url: Url,
        content_type: Option<&str>,
        body: Vec<u8>,
    ) -> Self {
        let host = url.host_str().unwrap_or("localhost");

        let mut request = HttpRequest::get(url.clone())
            .header("Host", host)
            .header("Accept", "application/json")
            .header("Authorization", auth.to_authorization())
            .body(body);

        if let Some(content_type) = content_type {
            request = request.header("Content-Type", content_type);
        }

        request.method = method.into();

        trace!("send People {method} request to {url}");

        Self {
            state: State::Send(Http11Send::new(request)),
            _phantom: PhantomData,
        }
    }
}

impl<T: DeserializeOwned> GpeopleCoroutine for GpeopleSend<T> {
    type Yield = GpeopleYield;
    type Return = Result<GpeopleSendOutput<T>, GpeopleSendError>;

    fn resume(&mut self, arg: Option<&[u8]>) -> GpeopleCoroutineState<Self::Yield, Self::Return> {
        trace!("send: {}", self.state);
        match &mut self.state {
            State::Send(send) => match send.resume(arg) {
                HttpCoroutineState::Yielded(HttpSendYield::WantsRead) => {
                    GpeopleCoroutineState::Yielded(GpeopleYield::WantsRead)
                }
                HttpCoroutineState::Yielded(HttpSendYield::WantsWrite(bytes)) => {
                    GpeopleCoroutineState::Yielded(GpeopleYield::WantsWrite(bytes))
                }
                HttpCoroutineState::Yielded(HttpSendYield::WantsRedirect { .. }) => {
                    GpeopleCoroutineState::Complete(Err(GpeopleSendError::UnexpectedRedirect))
                }
                HttpCoroutineState::Complete(Err(err)) => {
                    GpeopleCoroutineState::Complete(Err(err.into()))
                }
                HttpCoroutineState::Complete(Ok(HttpSendOutput {
                    response,
                    keep_alive,
                    ..
                })) => {
                    if response.status.is_success() {
                        let body = if response.body.is_empty() {
                            b"null".as_slice()
                        } else {
                            response.body.as_slice()
                        };

                        match serde_json::from_slice::<T>(body) {
                            Ok(response) => {
                                GpeopleCoroutineState::Complete(Ok(GpeopleSendOutput {
                                    response,
                                    keep_alive,
                                }))
                            }
                            Err(err) => GpeopleCoroutineState::Complete(Err(
                                GpeopleSendError::ParseResponse(err),
                            )),
                        }
                    } else {
                        let err = GpeopleApiError::parse(*response.status, &response.body);
                        GpeopleCoroutineState::Complete(Err(GpeopleSendError::Api(err)))
                    }
                }
            },
        }
    }
}

enum State {
    Send(Http11Send),
}

impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Send(_) => f.write_str("send"),
        }
    }
}

#[derive(Debug, Deserialize)]
struct ErrorEnvelope {
    error: ErrorBody,
}

#[derive(Debug, Deserialize)]
struct ErrorBody {
    code: Option<u16>,
    message: Option<String>,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    errors: Vec<ErrorItem>,
    #[serde(default)]
    details: Vec<ErrorItem>,
}

/// One entry of `error.errors[]` or `error.details[]`, of which only the
/// reason is read.
#[derive(Debug, Deserialize)]
struct ErrorItem {
    #[serde(default)]
    reason: Option<String>,
}

/// Extract a `(status, message)` pair from a People API error response body,
/// falling back to the HTTP status and a generic message when parsing fails.
pub fn parse_api_error(http_status: u16, body: &[u8]) -> (u16, String) {
    if let Ok(envelope) = serde_json::from_slice::<ErrorEnvelope>(body) {
        let status = envelope.error.code.unwrap_or(http_status);
        let message = envelope
            .error
            .message
            .filter(|message| !message.trim().is_empty())
            .unwrap_or_else(|| String::from("unknown People API error"));
        return (status, message);
    }

    let message = summarize_body(&String::from_utf8_lossy(body));

    if message.is_empty() {
        (http_status, String::from("unknown People API error"))
    } else {
        (http_status, message)
    }
}

/// Maximum length of a summarised error body, in characters.
const SUMMARY_LEN: usize = 200;

/// Boil a body that is not an error envelope down to one readable line.
///
/// Google answers some errors with an HTML page rather than its JSON
/// envelope, and a whole page makes a poor error message. The page's own
/// `title` says what happened, so it wins; failing that the markup is
/// stripped, the whitespace collapsed and the result capped.
fn summarize_body(body: &str) -> String {
    let text = element_text(body, "title").unwrap_or_else(|| strip_markup(body));
    let text = text.trim();

    match text.char_indices().nth(SUMMARY_LEN) {
        Some((end, _)) => format!("{}…", &text[..end]),
        None => text.to_string(),
    }
}

/// The trimmed text of the first `<local>` element, when it has any.
fn element_text(body: &str, local: &str) -> Option<String> {
    let open = format!("<{local}>");
    let start = body.find(&open)? + open.len();
    let rest = &body[start..];
    let end = rest.find("</")?;
    let text = rest[..end].trim();

    (!text.is_empty()).then(|| text.to_string())
}

/// Drop every `<...>` tag and collapse the remaining whitespace.
fn strip_markup(body: &str) -> String {
    let mut out = String::with_capacity(body.len());
    let mut in_tag = false;

    for ch in body.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if in_tag => continue,
            ch if ch.is_whitespace() => {
                if !out.ends_with(' ') {
                    out.push(' ');
                }
            }
            ch => out.push(ch),
        }
    }

    out
}
