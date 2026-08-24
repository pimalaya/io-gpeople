#![cfg(any(
    feature = "rustls-ring",
    feature = "rustls-aws",
    feature = "native-tls"
))]
//! End-to-end People API test.
//!
//! Exercises the whole CRUD surface (contacts, contact groups, group
//! members, batch methods, "Other contacts") and cleans up everything it
//! creates, leaving the account untouched.
//!
//! Requires an OAuth2 access token with the
//! https://www.googleapis.com/auth/contacts and
//! https://www.googleapis.com/auth/contacts.other.readonly scopes:
//!
//! ```sh
//! PEOPLE_ACCESS_TOKEN="<token>" \
//! cargo test --test people -- --include-ignored
//! ```

use std::{
    env,
    time::{SystemTime, UNIX_EPOCH},
};

use io_gpeople::v1::{
    client::{PeopleClientStd, PeopleClientStdConnectOptions},
    rest::{
        contact_groups::{PeopleContactGroup, PeopleGroupField},
        other_contacts::list::PeopleOtherContactsListParams,
        people::{
            PeopleEmailAddress, PeopleName, PeoplePerson, PeoplePersonField,
            batch_create_contacts::PeopleContactsBatchCreate,
            batch_delete_contacts::PeopleContactsBatchDelete,
            batch_update_contacts::PeopleContactsBatchUpdate,
            connections::list::PeopleConnectionsListParams, get_batch_get::PeoplePersonsBatchGet,
        },
    },
};
use pimalaya_stream::tls::Tls;

#[test]
#[ignore = "requires PEOPLE_ACCESS_TOKEN env var and --include-ignored"]
fn people() {
    env_logger::try_init().ok();

    let token = env::var("PEOPLE_ACCESS_TOKEN").expect("PEOPLE_ACCESS_TOKEN not set");

    let options = PeopleClientStdConnectOptions {
        tls: Tls::default(),
    };
    let mut client = PeopleClientStd::connect(token, options).expect("connect");
    let auth = client.auth.clone();

    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let group_name = format!("io-gpeople-test-{ts}");
    let group_name_renamed = format!("{group_name}-renamed");
    let contact_name = format!("io-gpeople-test-contact-{ts}");
    let contact_name_renamed = format!("{contact_name}-renamed");

    let name_fields = &[PeoplePersonField::Names, PeoplePersonField::EmailAddresses];

    // ── CONNECTIONS LIST (baseline) ──────────────────────────────────────────

    client
        .connections_list(name_fields, &PeopleConnectionsListParams::default())
        .expect("connections list");

    // ── CONTACT GROUPS LIST (baseline) ───────────────────────────────────────

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

    // ── CONTACT GROUP CREATE ─────────────────────────────────────────────────

    let new_group = PeopleContactGroup {
        name: Some(group_name.clone()),
        ..Default::default()
    };
    let group = client
        .contact_group_create(&new_group, &[])
        .expect("contact group create")
        .response;
    let group_resource_name = group.resource_name.clone();
    assert_eq!(
        group.name.as_deref(),
        Some(group_name.as_str()),
        "created group name mismatch"
    );

    // ── CONTACT GROUP GET (verify creation) ──────────────────────────────────

    let fetched_group = client
        .contact_group_get(&group_resource_name, None, &[])
        .expect("contact group get")
        .response;
    assert_eq!(
        fetched_group.resource_name, group_resource_name,
        "contact group get resource name mismatch"
    );

    // ── CONTACT GROUP UPDATE (rename) ────────────────────────────────────────

    let renamed_group = PeopleContactGroup {
        name: Some(group_name_renamed.clone()),
        ..fetched_group
    };
    let renamed = client
        .contact_group_update(&renamed_group, &[PeopleGroupField::Name], &[])
        .expect("contact group update")
        .response;
    assert_eq!(
        renamed.name.as_deref(),
        Some(group_name_renamed.as_str()),
        "group rename not reflected"
    );

    // ── CONTACT CREATE ───────────────────────────────────────────────────────

    let new_contact = PeoplePerson {
        names: vec![PeopleName {
            given_name: Some(contact_name.clone()),
            ..Default::default()
        }],
        email_addresses: vec![PeopleEmailAddress {
            value: Some(format!("io-gpeople-test-{ts}@example.com")),
            ..Default::default()
        }],
        ..Default::default()
    };
    let contact = client
        .contact_create(&new_contact, name_fields, &[])
        .expect("contact create")
        .response;
    let contact_resource_name = contact.resource_name.clone();
    assert_eq!(
        contact.names[0].given_name.as_deref(),
        Some(contact_name.as_str()),
        "created contact name mismatch"
    );

    // ── PERSON GET (verify creation) ─────────────────────────────────────────

    let fetched_contact = client
        .person_get(&contact_resource_name, name_fields, &[])
        .expect("person get")
        .response;
    assert_eq!(
        fetched_contact.resource_name, contact_resource_name,
        "person get resource name mismatch"
    );

    // ── CONTACT UPDATE (rename) ──────────────────────────────────────────────

    let renamed_contact = PeoplePerson {
        names: vec![PeopleName {
            given_name: Some(contact_name_renamed.clone()),
            ..Default::default()
        }],
        ..fetched_contact
    };
    let renamed = client
        .contact_update(
            &renamed_contact,
            &[PeoplePersonField::Names],
            name_fields,
            &[],
        )
        .expect("contact update")
        .response;
    assert_eq!(
        renamed.names[0].given_name.as_deref(),
        Some(contact_name_renamed.as_str()),
        "contact rename not reflected"
    );

    // ── GROUP MEMBERS MODIFY (add, verify, remove) ───────────────────────────

    let modified = client
        .contact_group_members_modify(
            &group_resource_name,
            std::slice::from_ref(&contact_resource_name),
            &[],
        )
        .expect("contact group members add")
        .response;
    assert!(
        modified.not_found_resource_names.is_empty(),
        "contact should have been found when adding to the group"
    );

    let membership = client
        .contact_group_get(&group_resource_name, Some(10), &[])
        .expect("contact group get members")
        .response;
    assert!(
        membership
            .member_resource_names
            .contains(&contact_resource_name),
        "group should carry the test contact after members modify"
    );

    client
        .contact_group_members_modify(
            &group_resource_name,
            &[],
            std::slice::from_ref(&contact_resource_name),
        )
        .expect("contact group members remove");

    // ── CONTACTS BATCH CREATE, GET, UPDATE then DELETE ───────────────────────

    let batch_contacts: Vec<PeoplePerson> = (0..2)
        .map(|i| PeoplePerson {
            names: vec![PeopleName {
                given_name: Some(format!("{contact_name}-batch-{i}")),
                ..Default::default()
            }],
            ..Default::default()
        })
        .collect();
    let coroutine =
        PeopleContactsBatchCreate::new(&auth, &batch_contacts, &[PeoplePersonField::Names], &[])
            .expect("contacts batch create coroutine");
    let created = client
        .run(coroutine)
        .expect("contacts batch create")
        .response;
    assert_eq!(
        created.created_people.len(),
        2,
        "batch create should return both contacts"
    );

    let batch_resource_names: Vec<String> = created
        .created_people
        .iter()
        .filter_map(|response| response.person.as_ref())
        .map(|person| person.resource_name.clone())
        .collect();
    assert_eq!(
        batch_resource_names.len(),
        2,
        "batch create should expose both created resource names"
    );

    let coroutine = PeoplePersonsBatchGet::new(
        &auth,
        &batch_resource_names,
        &[PeoplePersonField::Names],
        &[],
    )
    .expect("persons batch get coroutine");
    let fetched = client.run(coroutine).expect("persons batch get").response;
    assert_eq!(
        fetched.responses.len(),
        2,
        "batch get should return both contacts"
    );

    let batch_renamed: Vec<PeoplePerson> = fetched
        .responses
        .iter()
        .filter_map(|response| response.person.clone())
        .map(|person| PeoplePerson {
            names: vec![PeopleName {
                given_name: person.names[0]
                    .given_name
                    .as_ref()
                    .map(|name| format!("{name}-renamed")),
                ..Default::default()
            }],
            ..person
        })
        .collect();
    let coroutine = PeopleContactsBatchUpdate::new(
        &auth,
        &batch_renamed,
        &[PeoplePersonField::Names],
        &[PeoplePersonField::Names],
        &[],
    )
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

    let coroutine = PeopleContactsBatchDelete::new(&auth, &batch_resource_names)
        .expect("contacts batch delete coroutine");
    client.run(coroutine).expect("contacts batch delete");

    // ── CONTACTS SEARCH (warmup request, as advised by the reference) ────────

    client
        .contacts_search("", name_fields, Some(10), &[])
        .expect("contacts search warmup");

    // ── OTHER CONTACTS LIST ──────────────────────────────────────────────────

    client
        .other_contacts_list(
            &[PeoplePersonField::EmailAddresses],
            &PeopleOtherContactsListParams::default(),
        )
        .expect("other contacts list");

    // ── CLEANUP: delete the contact then the group ───────────────────────────

    client
        .contact_delete(&contact_resource_name)
        .expect("contact delete");
    client
        .contact_group_delete(&group_resource_name, false)
        .expect("contact group delete");
}
