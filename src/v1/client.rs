//! Std-blocking People client: wraps a `Read + Write` stream plus the
//! bearer credential and runs the coroutines against
//! `people.googleapis.com`. Gated behind the `client` feature.

#[cfg(any(
    feature = "rustls-aws",
    feature = "rustls-ring",
    feature = "native-tls"
))]
use core::time::Duration;
use core::{any::Any, fmt};

use alloc::{
    boxed::Box,
    string::{String, ToString},
};
use io_http::rfc6750::bearer::HttpAuthBearer;

use std::io::{self, Read, Write};

#[cfg(any(
    feature = "rustls-aws",
    feature = "rustls-ring",
    feature = "native-tls"
))]
pub use pimalaya_stream::tls::*;
#[cfg(any(
    feature = "rustls-aws",
    feature = "rustls-ring",
    feature = "native-tls"
))]
use pimalaya_stream::{
    proxy::Proxy,
    stream::{Stream, TcpConnectOptions, TlsConnectOptions},
};
use thiserror::Error;
#[cfg(any(
    feature = "rustls-aws",
    feature = "rustls-ring",
    feature = "native-tls"
))]
use url::Url;

#[cfg(any(
    feature = "rustls-aws",
    feature = "rustls-ring",
    feature = "native-tls"
))]
use crate::v1::send::GPEOPLE_API_BASE;
use crate::{
    coroutine::*,
    v1::{
        rest::{
            contact_groups::{
                GpeopleContactGroup, GpeopleGroupField,
                create::GpeopleContactGroupCreate,
                delete::GpeopleContactGroupDelete,
                get::GpeopleContactGroupGet,
                list::{
                    GpeopleContactGroupsList, GpeopleContactGroupsListParams,
                    GpeopleContactGroupsListResponse,
                },
                members::modify::{
                    GpeopleContactGroupMembersModify, GpeopleContactGroupMembersModifyResponse,
                },
                update::GpeopleContactGroupUpdate,
            },
            other_contacts::{
                copy_other_contact_to_my_contacts_group::GpeopleOtherContactCopy,
                list::{
                    GpeopleOtherContactsList, GpeopleOtherContactsListParams,
                    GpeopleOtherContactsListResponse,
                },
                search::GpeopleOtherContactsSearch,
            },
            people::{
                GpeoplePerson, GpeoplePersonField, GpeopleReadSourceType, GpeopleSearchResponse,
                connections::list::{
                    GpeopleConnectionsList, GpeopleConnectionsListParams,
                    GpeopleConnectionsListResponse,
                },
                create_contact::GpeopleContactCreate,
                delete_contact::GpeopleContactDelete,
                get::GpeoplePersonGet,
                search_contacts::GpeopleContactsSearch,
                update_contact::GpeopleContactUpdate,
            },
        },
        send::{GpeopleNoResponse, GpeopleSendError, GpeopleSendOutput},
    },
};

/// Errors produced by [`GpeopleClientStd`] operations.
#[derive(Debug, Error)]
pub enum GpeopleClientStdError {
    /// A People API send coroutine returned an error.
    #[error(transparent)]
    Send(#[from] GpeopleSendError),

    /// An I/O error occurred while reading from or writing to the stream.
    #[error(transparent)]
    Io(#[from] io::Error),

    #[cfg(any(
        feature = "rustls-aws",
        feature = "rustls-ring",
        feature = "native-tls"
    ))]
    /// A TLS handshake or connection error.
    #[error(transparent)]
    Tls(#[from] anyhow::Error),
    #[cfg(any(
        feature = "rustls-aws",
        feature = "rustls-ring",
        feature = "native-tls"
    ))]
    /// The People API base URL contains no host component.
    #[error("People URL `{0}` has no host")]
    UrlMissingHost(String),
    #[cfg(any(
        feature = "rustls-aws",
        feature = "rustls-ring",
        feature = "native-tls"
    ))]
    /// The URL scheme is neither `http` nor `https`.
    #[error("People URL `{0}` has unsupported scheme `{1}` (expected `http` or `https`)")]
    UrlUnsupportedScheme(String, String),
}

/// Optional settings for [`GpeopleClientStd::connect`]; every field has a
/// default (the TLS backend default, and the proxy resolved from the
/// environment).
#[derive(Default)]
pub struct GpeopleClientStdConnectOptions {
    #[cfg(any(
        feature = "rustls-aws",
        feature = "rustls-ring",
        feature = "native-tls"
    ))]
    /// TLS connector configuration passed to the underlying stream.
    pub tls: Tls,
    #[cfg(any(
        feature = "rustls-aws",
        feature = "rustls-ring",
        feature = "native-tls"
    ))]
    /// How the connection reaches the API: [`Proxy::System`] resolves it
    /// from the environment, [`Proxy::None`] connects directly.
    pub proxy: Proxy,
}

const READ_BUFFER_SIZE: usize = 16 * 1024;

/// Std-blocking People API client holding a stream and a bearer credential.
pub struct GpeopleClientStd {
    /// The underlying read/write stream used for all HTTP communication.
    pub stream: Box<dyn GpeopleStream>,
    /// The bearer-token credential attached to every outgoing request.
    pub auth: HttpAuthBearer,
}

impl GpeopleClientStd {
    /// Construct a client from an existing stream and a bearer token.
    pub fn new<S: Read + Write + Send + 'static>(stream: S, token: impl ToString) -> Self {
        Self {
            stream: Box::new(stream),
            auth: HttpAuthBearer::new(token.to_string()),
        }
    }

    #[cfg(any(
        feature = "rustls-aws",
        feature = "rustls-ring",
        feature = "native-tls"
    ))]
    /// Open a TLS connection to `people.googleapis.com` and return a client.
    pub fn connect(
        token: impl ToString,
        options: GpeopleClientStdConnectOptions,
    ) -> Result<Self, GpeopleClientStdError> {
        let GpeopleClientStdConnectOptions { tls, proxy } = options;

        let url = Url::parse(GPEOPLE_API_BASE).expect("People API base URL is valid");
        let host = url
            .host_str()
            .ok_or_else(|| GpeopleClientStdError::UrlMissingHost(url.to_string()))?;

        let stream = match url.scheme() {
            "http" => {
                let port = url.port().unwrap_or(80);
                let opts = TcpConnectOptions {
                    proxy,
                    ..Default::default()
                };

                Stream::connect_tcp(host, port, opts)?
            }
            "https" => {
                let port = url.port().unwrap_or(443);
                let opts = TlsConnectOptions {
                    tls,
                    proxy,
                    ..Default::default()
                };

                Stream::connect_tls(host, port, opts)?
            }
            scheme => {
                return Err(GpeopleClientStdError::UrlUnsupportedScheme(
                    url.to_string(),
                    scheme.to_string(),
                ));
            }
        };

        stream.set_read_timeout(Some(Duration::from_secs(30)))?;

        Ok(Self {
            stream: Box::new(stream),
            auth: HttpAuthBearer::new(token.to_string()),
        })
    }

    /// Replace the underlying stream, e.g. after a reconnect.
    pub fn set_stream<S: Read + Write + Send + 'static>(&mut self, stream: S) {
        self.stream = Box::new(stream);
    }

    /// Drive a People coroutine to completion, performing all I/O on the
    /// client's stream.
    pub fn run<C, T>(
        &mut self,
        mut coroutine: C,
    ) -> Result<GpeopleSendOutput<T>, GpeopleClientStdError>
    where
        C: GpeopleCoroutine<
                Yield = GpeopleYield,
                Return = Result<GpeopleSendOutput<T>, GpeopleSendError>,
            >,
    {
        let mut buf = [0u8; READ_BUFFER_SIZE];
        let mut arg: Option<&[u8]> = None;

        loop {
            match coroutine.resume(arg.take()) {
                GpeopleCoroutineState::Complete(Ok(out)) => return Ok(out),
                GpeopleCoroutineState::Complete(Err(err)) => return Err(err.into()),
                GpeopleCoroutineState::Yielded(GpeopleYield::WantsRead) => {
                    let n = self.stream.read(&mut buf)?;
                    arg = Some(&buf[..n]);
                }
                GpeopleCoroutineState::Yielded(GpeopleYield::WantsWrite(bytes)) => {
                    self.stream.write_all(&bytes)?;
                    arg = None;
                }
            }
        }
    }

    /// List the authenticated user's contacts (people.connections.list).
    pub fn connections_list(
        &mut self,
        person_fields: &[GpeoplePersonField],
        params: &GpeopleConnectionsListParams,
    ) -> Result<GpeopleSendOutput<GpeopleConnectionsListResponse>, GpeopleClientStdError> {
        let coroutine = GpeopleConnectionsList::new(&self.auth, person_fields, params)?;
        self.run(coroutine)
    }

    /// Fetch a single person by resource name (people.get).
    pub fn person_get(
        &mut self,
        resource_name: &str,
        person_fields: &[GpeoplePersonField],
        sources: &[GpeopleReadSourceType],
    ) -> Result<GpeopleSendOutput<GpeoplePerson>, GpeopleClientStdError> {
        let coroutine = GpeoplePersonGet::new(&self.auth, resource_name, person_fields, sources)?;
        self.run(coroutine)
    }

    /// Create a new contact and return the created person (people.createContact).
    pub fn contact_create(
        &mut self,
        person: &GpeoplePerson,
        person_fields: &[GpeoplePersonField],
        sources: &[GpeopleReadSourceType],
    ) -> Result<GpeopleSendOutput<GpeoplePerson>, GpeopleClientStdError> {
        let coroutine = GpeopleContactCreate::new(&self.auth, person, person_fields, sources)?;
        self.run(coroutine)
    }

    /// Update an existing contact and return the updated person
    /// (people.updateContact).
    pub fn contact_update(
        &mut self,
        person: &GpeoplePerson,
        update_person_fields: &[GpeoplePersonField],
        person_fields: &[GpeoplePersonField],
        sources: &[GpeopleReadSourceType],
    ) -> Result<GpeopleSendOutput<GpeoplePerson>, GpeopleClientStdError> {
        let coroutine = GpeopleContactUpdate::new(
            &self.auth,
            person,
            update_person_fields,
            person_fields,
            sources,
        )?;
        self.run(coroutine)
    }

    /// Delete a contact by resource name (people.deleteContact).
    pub fn contact_delete(
        &mut self,
        resource_name: &str,
    ) -> Result<GpeopleSendOutput<GpeopleNoResponse>, GpeopleClientStdError> {
        let coroutine = GpeopleContactDelete::new(&self.auth, resource_name)?;
        self.run(coroutine)
    }

    /// Search the authenticated user's contacts by query string
    /// (people.searchContacts).
    pub fn contacts_search(
        &mut self,
        query: &str,
        read_mask: &[GpeoplePersonField],
        page_size: Option<u32>,
        sources: &[GpeopleReadSourceType],
    ) -> Result<GpeopleSendOutput<GpeopleSearchResponse>, GpeopleClientStdError> {
        let coroutine =
            GpeopleContactsSearch::new(&self.auth, query, read_mask, page_size, sources)?;
        self.run(coroutine)
    }

    /// List the authenticated user's contact groups
    /// (contactGroups.list).
    pub fn contact_groups_list(
        &mut self,
        group_fields: &[GpeopleGroupField],
        params: &GpeopleContactGroupsListParams,
    ) -> Result<GpeopleSendOutput<GpeopleContactGroupsListResponse>, GpeopleClientStdError> {
        let coroutine = GpeopleContactGroupsList::new(&self.auth, group_fields, params)?;
        self.run(coroutine)
    }

    /// Fetch a single contact group by resource name (contactGroups.get).
    pub fn contact_group_get(
        &mut self,
        resource_name: &str,
        max_members: Option<u32>,
        group_fields: &[GpeopleGroupField],
    ) -> Result<GpeopleSendOutput<GpeopleContactGroup>, GpeopleClientStdError> {
        let coroutine =
            GpeopleContactGroupGet::new(&self.auth, resource_name, max_members, group_fields)?;
        self.run(coroutine)
    }

    /// Create a new contact group and return it (contactGroups.create).
    pub fn contact_group_create(
        &mut self,
        group: &GpeopleContactGroup,
        read_group_fields: &[GpeopleGroupField],
    ) -> Result<GpeopleSendOutput<GpeopleContactGroup>, GpeopleClientStdError> {
        let coroutine = GpeopleContactGroupCreate::new(&self.auth, group, read_group_fields)?;
        self.run(coroutine)
    }

    /// Update an existing contact group and return it
    /// (contactGroups.update).
    pub fn contact_group_update(
        &mut self,
        group: &GpeopleContactGroup,
        update_group_fields: &[GpeopleGroupField],
        read_group_fields: &[GpeopleGroupField],
    ) -> Result<GpeopleSendOutput<GpeopleContactGroup>, GpeopleClientStdError> {
        let coroutine = GpeopleContactGroupUpdate::new(
            &self.auth,
            group,
            update_group_fields,
            read_group_fields,
        )?;
        self.run(coroutine)
    }

    /// Delete a contact group; optionally also delete its member contacts
    /// (contactGroups.delete).
    pub fn contact_group_delete(
        &mut self,
        resource_name: &str,
        delete_contacts: bool,
    ) -> Result<GpeopleSendOutput<GpeopleNoResponse>, GpeopleClientStdError> {
        let coroutine = GpeopleContactGroupDelete::new(&self.auth, resource_name, delete_contacts)?;
        self.run(coroutine)
    }

    /// Add and/or remove members from a contact group
    /// (contactGroups.members.modify).
    pub fn contact_group_members_modify(
        &mut self,
        resource_name: &str,
        resource_names_to_add: &[String],
        resource_names_to_remove: &[String],
    ) -> Result<GpeopleSendOutput<GpeopleContactGroupMembersModifyResponse>, GpeopleClientStdError>
    {
        let coroutine = GpeopleContactGroupMembersModify::new(
            &self.auth,
            resource_name,
            resource_names_to_add,
            resource_names_to_remove,
        )?;
        self.run(coroutine)
    }

    /// List the authenticated user's "other contacts" (otherContacts.list).
    pub fn other_contacts_list(
        &mut self,
        read_mask: &[GpeoplePersonField],
        params: &GpeopleOtherContactsListParams,
    ) -> Result<GpeopleSendOutput<GpeopleOtherContactsListResponse>, GpeopleClientStdError> {
        let coroutine = GpeopleOtherContactsList::new(&self.auth, read_mask, params)?;
        self.run(coroutine)
    }

    /// Search other contacts by query string (otherContacts.search).
    pub fn other_contacts_search(
        &mut self,
        query: &str,
        read_mask: &[GpeoplePersonField],
        page_size: Option<u32>,
    ) -> Result<GpeopleSendOutput<GpeopleSearchResponse>, GpeopleClientStdError> {
        let coroutine = GpeopleOtherContactsSearch::new(&self.auth, query, read_mask, page_size)?;
        self.run(coroutine)
    }

    /// Copy an other contact into the user's personal contacts
    /// (otherContacts.copyOtherContactToMyContactsGroup).
    pub fn other_contact_copy(
        &mut self,
        resource_name: &str,
        copy_mask: &[GpeoplePersonField],
        read_mask: &[GpeoplePersonField],
        sources: &[GpeopleReadSourceType],
    ) -> Result<GpeopleSendOutput<GpeoplePerson>, GpeopleClientStdError> {
        let coroutine =
            GpeopleOtherContactCopy::new(&self.auth, resource_name, copy_mask, read_mask, sources)?;
        self.run(coroutine)
    }
}

impl fmt::Debug for GpeopleClientStd {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GpeopleClientStd")
            .field("auth", &self.auth)
            .finish_non_exhaustive()
    }
}

/// Object-safe stream used by [`GpeopleClientStd`]; combines `Read`, `Write`,
/// `Send`, and `Any` so the concrete type can be recovered at runtime.
pub trait GpeopleStream: Read + Write + Send + Any {
    /// Return a mutable `Any` reference to the underlying concrete type.
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T: Read + Write + Send + Any> GpeopleStream for T {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
