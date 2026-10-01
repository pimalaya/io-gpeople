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
    #[error("People API returned HTTP {status}: {message}")]
    Api {
        /// HTTP status code reported by the API error envelope.
        status: u16,
        /// Human-readable error message from the API error envelope.
        message: String,
    },
    /// The server issued a redirect, which the client never follows.
    #[error("People server returned an unexpected redirect")]
    UnexpectedRedirect,
}

impl GpeopleSendError {
    /// Return the HTTP status code if this is an [`GpeopleSendError::Api`]
    /// error, otherwise `None`.
    pub fn status(&self) -> Option<u16> {
        match self {
            Self::Api { status, .. } => Some(*status),
            _ => None,
        }
    }

    /// Return `true` if the error status indicates a transient failure that
    /// may succeed on retry (429, 500, 502, 503, 504).
    pub fn is_retryable(&self) -> bool {
        matches!(self.status(), Some(429 | 500 | 502 | 503 | 504))
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
                        let (status, message) = parse_api_error(*response.status, &response.body);
                        GpeopleCoroutineState::Complete(Err(GpeopleSendError::Api {
                            status,
                            message,
                        }))
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
