#![cfg(any(
    feature = "rustls-ring",
    feature = "rustls-aws",
    feature = "native-tls"
))]
//! End-to-end People API test.
//!
//! Exercises the whole CRUD surface (contacts, contact photos, contact
//! groups, group members, batch methods, search, "Other contacts").
//! Everything the run creates is named after one
//! `io-gpeople-test-<millis>` tag, and [`with_cleanup`] deletes it
//! however the run ends, leaving the account untouched. With the
//! `vcard` feature, [`vcard`] also round-trips a contact through the
//! vCard projection.
//!
//! It needs the https://www.googleapis.com/auth/contacts and
//! https://www.googleapis.com/auth/contacts.other.readonly scopes, and
//! there are two ways to hand it a Bearer token.
//!
//! A token minted by hand acts as you and dies within the hour:
//!
//! ```sh
//! GPEOPLE_ACCESS_TOKEN="<token>" \
//! cargo test --features vcard --test people -- --include-ignored
//! ```
//!
//! A Workspace service account with domain-wide delegation instead
//! signs its own assertion on behalf of a user of the domain, so the
//! run needs no human. The account owns no contacts of its own, hence
//! the subject naming the user whose contacts the test borrows:
//!
//! ```sh
//! GPEOPLE_SERVICE_ACCOUNT_KEY_FILE=key.json \
//! GPEOPLE_SERVICE_ACCOUNT_SUBJECT=google@pimalaya.org \
//! cargo test --features vcard --test people -- --include-ignored
//! ```
//!
//! CI passes the key itself rather than a path, as
//! `GPEOPLE_SERVICE_ACCOUNT_KEY`, since it comes straight out of a
//! secret. The subject defaults to `google@pimalaya.org`, the Pimalaya
//! test user. The delegation must grant both scopes above.
//!
//! Left out on purpose: copying an "Other contact" into the contacts,
//! which moves an entry the run did not create, and the directory
//! methods, which need the `directory.readonly` scope.

use core::fmt::Debug;

use std::{
    borrow::Cow,
    env, fs,
    panic::{self, AssertUnwindSafe},
    slice,
    sync::{Mutex, PoisonError},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use base64::{Engine, engine::general_purpose::STANDARD};
#[cfg(feature = "vcard")]
use io_gpeople::v1::rest::people::vcard::GPEOPLE_PERSON_VCARD_FIELDS;
use io_gpeople::v1::{
    client::GpeopleClientStd,
    rest::{
        contact_groups::{
            GpeopleContactGroup, GpeopleGroupField, batch_get::GpeopleContactGroupsBatchGet,
        },
        other_contacts::list::GpeopleOtherContactsListParams,
        people::{
            GpeopleEmailAddress, GpeopleName, GpeoplePerson, GpeoplePersonField,
            batch_create_contacts::GpeopleContactsBatchCreate,
            batch_delete_contacts::GpeopleContactsBatchDelete,
            batch_update_contacts::GpeopleContactsBatchUpdate,
            connections::list::GpeopleConnectionsListParams,
            delete_contact_photo::GpeopleContactPhotoDelete, get_batch_get::GpeoplePersonsBatchGet,
            update_contact_photo::GpeopleContactPhotoUpdate,
        },
    },
};
use io_oauth::{
    client::Oauth20ClientStd,
    rfc7523::{
        assertion::{Oauth20JwtBearerClaims, Oauth20JwtBearerKey},
        auth_grant::Oauth20JwtBearerGrantRequestParams,
    },
};
use pimalaya_stream::tls::Tls;
use secrecy::ExposeSecret;
use serde::Deserialize;
use url::Url;

/// The scopes the test needs: read and write on the contacts, read on
/// the "Other contacts".
const GPEOPLE_SCOPES: [&str; 2] = [
    "https://www.googleapis.com/auth/contacts",
    "https://www.googleapis.com/auth/contacts.other.readonly",
];

/// The Pimalaya Workspace user the service account acts as when
/// `GPEOPLE_SERVICE_ACCOUNT_SUBJECT` names none.
const DEFAULT_SUBJECT: &str = "google@pimalaya.org";

/// A 1x1 PNG, the contact photo the run uploads.
const PHOTO_BASE64: &str =
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII=";

/// The fields the run reads back from a person.
const NAME_FIELDS: &[GpeoplePersonField] = &[
    GpeoplePersonField::Names,
    GpeoplePersonField::EmailAddresses,
];

/// Serializes the tests, which share one account: People aborts
/// concurrent writes to the contacts of one user (409).
static ACCOUNT: Mutex<()> = Mutex::new(());

#[test]
#[ignore = "requires GPEOPLE_ACCESS_TOKEN or a service account key, and --include-ignored"]
fn people() {
    let _account = ACCOUNT.lock().unwrap_or_else(PoisonError::into_inner);
    let mut client = connect();
    let tag = format!("io-gpeople-test-{}", unix_millis());
    let mut leftovers = Leftovers::default();

    with_cleanup(
        &mut client,
        &mut leftovers,
        |client, leftovers| {
            baseline(client);
            let group = contact_group(client, &tag, leftovers);
            let contact = contact(client, &tag, leftovers);
            membership(client, &group, &contact);
            photo(client, &contact);
            search(client, &tag, &contact);
            batch(client, &tag, leftovers);
            other_contacts(client);

            client.contact_delete(&contact).expect("contact delete");
            leftovers.contacts.retain(|c| c != &contact);
        },
        |client, leftovers| {
            for name in &leftovers.contacts {
                if let Err(err) = client.contact_delete(name) {
                    report_leftover("contact", name, &err);
                }
            }

            if let Some(name) = &leftovers.group
                && let Err(err) = client.contact_group_delete(name, false)
            {
                report_leftover("contact group", name, &err);
            }
        },
    );
}

/// Writes a contact from a vCard and reads it back as one, then edits
/// the vCard and writes the change through the update mask the
/// projection computes.
///
/// Whatever Google normalizes or drops shows as a difference between
/// the projection of the card written and that of the card read back.
/// Needs the `vcard` feature.
#[cfg(feature = "vcard")]
#[test]
#[ignore = "requires GPEOPLE_ACCESS_TOKEN or a service account key, and --include-ignored"]
fn vcard() {
    let _account = ACCOUNT.lock().unwrap_or_else(PoisonError::into_inner);
    let mut client = connect();
    let tag = format!("io-gpeople-test-{}", unix_millis());
    let uid = format!("{tag}@pimalaya.org");
    let mut leftovers = Leftovers::default();

    with_cleanup(
        &mut client,
        &mut leftovers,
        |client, leftovers| {
            let card = vcard_document(&tag, &uid, "+33 6 12 34 56 78", true);
            let written = GpeoplePerson::from_vcard(&card).expect("the card projects");
            let created = client
                .contact_create(&written, GPEOPLE_PERSON_VCARD_FIELDS, &[])
                .expect("contact create")
                .response;
            leftovers.contacts.push(created.resource_name.clone());

            let fetched = client
                .person_get(&created.resource_name, GPEOPLE_PERSON_VCARD_FIELDS, &[])
                .expect("person get")
                .response;
            assert_eq!(
                fetched.stashed_uid().as_deref(),
                Some(uid.as_str()),
                "the UID survives through the stash"
            );

            let read = fetched.to_vcard();
            assert!(
                read.contains("X-PIMALAYA-TEST:kept verbatim"),
                "the stash restores the unmanaged lines:\n{read}"
            );

            let back = GpeoplePerson::from_vcard(&read).expect("the read card projects");
            let changed = back.changed_fields(&written);
            assert!(
                changed.is_empty(),
                "Google altered {changed:?}\nwritten:\n{card}\nread:\n{read}"
            );

            let edited_card = vcard_document(&tag, &uid, "+33 6 98 76 54 32", false);
            let edited = GpeoplePerson::from_vcard(&edited_card).expect("the edited card projects");
            let mask = edited.changed_fields(&back);
            assert!(
                mask.len() == 2
                    && mask.contains(&GpeoplePersonField::PhoneNumbers)
                    && mask.contains(&GpeoplePersonField::Biographies),
                "the mask holds the phone and the note alone, got {mask:?}"
            );

            let updated = client
                .contact_update(
                    &GpeoplePerson {
                        resource_name: fetched.resource_name.clone(),
                        etag: fetched.etag.clone(),
                        ..edited.clone()
                    },
                    &mask,
                    GPEOPLE_PERSON_VCARD_FIELDS,
                    &[],
                )
                .expect("contact update")
                .response;

            let reread = updated.to_vcard();
            let back = GpeoplePerson::from_vcard(&reread).expect("the updated card projects");
            let changed = back.changed_fields(&edited);
            assert!(
                changed.is_empty(),
                "Google altered {changed:?}\nwritten:\n{edited_card}\nread:\n{reread}"
            );
        },
        |client, leftovers| {
            for name in &leftovers.contacts {
                if let Err(err) = client.contact_delete(name) {
                    report_leftover("contact", name, &err);
                }
            }
        },
    );
}

/// Lists the contacts and the contact groups the account starts with.
fn baseline(client: &mut GpeopleClientStd) {
    client
        .connections_list(NAME_FIELDS, &GpeopleConnectionsListParams::default())
        .expect("connections list");

    let groups = client
        .contact_groups_list(&[], &Default::default())
        .expect("contact groups list")
        .response;
    assert!(
        groups
            .contact_groups
            .iter()
            .any(|group| group.resource_name == "contactGroups/myContacts"),
        "contact groups list should contain the myContacts system group"
    );
}

/// Creates, reads, renames and batch-reads the test contact group, and
/// returns its resource name.
fn contact_group(client: &mut GpeopleClientStd, tag: &str, leftovers: &mut Leftovers) -> String {
    let group = client
        .contact_group_create(
            &GpeopleContactGroup {
                name: Some(tag.to_owned()),
                ..Default::default()
            },
            &[],
        )
        .expect("contact group create")
        .response;
    let name = group.resource_name.clone();
    leftovers.group = Some(name.clone());
    assert_eq!(
        group.name.as_deref(),
        Some(tag),
        "created group name mismatch"
    );

    let fetched = client
        .contact_group_get(&name, None, &[])
        .expect("contact group get")
        .response;
    assert_eq!(
        fetched.resource_name, name,
        "contact group get resource name mismatch"
    );

    let renamed_name = format!("{tag}-renamed");
    let renamed = client
        .contact_group_update(
            &GpeopleContactGroup {
                name: Some(renamed_name.clone()),
                ..fetched
            },
            &[GpeopleGroupField::Name],
            &[],
        )
        .expect("contact group update")
        .response;
    assert_eq!(
        renamed.name.as_deref(),
        Some(renamed_name.as_str()),
        "group rename not reflected"
    );

    let coroutine =
        GpeopleContactGroupsBatchGet::new(&client.auth, slice::from_ref(&name), None, &[])
            .expect("contact groups batch get coroutine");
    let fetched = client
        .run(coroutine)
        .expect("contact groups batch get")
        .response;
    assert!(
        fetched.responses.iter().any(|response| response
            .contact_group
            .as_ref()
            .is_some_and(|group| group.resource_name == name)),
        "batch get should return the test group"
    );

    name
}

/// Creates, reads and renames the test contact, and returns its
/// resource name.
fn contact(client: &mut GpeopleClientStd, tag: &str, leftovers: &mut Leftovers) -> String {
    let given_name = format!("{tag}-contact");
    let contact = client
        .contact_create(
            &GpeoplePerson {
                names: vec![GpeopleName {
                    given_name: Some(given_name.clone()),
                    ..Default::default()
                }],
                email_addresses: vec![GpeopleEmailAddress {
                    value: Some(format!("{tag}@example.com")),
                    ..Default::default()
                }],
                ..Default::default()
            },
            NAME_FIELDS,
            &[],
        )
        .expect("contact create")
        .response;
    let name = contact.resource_name.clone();
    leftovers.contacts.push(name.clone());
    assert_eq!(
        contact.names[0].given_name.as_deref(),
        Some(given_name.as_str()),
        "created contact name mismatch"
    );

    let fetched = client
        .person_get(&name, NAME_FIELDS, &[])
        .expect("person get")
        .response;
    assert_eq!(
        fetched.resource_name, name,
        "person get resource name mismatch"
    );

    let renamed_name = format!("{given_name}-renamed");
    let renamed = client
        .contact_update(
            &GpeoplePerson {
                names: vec![GpeopleName {
                    given_name: Some(renamed_name.clone()),
                    ..Default::default()
                }],
                ..fetched
            },
            &[GpeoplePersonField::Names],
            NAME_FIELDS,
            &[],
        )
        .expect("contact update")
        .response;
    assert_eq!(
        renamed.names[0].given_name.as_deref(),
        Some(renamed_name.as_str()),
        "contact rename not reflected"
    );

    name
}

/// Adds the test contact to the test group, checks it, then removes it.
fn membership(client: &mut GpeopleClientStd, group: &str, contact: &str) {
    let contact = [contact.to_owned()];

    let modified = client
        .contact_group_members_modify(group, &contact, &[])
        .expect("contact group members add")
        .response;
    assert!(
        modified.not_found_resource_names.is_empty(),
        "contact should have been found when adding to the group"
    );

    let membership = client
        .contact_group_get(group, Some(10), &[])
        .expect("contact group get members")
        .response;
    assert!(
        membership.member_resource_names.contains(&contact[0]),
        "group should carry the test contact after members modify"
    );

    client
        .contact_group_members_modify(group, &[], &contact)
        .expect("contact group members remove");
}

/// Uploads a photo for the test contact, then deletes it.
fn photo(client: &mut GpeopleClientStd, contact: &str) {
    let photo = STANDARD.decode(PHOTO_BASE64).expect("the photo is base64");
    let fields = &[GpeoplePersonField::Photos];

    let coroutine = GpeopleContactPhotoUpdate::new(&client.auth, contact, &photo, fields, &[])
        .expect("contact photo update coroutine");
    let updated = client
        .run(coroutine)
        .expect("contact photo update")
        .response;
    let person = updated.person.expect("the update returns the person");
    assert!(
        person.photos.iter().any(|p| p.default != Some(true)),
        "the contact carries a custom photo after the update"
    );

    let coroutine = GpeopleContactPhotoDelete::new(&client.auth, contact, fields, &[])
        .expect("contact photo delete coroutine");
    let deleted = client
        .run(coroutine)
        .expect("contact photo delete")
        .response;
    assert!(deleted.person.is_some(), "the delete returns the person");

    // NOTE: the person the delete answers, and the reads right after,
    // may still carry the custom photo for a moment.
    for _ in 0..10 {
        let person = client
            .person_get(contact, fields, &[])
            .expect("person get")
            .response;

        if person.photos.iter().all(|p| p.default == Some(true)) {
            return;
        }

        thread::sleep(Duration::from_secs(1));
    }

    panic!("the custom photo outlived its delete");
}

/// Searches the contacts for the test contact.
///
/// The search runs on a cache that a request has to warm up first, as
/// the reference advises, and that only catches up with a write after a
/// few seconds, hence the polling.
fn search(client: &mut GpeopleClientStd, tag: &str, contact: &str) {
    client
        .contacts_search("", NAME_FIELDS, Some(10), &[])
        .expect("contacts search warmup");

    for _ in 0..10 {
        let found = client
            .contacts_search(tag, NAME_FIELDS, Some(10), &[])
            .expect("contacts search")
            .response;

        if found
            .results
            .iter()
            .filter_map(|result| result.person.as_ref())
            .any(|person| person.resource_name == contact)
        {
            return;
        }

        thread::sleep(Duration::from_secs(3));
    }

    panic!("contacts search never surfaced the test contact");
}

/// Creates, reads, renames then deletes two contacts in batch.
fn batch(client: &mut GpeopleClientStd, tag: &str, leftovers: &mut Leftovers) {
    let names = &[GpeoplePersonField::Names];

    let contacts: Vec<GpeoplePerson> = (0..2)
        .map(|i| GpeoplePerson {
            names: vec![GpeopleName {
                given_name: Some(format!("{tag}-batch-{i}")),
                ..Default::default()
            }],
            ..Default::default()
        })
        .collect();
    let coroutine = GpeopleContactsBatchCreate::new(&client.auth, &contacts, names, &[])
        .expect("contacts batch create coroutine");
    let created = client
        .run(coroutine)
        .expect("contacts batch create")
        .response;
    let resource_names: Vec<String> = created
        .created_people
        .iter()
        .filter_map(|response| response.person.as_ref())
        .map(|person| person.resource_name.clone())
        .collect();
    leftovers.contacts.extend(resource_names.iter().cloned());
    assert_eq!(
        resource_names.len(),
        2,
        "batch create should expose both created resource names"
    );

    let coroutine = GpeoplePersonsBatchGet::new(&client.auth, &resource_names, names, &[])
        .expect("persons batch get coroutine");
    let fetched = client.run(coroutine).expect("persons batch get").response;
    assert_eq!(
        fetched.responses.len(),
        2,
        "batch get should return both contacts"
    );

    let renamed: Vec<GpeoplePerson> = fetched
        .responses
        .iter()
        .filter_map(|response| response.person.clone())
        .map(|person| GpeoplePerson {
            names: vec![GpeopleName {
                given_name: person.names[0]
                    .given_name
                    .as_ref()
                    .map(|name| format!("{name}-renamed")),
                ..Default::default()
            }],
            ..person
        })
        .collect();
    let coroutine = GpeopleContactsBatchUpdate::new(&client.auth, &renamed, names, names, &[])
        .expect("contacts batch update coroutine");
    let updated = client
        .run(coroutine)
        .expect("contacts batch update")
        .response;
    assert_eq!(
        updated.update_result.len(),
        2,
        "batch update should return both contacts"
    );

    let coroutine = GpeopleContactsBatchDelete::new(&client.auth, &resource_names)
        .expect("contacts batch delete coroutine");
    client.run(coroutine).expect("contacts batch delete");
    leftovers.contacts.retain(|c| !resource_names.contains(c));
}

/// Lists and searches the "Other contacts", read-only.
fn other_contacts(client: &mut GpeopleClientStd) {
    let fields = &[GpeoplePersonField::EmailAddresses];

    client
        .other_contacts_list(fields, &GpeopleOtherContactsListParams::default())
        .expect("other contacts list");

    client
        .other_contacts_search("", fields, Some(10))
        .expect("other contacts search warmup");

    client
        .other_contacts_search("io-gpeople", fields, Some(10))
        .expect("other contacts search");
}

// --- utils ---------------------------------------------------------------

/// What the run created and has not deleted yet, for [`with_cleanup`]
/// to remove whichever way the run ends.
#[derive(Debug, Default)]
struct Leftovers {
    group: Option<String>,
    contacts: Vec<String>,
}

/// A vCard 4.0 touching every property the projection manages, plus
/// one it stashes.
#[cfg(feature = "vcard")]
fn vcard_document(tag: &str, uid: &str, tel: &str, note: bool) -> String {
    let mut lines = vec![
        String::from("BEGIN:VCARD"),
        String::from("VERSION:4.0"),
        format!("UID:{uid}"),
        format!("FN:{tag} Doe"),
        format!("N:Doe;{tag};;;"),
        String::from("NICKNAME:jd"),
        format!("EMAIL;TYPE=work:{tag}@example.com"),
        format!("TEL;TYPE=cell:{tel}"),
        String::from("ADR;TYPE=home:;;1 rue de la Paix;Paris;;75002;France"),
        String::from("ORG:Pimalaya"),
        String::from("TITLE:Tester"),
        String::from("URL:https://pimalaya.org"),
        String::from("BDAY:19900102"),
        String::from("RELATED;TYPE=spouse;VALUE=text:Jane Doe"),
        String::from("X-PIMALAYA-TEST:kept verbatim"),
    ];

    if note {
        lines.push(String::from("NOTE:written by the io-gpeople suite"));
    }

    lines.push(String::from("END:VCARD"));
    lines.push(String::new());
    lines.join("\r\n")
}

/// Opens the TCP and TLS connection out of the environment token.
fn connect() -> GpeopleClientStd {
    env_logger::try_init().ok();

    GpeopleClientStd::connect(token(), Default::default()).expect("connect")
}

/// Milliseconds since the Unix epoch, to mint a unique tag per run.
fn unix_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
}

/// Runs `body`, then `cleanup` whichever way `body` went, and only then
/// re-raises a panic `body` may have raised.
///
/// The flow runs against a real account. Every step panics on failure,
/// so a teardown written as the last statements of the flow is skipped
/// the moment anything goes wrong, and each failed run leaves contacts
/// and groups behind for good. `body` records what it creates in
/// `state`, which `cleanup` then removes.
fn with_cleanup<S, B, C>(client: &mut GpeopleClientStd, state: &mut S, body: B, cleanup: C)
where
    B: FnOnce(&mut GpeopleClientStd, &mut S),
    C: FnOnce(&mut GpeopleClientStd, &mut S),
{
    let outcome = panic::catch_unwind(AssertUnwindSafe(|| body(client, state)));

    if panic::catch_unwind(AssertUnwindSafe(|| cleanup(client, state))).is_err() {
        eprintln!("WARNING: cleanup itself failed, the account may hold leftovers");
    }

    if let Err(payload) = outcome {
        panic::resume_unwind(payload);
    }
}

/// Reports a failed teardown without panicking, naming what was left
/// behind so it can be removed by hand.
fn report_leftover(what: &str, name: &str, err: &dyn Debug) {
    eprintln!("WARNING: could not clean up {what} `{name}`, remove it by hand: {err:?}");
}

/// The bearer token the run authenticates with.
///
/// `GPEOPLE_ACCESS_TOKEN` short-circuits everything when it is set, for
/// the token you minted by hand. Otherwise a service account key, held
/// inline in `GPEOPLE_SERVICE_ACCOUNT_KEY` or at the path
/// `GPEOPLE_SERVICE_ACCOUNT_KEY_FILE`, is traded for a fresh token acting
/// as `GPEOPLE_SERVICE_ACCOUNT_SUBJECT`.
fn token() -> String {
    if let Ok(token) = env::var("GPEOPLE_ACCESS_TOKEN") {
        return token;
    }

    if let Some(key) = env::var("GPEOPLE_SERVICE_ACCOUNT_KEY")
        .ok()
        .filter(|key| !key.is_empty())
    {
        return mint_token(&key);
    }

    if let Ok(path) = env::var("GPEOPLE_SERVICE_ACCOUNT_KEY_FILE") {
        let key = fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("cannot read the service account key at {path}: {err}"));

        return mint_token(&key);
    }

    panic!(
        "set GPEOPLE_ACCESS_TOKEN, or GPEOPLE_SERVICE_ACCOUNT_KEY / \
         GPEOPLE_SERVICE_ACCOUNT_KEY_FILE to mint one"
    );
}

/// The subset of a service account key file the JWT bearer grant needs.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
struct ServiceAccountKey {
    client_email: String,
    private_key: String,
    #[serde(default = "default_token_uri")]
    token_uri: String,
}

fn default_token_uri() -> String {
    String::from("https://oauth2.googleapis.com/token")
}

/// Signs a JWT bearer assertion with the service account key, on behalf
/// of the delegated subject, and trades it for an access token (RFC 7523
/// section 2.1).
///
/// The scopes ride in the claims, which is Google's deviation from the
/// RFC, and io-oauth models it: the token endpoint reads them from there
/// rather than from the request body.
fn mint_token(key: &str) -> String {
    let key: ServiceAccountKey =
        serde_json::from_str(key).expect("the service account key is valid JSON");
    let subject = env::var("GPEOPLE_SERVICE_ACCOUNT_SUBJECT")
        .unwrap_or_else(|_| String::from(DEFAULT_SUBJECT));

    let signer = Oauth20JwtBearerKey::from_pkcs8_pem(&key.private_key)
        .expect("the service account key holds a PKCS#8 private key");

    let token_uri: Url = key.token_uri.parse().expect("the token URI is a valid URL");

    let mut client =
        Oauth20ClientStd::connect(token_uri, &Tls::default(), key.client_email.as_str())
            .expect("connect to the token endpoint");

    let claims = Oauth20JwtBearerClaims {
        iss: key.client_email.as_str().into(),
        sub: Some(subject.into()),
        scope: GPEOPLE_SCOPES.into_iter().map(Cow::from).collect(),
        ..Default::default()
    };

    // NOTE: iat and exp come from the clock here, in the std client;
    // the coroutine layer underneath stays clock-free.
    let assertion = client
        .sign_jwt_bearer_assertion(&signer, claims, None, Duration::from_secs(600))
        .expect("sign the assertion");

    let params = Oauth20JwtBearerGrantRequestParams {
        assertion,
        scope: Default::default(),
    };

    let response = client
        .request_jwt_bearer_grant(params)
        .expect("trade the assertion for an access token");

    match response {
        Ok(granted) => granted.access_token.expose_secret().to_owned(),
        Err(err) => panic!("the token endpoint refused the assertion: {err:?}"),
    }
}
