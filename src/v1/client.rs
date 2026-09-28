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

#[cfg(any(
    feature = "rustls-aws",
    feature = "rustls-ring",
    feature = "native-tls"
))]
use alloc::string::ToString;
use alloc::{boxed::Box, string::String};
use io_http::rfc6750::bearer::HttpAuthBearer;

use std::io::{self, Read, Write};

#[cfg(any(
    feature = "rustls-aws",
    feature = "rustls-ring",
    feature = "native-tls"
))]
use pimalaya_stream::stream::{Stream, TcpConnectOptions, TlsConnectOptions};
#[cfg(any(
    feature = "rustls-aws",
    feature = "rustls-ring",
    feature = "native-tls"
))]
pub use pimalaya_stream::tls::*;
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
use crate::v1::send::PEOPLE_API_BASE;
use crate::{
    coroutine::*,
    v1::{
        rest::{
            contact_groups::{
                PeopleContactGroup, PeopleGroupField,
                create::PeopleContactGroupCreate,
                delete::PeopleContactGroupDelete,
                get::PeopleContactGroupGet,
                list::{
                    PeopleContactGroupsList, PeopleContactGroupsListParams,
                    PeopleContactGroupsListResponse,
                },
                members::modify::{
                    PeopleContactGroupMembersModify, PeopleContactGroupMembersModifyResponse,
                },
                update::PeopleContactGroupUpdate,
            },
            other_contacts::{
                copy_other_contact_to_my_contacts_group::PeopleOtherContactCopy,
                list::{
                    PeopleOtherContactsList, PeopleOtherContactsListParams,
                    PeopleOtherContactsListResponse,
                },
                search::PeopleOtherContactsSearch,
            },
            people::{
                PeoplePerson, PeoplePersonField, PeopleReadSourceType, PeopleSearchResponse,
                connections::list::{
                    PeopleConnectionsList, PeopleConnectionsListParams,
                    PeopleConnectionsListResponse,
                },
                create_contact::PeopleContactCreate,
                delete_contact::PeopleContactDelete,
                get::PeoplePersonGet,
                search_contacts::PeopleContactsSearch,
                update_contact::PeopleContactUpdate,
            },
        },
        send::{PeopleNoResponse, PeopleSendError, PeopleSendOutput},
    },
};

/// Errors produced by [`PeopleClientStd`] operations.
#[derive(Debug, Error)]
pub enum PeopleClientStdError {
    /// A People API send coroutine returned an error.
    #[error(transparent)]
    Send(#[from] PeopleSendError),

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

/// Optional settings for [`PeopleClientStd::connect`]; every field has a
/// default (the TLS backend default).
#[derive(Default)]
pub struct PeopleClientStdConnectOptions {
    #[cfg(any(
        feature = "rustls-aws",
        feature = "rustls-ring",
        feature = "native-tls"
    ))]
    /// TLS connector configuration passed to the underlying stream.
    pub tls: Tls,
}

const READ_BUFFER_SIZE: usize = 16 * 1024;

/// Std-blocking People API client holding a stream and a bearer credential.
pub struct PeopleClientStd {
    /// The underlying read/write stream used for all HTTP communication.
    pub stream: Box<dyn PeopleStream>,
    /// The bearer-token credential attached to every outgoing request.
    pub auth: HttpAuthBearer,
}

impl PeopleClientStd {
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
        options: PeopleClientStdConnectOptions,
    ) -> Result<Self, PeopleClientStdError> {
        let PeopleClientStdConnectOptions { tls } = options;

        let url = Url::parse(PEOPLE_API_BASE).expect("People API base URL is valid");
        let host = url
            .host_str()
            .ok_or_else(|| PeopleClientStdError::UrlMissingHost(url.to_string()))?;

        let stream = match url.scheme() {
            "http" => {
                let port = url.port().unwrap_or(80);
                Stream::connect_tcp(host, port, TcpConnectOptions::default())?
            }
            "https" => {
                let port = url.port().unwrap_or(443);
                let opts = TlsConnectOptions {
                    tls: tls.clone(),
                    ..Default::default()
                };

                Stream::connect_tls(host, port, opts)?
            }
            scheme => {
                return Err(PeopleClientStdError::UrlUnsupportedScheme(
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
    ) -> Result<PeopleSendOutput<T>, PeopleClientStdError>
    where
        C: PeopleCoroutine<
                Yield = PeopleYield,
                Return = Result<PeopleSendOutput<T>, PeopleSendError>,
            >,
    {
        let mut buf = [0u8; READ_BUFFER_SIZE];
        let mut arg: Option<&[u8]> = None;

        loop {
            match coroutine.resume(arg.take()) {
                PeopleCoroutineState::Complete(Ok(out)) => return Ok(out),
                PeopleCoroutineState::Complete(Err(err)) => return Err(err.into()),
                PeopleCoroutineState::Yielded(PeopleYield::WantsRead) => {
                    let n = self.stream.read(&mut buf)?;
                    arg = Some(&buf[..n]);
                }
                PeopleCoroutineState::Yielded(PeopleYield::WantsWrite(bytes)) => {
                    self.stream.write_all(&bytes)?;
                    arg = None;
                }
            }
        }
    }

    /// List the authenticated user's contacts (people.connections.list).
    pub fn connections_list(
        &mut self,
        person_fields: &[PeoplePersonField],
        params: &PeopleConnectionsListParams,
    ) -> Result<PeopleSendOutput<PeopleConnectionsListResponse>, PeopleClientStdError> {
        let coroutine = PeopleConnectionsList::new(&self.auth, person_fields, params)?;
        self.run(coroutine)
    }

    /// Fetch a single person by resource name (people.get).
    pub fn person_get(
        &mut self,
        resource_name: &str,
        person_fields: &[PeoplePersonField],
        sources: &[PeopleReadSourceType],
    ) -> Result<PeopleSendOutput<PeoplePerson>, PeopleClientStdError> {
        let coroutine = PeoplePersonGet::new(&self.auth, resource_name, person_fields, sources)?;
        self.run(coroutine)
    }

    /// Create a new contact and return the created person (people.createContact).
    pub fn contact_create(
        &mut self,
        person: &PeoplePerson,
        person_fields: &[PeoplePersonField],
        sources: &[PeopleReadSourceType],
    ) -> Result<PeopleSendOutput<PeoplePerson>, PeopleClientStdError> {
        let coroutine = PeopleContactCreate::new(&self.auth, person, person_fields, sources)?;
        self.run(coroutine)
    }

    /// Update an existing contact and return the updated person
    /// (people.updateContact).
    pub fn contact_update(
        &mut self,
        person: &PeoplePerson,
        update_person_fields: &[PeoplePersonField],
        person_fields: &[PeoplePersonField],
        sources: &[PeopleReadSourceType],
    ) -> Result<PeopleSendOutput<PeoplePerson>, PeopleClientStdError> {
        let coroutine = PeopleContactUpdate::new(
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
    ) -> Result<PeopleSendOutput<PeopleNoResponse>, PeopleClientStdError> {
        let coroutine = PeopleContactDelete::new(&self.auth, resource_name)?;
        self.run(coroutine)
    }

    /// Search the authenticated user's contacts by query string
    /// (people.searchContacts).
    pub fn contacts_search(
        &mut self,
        query: &str,
        read_mask: &[PeoplePersonField],
        page_size: Option<u32>,
        sources: &[PeopleReadSourceType],
    ) -> Result<PeopleSendOutput<PeopleSearchResponse>, PeopleClientStdError> {
        let coroutine =
            PeopleContactsSearch::new(&self.auth, query, read_mask, page_size, sources)?;
        self.run(coroutine)
    }

    /// List the authenticated user's contact groups
    /// (contactGroups.list).
    pub fn contact_groups_list(
        &mut self,
        group_fields: &[PeopleGroupField],
        params: &PeopleContactGroupsListParams,
    ) -> Result<PeopleSendOutput<PeopleContactGroupsListResponse>, PeopleClientStdError> {
        let coroutine = PeopleContactGroupsList::new(&self.auth, group_fields, params)?;
        self.run(coroutine)
    }

    /// Fetch a single contact group by resource name (contactGroups.get).
    pub fn contact_group_get(
        &mut self,
        resource_name: &str,
        max_members: Option<u32>,
        group_fields: &[PeopleGroupField],
    ) -> Result<PeopleSendOutput<PeopleContactGroup>, PeopleClientStdError> {
        let coroutine =
            PeopleContactGroupGet::new(&self.auth, resource_name, max_members, group_fields)?;
        self.run(coroutine)
    }

    /// Create a new contact group and return it (contactGroups.create).
    pub fn contact_group_create(
        &mut self,
        group: &PeopleContactGroup,
        read_group_fields: &[PeopleGroupField],
    ) -> Result<PeopleSendOutput<PeopleContactGroup>, PeopleClientStdError> {
        let coroutine = PeopleContactGroupCreate::new(&self.auth, group, read_group_fields)?;
        self.run(coroutine)
    }

    /// Update an existing contact group and return it
    /// (contactGroups.update).
    pub fn contact_group_update(
        &mut self,
        group: &PeopleContactGroup,
        update_group_fields: &[PeopleGroupField],
        read_group_fields: &[PeopleGroupField],
    ) -> Result<PeopleSendOutput<PeopleContactGroup>, PeopleClientStdError> {
        let coroutine = PeopleContactGroupUpdate::new(
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
    ) -> Result<PeopleSendOutput<PeopleNoResponse>, PeopleClientStdError> {
        let coroutine = PeopleContactGroupDelete::new(&self.auth, resource_name, delete_contacts)?;
        self.run(coroutine)
    }

    /// Add and/or remove members from a contact group
    /// (contactGroups.members.modify).
    pub fn contact_group_members_modify(
        &mut self,
        resource_name: &str,
        resource_names_to_add: &[String],
        resource_names_to_remove: &[String],
    ) -> Result<PeopleSendOutput<PeopleContactGroupMembersModifyResponse>, PeopleClientStdError>
    {
        let coroutine = PeopleContactGroupMembersModify::new(
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
        read_mask: &[PeoplePersonField],
        params: &PeopleOtherContactsListParams,
    ) -> Result<PeopleSendOutput<PeopleOtherContactsListResponse>, PeopleClientStdError> {
        let coroutine = PeopleOtherContactsList::new(&self.auth, read_mask, params)?;
        self.run(coroutine)
    }

    /// Search other contacts by query string (otherContacts.search).
    pub fn other_contacts_search(
        &mut self,
        query: &str,
        read_mask: &[PeoplePersonField],
        page_size: Option<u32>,
    ) -> Result<PeopleSendOutput<PeopleSearchResponse>, PeopleClientStdError> {
        let coroutine = PeopleOtherContactsSearch::new(&self.auth, query, read_mask, page_size)?;
        self.run(coroutine)
    }

    /// Copy an other contact into the user's personal contacts
    /// (otherContacts.copyOtherContactToMyContactsGroup).
    pub fn other_contact_copy(
        &mut self,
        resource_name: &str,
        copy_mask: &[PeoplePersonField],
        read_mask: &[PeoplePersonField],
        sources: &[PeopleReadSourceType],
    ) -> Result<PeopleSendOutput<PeoplePerson>, PeopleClientStdError> {
        let coroutine =
            PeopleOtherContactCopy::new(&self.auth, resource_name, copy_mask, read_mask, sources)?;
        self.run(coroutine)
    }
}

impl fmt::Debug for PeopleClientStd {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PeopleClientStd")
            .field("auth", &self.auth)
            .finish_non_exhaustive()
    }
}

/// Object-safe stream used by [`PeopleClientStd`]; combines `Read`, `Write`,
/// `Send`, and `Any` so the concrete type can be recovered at runtime.
pub trait PeopleStream: Read + Write + Send + Any {
    /// Return a mutable `Any` reference to the underlying concrete type.
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T: Read + Write + Send + Any> PeopleStream for T {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
