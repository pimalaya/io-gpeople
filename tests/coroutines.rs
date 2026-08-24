mod common;

use io_gpeople::v1::{
    query::{to_field_mask, to_query_pairs},
    rest::{
        contact_groups::{
            PeopleContactGroup, PeopleContactGroupType, PeopleGroupField,
            create::PeopleContactGroupCreate, delete::PeopleContactGroupDelete,
            list::PeopleContactGroupsList, list::PeopleContactGroupsListParams,
            members::modify::PeopleContactGroupMembersModify,
        },
        other_contacts::{
            copy_other_contact_to_my_contacts_group::PeopleOtherContactCopy,
            list::PeopleOtherContactsList, list::PeopleOtherContactsListParams,
        },
        people::{
            PeopleName, PeoplePerson, PeoplePersonField, PeopleReadSourceType,
            connections::list::PeopleConnectionsList,
            connections::list::PeopleConnectionsListParams, create_contact::PeopleContactCreate,
            delete_contact::PeopleContactDelete, get::PeoplePersonGet,
            search_contacts::PeopleContactsSearch, update_contact::PeopleContactUpdate,
        },
    },
    send::{PeopleSendError, parse_api_error},
};
use io_http::rfc6750::bearer::HttpAuthBearer;

use common::{empty_response, json_response, run};

fn auth() -> HttpAuthBearer {
    HttpAuthBearer::new("fake-token")
}

#[test]
fn lists_connections() {
    let response = json_response(
        "HTTP/1.1 200 OK",
        r#"{"connections":[{"resourceName":"people/c1","names":[{"displayName":"Jane Doe"}]}],"nextSyncToken":"sync-1","totalItems":1}"#,
    );
    let params = PeopleConnectionsListParams {
        request_sync_token: true,
        ..Default::default()
    };
    let mut coroutine = PeopleConnectionsList::new(
        &auth(),
        &[PeoplePersonField::Names, PeoplePersonField::EmailAddresses],
        &params,
    )
    .unwrap();
    let (ret, written) = run(&mut coroutine, &response);
    let out = ret.unwrap();

    assert_eq!(out.response.connections.len(), 1);
    assert_eq!(out.response.connections[0].resource_name, "people/c1");
    assert_eq!(out.response.next_sync_token.as_deref(), Some("sync-1"));

    let request = String::from_utf8_lossy(&written);
    assert!(
        request.starts_with("GET /v1/people/me/connections?"),
        "got: {request}"
    );
    assert!(
        request.contains("personFields=names%2CemailAddresses"),
        "got: {request}"
    );
    assert!(request.contains("requestSyncToken=true"), "got: {request}");
}

#[test]
fn gets_person() {
    let response = json_response(
        "HTTP/1.1 200 OK",
        r#"{"resourceName":"people/me","etag":"tag-1","names":[{"displayName":"Jane Doe","givenName":"Jane"}]}"#,
    );
    let mut coroutine =
        PeoplePersonGet::new(&auth(), "people/me", &[PeoplePersonField::Names], &[]).unwrap();
    let (ret, written) = run(&mut coroutine, &response);
    let out = ret.unwrap();

    assert_eq!(out.response.resource_name, "people/me");
    assert_eq!(out.response.names[0].given_name.as_deref(), Some("Jane"));

    let request = String::from_utf8_lossy(&written);
    assert!(
        request.starts_with("GET /v1/people/me?personFields=names"),
        "got: {request}"
    );
}

#[test]
fn rejects_empty_person_fields() {
    let result = PeoplePersonGet::new(&auth(), "people/me", &[], &[]);
    assert!(matches!(result, Err(PeopleSendError::InvalidRequest(_))));
}

#[test]
fn creates_contact() {
    let response = json_response(
        "HTTP/1.1 200 OK",
        r#"{"resourceName":"people/c1","etag":"tag-1","names":[{"givenName":"Jane"}]}"#,
    );
    let person = PeoplePerson {
        names: vec![PeopleName {
            given_name: Some("Jane".into()),
            family_name: Some("Doe".into()),
            ..Default::default()
        }],
        ..Default::default()
    };
    let mut coroutine =
        PeopleContactCreate::new(&auth(), &person, &[PeoplePersonField::Names], &[]).unwrap();
    let (ret, written) = run(&mut coroutine, &response);

    assert_eq!(ret.unwrap().response.resource_name, "people/c1");

    let request = String::from_utf8_lossy(&written);
    assert!(
        request.starts_with("POST /v1/people:createContact?personFields=names"),
        "got: {request}"
    );
    assert!(request.contains(r#""givenName":"Jane""#), "got: {request}");
    assert!(
        !request.contains("resourceName"),
        "empty resource name should not be serialized, got: {request}"
    );
}

#[test]
fn updates_contact() {
    let response = json_response(
        "HTTP/1.1 200 OK",
        r#"{"resourceName":"people/c1","etag":"tag-2","names":[{"givenName":"Janet"}]}"#,
    );
    let person = PeoplePerson {
        resource_name: "people/c1".into(),
        etag: "tag-1".into(),
        names: vec![PeopleName {
            given_name: Some("Janet".into()),
            ..Default::default()
        }],
        ..Default::default()
    };
    let mut coroutine =
        PeopleContactUpdate::new(&auth(), &person, &[PeoplePersonField::Names], &[], &[]).unwrap();
    let (ret, written) = run(&mut coroutine, &response);

    assert_eq!(ret.unwrap().response.etag, "tag-2");

    let request = String::from_utf8_lossy(&written);
    assert!(
        request.starts_with("PATCH /v1/people/c1:updateContact?updatePersonFields=names"),
        "got: {request}"
    );
    assert!(request.contains(r#""etag":"tag-1""#), "got: {request}");
}

#[test]
fn rejects_update_without_resource_name() {
    let person = PeoplePerson::default();
    let result = PeopleContactUpdate::new(&auth(), &person, &[PeoplePersonField::Names], &[], &[]);
    assert!(matches!(result, Err(PeopleSendError::InvalidRequest(_))));
}

#[test]
fn deletes_contact() {
    let response = empty_response("HTTP/1.1 200 OK");
    let mut coroutine = PeopleContactDelete::new(&auth(), "people/c1").unwrap();
    let (ret, written) = run(&mut coroutine, &response);

    ret.unwrap();

    let request = String::from_utf8_lossy(&written);
    assert!(
        request.starts_with("DELETE /v1/people/c1:deleteContact"),
        "got: {request}"
    );
}

#[test]
fn searches_contacts() {
    let response = json_response(
        "HTTP/1.1 200 OK",
        r#"{"results":[{"person":{"resourceName":"people/c1"}}]}"#,
    );
    let mut coroutine = PeopleContactsSearch::new(
        &auth(),
        "jane",
        &[PeoplePersonField::Names],
        Some(10),
        &[PeopleReadSourceType::ReadSourceTypeContact],
    )
    .unwrap();
    let (ret, written) = run(&mut coroutine, &response);
    let out = ret.unwrap();

    assert_eq!(out.response.results.len(), 1);

    let request = String::from_utf8_lossy(&written);
    assert!(
        request.starts_with("GET /v1/people:searchContacts?query=jane&readMask=names"),
        "got: {request}"
    );
    assert!(
        request.contains("sources=READ_SOURCE_TYPE_CONTACT"),
        "got: {request}"
    );
}

#[test]
fn lists_contact_groups() {
    let response = json_response(
        "HTTP/1.1 200 OK",
        r#"{"contactGroups":[{"resourceName":"contactGroups/myContacts","groupType":"SYSTEM_CONTACT_GROUP","name":"myContacts"}],"totalItems":1}"#,
    );
    let mut coroutine =
        PeopleContactGroupsList::new(&auth(), &[], &PeopleContactGroupsListParams::default())
            .unwrap();
    let (ret, _) = run(&mut coroutine, &response);
    let out = ret.unwrap();

    assert_eq!(out.response.contact_groups.len(), 1);
    assert_eq!(
        out.response.contact_groups[0].group_type,
        Some(PeopleContactGroupType::SystemContactGroup)
    );
}

#[test]
fn creates_contact_group() {
    let response = json_response(
        "HTTP/1.1 200 OK",
        r#"{"resourceName":"contactGroups/abc","etag":"tag-1","name":"todo"}"#,
    );
    let group = PeopleContactGroup {
        name: Some("todo".into()),
        ..Default::default()
    };
    let mut coroutine =
        PeopleContactGroupCreate::new(&auth(), &group, &[PeopleGroupField::Name]).unwrap();
    let (ret, written) = run(&mut coroutine, &response);

    assert_eq!(ret.unwrap().response.resource_name, "contactGroups/abc");

    let request = String::from_utf8_lossy(&written);
    assert!(
        request.starts_with("POST /v1/contactGroups"),
        "got: {request}"
    );
    assert!(
        request.contains(r#""contactGroup":{"name":"todo"}"#),
        "got: {request}"
    );
    assert!(
        request.contains(r#""readGroupFields":"name""#),
        "got: {request}"
    );
}

#[test]
fn rejects_empty_group_name() {
    let group = PeopleContactGroup {
        name: Some("  ".into()),
        ..Default::default()
    };
    let result = PeopleContactGroupCreate::new(&auth(), &group, &[]);
    assert!(matches!(result, Err(PeopleSendError::InvalidRequest(_))));
}

#[test]
fn deletes_contact_group_with_contacts() {
    let response = empty_response("HTTP/1.1 200 OK");
    let mut coroutine = PeopleContactGroupDelete::new(&auth(), "contactGroups/abc", true).unwrap();
    let (ret, written) = run(&mut coroutine, &response);

    ret.unwrap();

    let request = String::from_utf8_lossy(&written);
    assert!(
        request.starts_with("DELETE /v1/contactGroups/abc?deleteContacts=true"),
        "got: {request}"
    );
}

#[test]
fn modifies_contact_group_members() {
    let response = json_response("HTTP/1.1 200 OK", r#"{"notFoundResourceNames":[]}"#);
    let mut coroutine = PeopleContactGroupMembersModify::new(
        &auth(),
        "contactGroups/abc",
        &["people/c1".to_string()],
        &[],
    )
    .unwrap();
    let (ret, written) = run(&mut coroutine, &response);

    ret.unwrap();

    let request = String::from_utf8_lossy(&written);
    assert!(
        request.starts_with("POST /v1/contactGroups/abc/members:modify"),
        "got: {request}"
    );
    assert!(
        request.contains(r#""resourceNamesToAdd":["people/c1"]"#),
        "got: {request}"
    );
    assert!(
        !request.contains("resourceNamesToRemove"),
        "empty removal list should not be serialized, got: {request}"
    );
}

#[test]
fn lists_other_contacts() {
    let response = json_response(
        "HTTP/1.1 200 OK",
        r#"{"otherContacts":[{"resourceName":"otherContacts/o1","emailAddresses":[{"value":"jane@example.com"}]}],"nextSyncToken":"sync-1"}"#,
    );
    let mut coroutine = PeopleOtherContactsList::new(
        &auth(),
        &[PeoplePersonField::EmailAddresses],
        &PeopleOtherContactsListParams {
            request_sync_token: true,
            ..Default::default()
        },
    )
    .unwrap();
    let (ret, written) = run(&mut coroutine, &response);
    let out = ret.unwrap();

    assert_eq!(out.response.other_contacts.len(), 1);
    assert_eq!(
        out.response.other_contacts[0].email_addresses[0]
            .value
            .as_deref(),
        Some("jane@example.com")
    );

    let request = String::from_utf8_lossy(&written);
    assert!(
        request.starts_with("GET /v1/otherContacts?readMask=emailAddresses"),
        "got: {request}"
    );
}

#[test]
fn copies_other_contact() {
    let response = json_response(
        "HTTP/1.1 200 OK",
        r#"{"resourceName":"people/c9","etag":"tag-1"}"#,
    );
    let mut coroutine = PeopleOtherContactCopy::new(
        &auth(),
        "otherContacts/o1",
        &[PeoplePersonField::Names, PeoplePersonField::EmailAddresses],
        &[],
        &[],
    )
    .unwrap();
    let (ret, written) = run(&mut coroutine, &response);

    assert_eq!(ret.unwrap().response.resource_name, "people/c9");

    let request = String::from_utf8_lossy(&written);
    assert!(
        request.starts_with("POST /v1/otherContacts/o1:copyOtherContactToMyContactsGroup"),
        "got: {request}"
    );
    assert!(
        request.contains(r#""copyMask":"names,emailAddresses""#),
        "got: {request}"
    );
}

#[test]
fn surfaces_api_errors() {
    let response = json_response(
        "HTTP/1.1 403 Forbidden",
        r#"{"error":{"code":403,"message":"insufficient permissions"}}"#,
    );
    let mut coroutine = PeopleConnectionsList::new(
        &auth(),
        &[PeoplePersonField::Names],
        &PeopleConnectionsListParams::default(),
    )
    .unwrap();
    let (ret, _) = run(&mut coroutine, &response);

    match ret.unwrap_err() {
        PeopleSendError::Api { status, message } => {
            assert_eq!(status, 403);
            assert_eq!(message, "insufficient permissions");
        }
        err => panic!("unexpected error: {err}"),
    }
}

#[test]
fn parses_error_envelope() {
    let (status, message) =
        parse_api_error(400, br#"{"error":{"code":401,"message":"bad token"}}"#);
    assert_eq!(status, 401);
    assert_eq!(message, "bad token");
}

#[test]
fn falls_back_when_message_missing() {
    let (status, message) = parse_api_error(403, br#"{"error":{"code":403}}"#);
    assert_eq!(status, 403);
    assert_eq!(message, "unknown People API error");
}

#[test]
fn handles_non_json_error_body() {
    let (status, message) = parse_api_error(502, b"upstream failure");
    assert_eq!(status, 502);
    assert_eq!(message, "upstream failure");
}

#[test]
fn summarizes_an_html_error_body() {
    // NOTE: Google answers some 404s with a whole page; its title is
    // the one part worth showing.
    let html = b"<!DOCTYPE html>\n<html lang=en>\n  <title>Error 404 (Not Found)!!1</title>\n  <p>The requested URL was not found on this server.</p>\n</html>";
    let (status, message) = parse_api_error(404, html);
    assert_eq!(status, 404);
    assert_eq!(message, "Error 404 (Not Found)!!1");

    // NOTE: no title, so the markup is stripped and collapsed.
    let (_, message) = parse_api_error(500, b"<div>\n  boom  </div>\n<p>twice</p>");
    assert_eq!(message, "boom twice");

    let (_, message) = parse_api_error(500, b"   \n  ");
    assert_eq!(message, "unknown People API error");
}

#[test]
fn joins_field_masks() {
    assert_eq!(
        to_field_mask(&[PeoplePersonField::Names, PeoplePersonField::EmailAddresses]),
        "names,emailAddresses"
    );
    assert_eq!(to_field_mask::<PeoplePersonField>(&[]), "");
}

#[test]
fn serializes_params_into_query_pairs() {
    let params = PeopleConnectionsListParams {
        page_size: Some(10),
        page_token: None,
        request_sync_token: false,
        sync_token: Some("sync-1"),
        sort_order: None,
        sources: &[
            PeopleReadSourceType::ReadSourceTypeContact,
            PeopleReadSourceType::ReadSourceTypeProfile,
        ],
    };

    let pairs = to_query_pairs(&params);

    // None and the false flag vanish; the slice expands into one
    // repeated key per element; field names come from the serde rename.
    assert_eq!(
        pairs,
        vec![
            ("pageSize".to_string(), "10".to_string()),
            ("syncToken".to_string(), "sync-1".to_string()),
            (
                "sources".to_string(),
                "READ_SOURCE_TYPE_CONTACT".to_string()
            ),
            (
                "sources".to_string(),
                "READ_SOURCE_TYPE_PROFILE".to_string()
            ),
        ],
    );
}
