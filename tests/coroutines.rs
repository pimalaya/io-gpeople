mod common;

use io_gpeople::{
    coroutine::*,
    v1::{
        query::{to_field_mask, to_query_pairs},
        rest::{
            contact_groups::{
                GpeopleContactGroup, GpeopleContactGroupType, GpeopleGroupField,
                create::GpeopleContactGroupCreate,
                delete::GpeopleContactGroupDelete,
                get::GpeopleContactGroupGet,
                list::{GpeopleContactGroupsList, GpeopleContactGroupsListParams},
                members::modify::GpeopleContactGroupMembersModify,
                update::GpeopleContactGroupUpdate,
            },
            other_contacts::{
                copy_other_contact_to_my_contacts_group::GpeopleOtherContactCopy,
                list::{GpeopleOtherContactsList, GpeopleOtherContactsListParams},
            },
            people::{
                GpeopleAgeRange, GpeopleContentType, GpeopleMiscKeywordType, GpeopleName,
                GpeopleNicknameType, GpeopleObjectType, GpeoplePerson, GpeoplePersonField,
                GpeopleReadSourceType, GpeopleSourceType,
                connections::list::{GpeopleConnectionsList, GpeopleConnectionsListParams},
                create_contact::GpeopleContactCreate,
                delete_contact::GpeopleContactDelete,
                delete_contact_photo::GpeopleContactPhotoDelete,
                get::GpeoplePersonGet,
                search_contacts::GpeopleContactsSearch,
                update_contact::GpeopleContactUpdate,
                update_contact_photo::GpeopleContactPhotoUpdate,
            },
        },
        send::{GpeopleApiError, GpeopleSendError, parse_api_error},
    },
};
use io_http::rfc6750::bearer::HttpAuthBearer;

use common::{empty_response, json_response, run};

fn auth() -> HttpAuthBearer {
    HttpAuthBearer::new("fake-token")
}

/// The method and path a coroutine requests, its query left out.
fn request_path<C: GpeopleCoroutine<Yield = GpeopleYield>>(mut coroutine: C) -> String {
    let (_, written) = run(&mut coroutine, &empty_response("HTTP/1.1 200 OK"));
    let request = String::from_utf8_lossy(&written);
    let line = request.lines().next().unwrap_or_default();
    let target = line.strip_suffix(" HTTP/1.1").unwrap_or(line);

    target.split('?').next().unwrap_or_default().to_string()
}

#[test]
fn lists_connections() {
    let response = json_response(
        "HTTP/1.1 200 OK",
        r#"{"connections":[{"resourceName":"people/c1","names":[{"displayName":"Jane Doe"}]}],"nextSyncToken":"sync-1","totalItems":1}"#,
    );
    let params = GpeopleConnectionsListParams {
        request_sync_token: true,
        ..Default::default()
    };
    let mut coroutine = GpeopleConnectionsList::new(
        &auth(),
        &[
            GpeoplePersonField::Names,
            GpeoplePersonField::EmailAddresses,
        ],
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
        GpeoplePersonGet::new(&auth(), "people/me", &[GpeoplePersonField::Names], &[]).unwrap();
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
    let result = GpeoplePersonGet::new(&auth(), "people/me", &[], &[]);
    assert!(matches!(result, Err(GpeopleSendError::InvalidRequest(_))));
}

#[test]
fn creates_contact() {
    let response = json_response(
        "HTTP/1.1 200 OK",
        r#"{"resourceName":"people/c1","etag":"tag-1","names":[{"givenName":"Jane"}]}"#,
    );
    let person = GpeoplePerson {
        names: vec![GpeopleName {
            given_name: Some("Jane".into()),
            family_name: Some("Doe".into()),
            ..Default::default()
        }],
        ..Default::default()
    };
    let mut coroutine =
        GpeopleContactCreate::new(&auth(), &person, &[GpeoplePersonField::Names], &[]).unwrap();
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
    let person = GpeoplePerson {
        resource_name: "people/c1".into(),
        etag: "tag-1".into(),
        names: vec![GpeopleName {
            given_name: Some("Janet".into()),
            ..Default::default()
        }],
        ..Default::default()
    };
    let mut coroutine =
        GpeopleContactUpdate::new(&auth(), &person, &[GpeoplePersonField::Names], &[], &[])
            .unwrap();
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
    let person = GpeoplePerson::default();
    let result =
        GpeopleContactUpdate::new(&auth(), &person, &[GpeoplePersonField::Names], &[], &[]);
    assert!(matches!(result, Err(GpeopleSendError::InvalidRequest(_))));
}

#[test]
fn deletes_contact() {
    let response = empty_response("HTTP/1.1 200 OK");
    let mut coroutine = GpeopleContactDelete::new(&auth(), "people/c1").unwrap();
    let (ret, written) = run(&mut coroutine, &response);

    ret.unwrap();

    let request = String::from_utf8_lossy(&written);
    assert!(
        request.starts_with("DELETE /v1/people/c1:deleteContact"),
        "got: {request}"
    );
}

#[test]
fn updates_contact_photo() {
    let response = json_response("HTTP/1.1 200 OK", "{}");
    let photo = [0xfb, 0xff];
    let mut coroutine =
        GpeopleContactPhotoUpdate::new(&auth(), "people/c1", &photo, &[], &[]).unwrap();
    let (ret, written) = run(&mut coroutine, &response);

    ret.unwrap();

    let request = String::from_utf8_lossy(&written);
    assert!(
        request.starts_with("PATCH /v1/people/c1:updateContactPhoto"),
        "got: {request}"
    );
    // NOTE: the standard alphabet, padded, not base64url.
    assert!(request.contains(r#""photoBytes":"+/8=""#), "got: {request}");
}

#[test]
fn searches_contacts() {
    let response = json_response(
        "HTTP/1.1 200 OK",
        r#"{"results":[{"person":{"resourceName":"people/c1"}}]}"#,
    );
    let mut coroutine = GpeopleContactsSearch::new(
        &auth(),
        "jane",
        &[GpeoplePersonField::Names],
        Some(10),
        &[GpeopleReadSourceType::ReadSourceTypeContact],
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
        GpeopleContactGroupsList::new(&auth(), &[], &GpeopleContactGroupsListParams::default())
            .unwrap();
    let (ret, _) = run(&mut coroutine, &response);
    let out = ret.unwrap();

    assert_eq!(out.response.contact_groups.len(), 1);
    assert_eq!(
        out.response.contact_groups[0].group_type,
        Some(GpeopleContactGroupType::SystemContactGroup)
    );
}

#[test]
fn creates_contact_group() {
    let response = json_response(
        "HTTP/1.1 200 OK",
        r#"{"resourceName":"contactGroups/abc","etag":"tag-1","name":"todo"}"#,
    );
    let group = GpeopleContactGroup {
        name: Some("todo".into()),
        ..Default::default()
    };
    let mut coroutine =
        GpeopleContactGroupCreate::new(&auth(), &group, &[GpeopleGroupField::Name]).unwrap();
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
    let group = GpeopleContactGroup {
        name: Some("  ".into()),
        ..Default::default()
    };
    let result = GpeopleContactGroupCreate::new(&auth(), &group, &[]);
    assert!(matches!(result, Err(GpeopleSendError::InvalidRequest(_))));
}

#[test]
fn deletes_contact_group_with_contacts() {
    let response = empty_response("HTTP/1.1 200 OK");
    let mut coroutine = GpeopleContactGroupDelete::new(&auth(), "contactGroups/abc", true).unwrap();
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
    let mut coroutine = GpeopleContactGroupMembersModify::new(
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
    let mut coroutine = GpeopleOtherContactsList::new(
        &auth(),
        &[GpeoplePersonField::EmailAddresses],
        &GpeopleOtherContactsListParams {
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
    let mut coroutine = GpeopleOtherContactCopy::new(
        &auth(),
        "otherContacts/o1",
        &[
            GpeoplePersonField::Names,
            GpeoplePersonField::EmailAddresses,
        ],
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
fn a_resource_name_adds_no_query() {
    let response = empty_response("HTTP/1.1 200 OK");
    let name = "contactGroups/abc?deleteContacts=true";
    let mut coroutine = GpeopleContactGroupDelete::new(&auth(), name, false).unwrap();
    let (_, written) = run(&mut coroutine, &response);

    let request = String::from_utf8_lossy(&written);
    assert!(
        request.starts_with("DELETE /v1/contactGroups/abc%3FdeleteContacts=true HTTP/1.1\r\n"),
        "got: {request}"
    );
}

#[test]
fn a_resource_id_is_one_path_segment() {
    let auth = auth();
    let fields = [GpeoplePersonField::Names];
    let id = "x?a=1#b/../c\\d";
    let segment = "x%3Fa=1%23b%2F..%2Fc%5Cd";

    let person_name = format!("people/{id}");
    let person = GpeoplePerson {
        resource_name: person_name.clone(),
        ..Default::default()
    };
    let group_name = format!("contactGroups/{id}");
    let group = GpeopleContactGroup {
        resource_name: group_name.clone(),
        ..Default::default()
    };
    let other_name = format!("otherContacts/{id}");
    let members = ["people/c1".to_string()];

    let paths = [
        request_path(GpeoplePersonGet::new(&auth, &person_name, &fields, &[]).unwrap()),
        request_path(GpeopleContactUpdate::new(&auth, &person, &fields, &[], &[]).unwrap()),
        request_path(GpeopleContactDelete::new(&auth, &person_name).unwrap()),
        request_path(GpeopleContactPhotoUpdate::new(&auth, &person_name, &[1], &[], &[]).unwrap()),
        request_path(GpeopleContactPhotoDelete::new(&auth, &person_name, &[], &[]).unwrap()),
        request_path(GpeopleContactGroupGet::new(&auth, &group_name, None, &[]).unwrap()),
        request_path(GpeopleContactGroupUpdate::new(&auth, &group, &[], &[]).unwrap()),
        request_path(GpeopleContactGroupDelete::new(&auth, &group_name, false).unwrap()),
        request_path(
            GpeopleContactGroupMembersModify::new(&auth, &group_name, &members, &[]).unwrap(),
        ),
        request_path(GpeopleOtherContactCopy::new(&auth, &other_name, &fields, &[], &[]).unwrap()),
    ];

    assert_eq!(
        paths,
        [
            format!("GET /v1/people/{segment}"),
            format!("PATCH /v1/people/{segment}:updateContact"),
            format!("DELETE /v1/people/{segment}:deleteContact"),
            format!("PATCH /v1/people/{segment}:updateContactPhoto"),
            format!("DELETE /v1/people/{segment}:deleteContactPhoto"),
            format!("GET /v1/contactGroups/{segment}"),
            format!("PUT /v1/contactGroups/{segment}"),
            format!("DELETE /v1/contactGroups/{segment}"),
            format!("POST /v1/contactGroups/{segment}/members:modify"),
            format!("POST /v1/otherContacts/{segment}:copyOtherContactToMyContactsGroup"),
        ]
    );
}

#[test]
fn rejects_a_resource_name_outside_its_collection() {
    let names = [
        "c1",
        "people",
        "people/",
        "people/.",
        "people/..",
        "../people/c1",
        "/people/c1",
        "//evil.example/people/c1",
        "https://evil.example/v1/people/c1",
        "peoplex/c1",
        "contactGroups/abc",
    ];

    for name in names {
        let result = GpeopleContactDelete::new(&auth(), name);
        assert!(
            matches!(result, Err(GpeopleSendError::InvalidRequest(_))),
            "accepted {name:?}"
        );
    }

    let result = GpeopleContactGroupDelete::new(&auth(), "people/c1", true);
    assert!(matches!(result, Err(GpeopleSendError::InvalidRequest(_))));

    let fields = [GpeoplePersonField::Names];
    let result = GpeopleOtherContactCopy::new(&auth(), "people/c1", &fields, &[], &[]);
    assert!(matches!(result, Err(GpeopleSendError::InvalidRequest(_))));
}

/// A person holding, in every enum a response fills, a value to come.
const FUTURE_PERSON: &str = r#"{"resourceName":"people/c1","etag":"tag-1","metadata":{"sources":[{"type":"FUTURE_SOURCE","id":"1"}],"objectType":"FUTURE_OBJECT"},"names":[{"metadata":{"primary":true,"source":{"type":"FUTURE_SOURCE","id":"1","profileMetadata":{"objectType":"FUTURE_OBJECT"}}},"displayName":"Jane Doe"}],"nicknames":[{"value":"Janie","type":"FUTURE_NICKNAME"}],"biographies":[{"value":"a note","contentType":"FUTURE_CONTENT"}],"miscKeywords":[{"value":"vip","type":"FUTURE_KEYWORD"}],"ageRanges":[{"ageRange":"FUTURE_RANGE"}]}"#;

#[test]
fn an_unknown_enum_value_does_not_fail_the_page() {
    let body = format!(r#"{{"connections":[{FUTURE_PERSON}],"totalItems":1}}"#);
    let response = json_response("HTTP/1.1 200 OK", &body);
    let mut coroutine = GpeopleConnectionsList::new(
        &auth(),
        &[GpeoplePersonField::Names],
        &GpeopleConnectionsListParams::default(),
    )
    .unwrap();
    let (ret, _) = run(&mut coroutine, &response);

    let person = &ret.unwrap().response.connections[0];
    assert_eq!(person.names[0].display_name.as_deref(), Some("Jane Doe"));
    assert_eq!(person.nicknames[0].value.as_deref(), Some("Janie"));
    assert_eq!(person.biographies[0].value.as_deref(), Some("a note"));
    assert_eq!(person.misc_keywords[0].value.as_deref(), Some("vip"));

    let metadata = person.metadata.as_ref().unwrap();
    assert_eq!(metadata.object_type, Some(GpeopleObjectType::Unknown));
    assert_eq!(
        metadata.sources[0].source_type,
        Some(GpeopleSourceType::Unknown)
    );

    let source = person.names[0].metadata.as_ref().unwrap().source.as_ref();
    let source = source.unwrap();
    assert_eq!(source.source_type, Some(GpeopleSourceType::Unknown));
    assert_eq!(
        source.profile_metadata.as_ref().unwrap().object_type,
        Some(GpeopleObjectType::Unknown)
    );

    assert_eq!(
        person.nicknames[0].nickname_type,
        Some(GpeopleNicknameType::Unknown)
    );
    assert_eq!(
        person.biographies[0].content_type,
        Some(GpeopleContentType::Unknown)
    );
    assert_eq!(
        person.misc_keywords[0].keyword_type,
        Some(GpeopleMiscKeywordType::Unknown)
    );
    assert_eq!(
        person.age_ranges[0].age_range,
        Some(GpeopleAgeRange::Unknown)
    );
}

#[test]
fn an_unknown_group_type_does_not_fail_the_listing() {
    let response = json_response(
        "HTTP/1.1 200 OK",
        r#"{"contactGroups":[{"resourceName":"contactGroups/abc","groupType":"FUTURE_GROUP","name":"Friends"}]}"#,
    );
    let params = GpeopleContactGroupsListParams::default();
    let mut coroutine = GpeopleContactGroupsList::new(&auth(), &[], &params).unwrap();
    let (ret, _) = run(&mut coroutine, &response);

    let group = &ret.unwrap().response.contact_groups[0];
    assert_eq!(group.name.as_deref(), Some("Friends"));
    assert_eq!(group.group_type, Some(GpeopleContactGroupType::Unknown));

    let mut coroutine = GpeopleContactGroupUpdate::new(&auth(), group, &[], &[]).unwrap();
    let (_, written) = run(&mut coroutine, &empty_response("HTTP/1.1 200 OK"));
    let request = String::from_utf8_lossy(&written);
    assert!(!request.contains("groupType"), "got: {request}");
}

#[test]
fn an_unknown_enum_value_is_never_written_back() {
    let person: GpeoplePerson = serde_json::from_str(FUTURE_PERSON).unwrap();
    let fields = [GpeoplePersonField::Nicknames];
    let mut coroutine = GpeopleContactUpdate::new(&auth(), &person, &fields, &[], &[]).unwrap();
    let (_, written) = run(&mut coroutine, &empty_response("HTTP/1.1 200 OK"));

    let request = String::from_utf8_lossy(&written);
    assert!(
        request.contains(r#""nicknames":[{"value":"Janie"}]"#),
        "got: {request}"
    );
    assert!(
        request.contains(r#""biographies":[{"value":"a note"}]"#),
        "got: {request}"
    );
    assert!(!request.contains("FUTURE"), "got: {request}");
    assert!(
        !request.to_lowercase().contains("unknown"),
        "got: {request}"
    );
}

#[test]
fn an_empty_success_body_reads_as_an_empty_object() {
    let mut coroutine = GpeopleContactPhotoDelete::new(&auth(), "people/c1", &[], &[]).unwrap();
    let (ret, _) = run(&mut coroutine, &empty_response("HTTP/1.1 200 OK"));
    assert_eq!(ret.unwrap().response.person, None);

    let mut coroutine = GpeopleContactDelete::new(&auth(), "people/c1").unwrap();
    let (ret, _) = run(&mut coroutine, &empty_response("HTTP/1.1 204 No Content"));
    ret.unwrap();
}

#[test]
fn surfaces_api_errors() {
    let response = json_response(
        "HTTP/1.1 403 Forbidden",
        r#"{"error":{"code":403,"message":"insufficient permissions"}}"#,
    );
    let mut coroutine = GpeopleConnectionsList::new(
        &auth(),
        &[GpeoplePersonField::Names],
        &GpeopleConnectionsListParams::default(),
    )
    .unwrap();
    let (ret, _) = run(&mut coroutine, &response);

    match ret.unwrap_err() {
        GpeopleSendError::Api(err) => {
            assert_eq!(err.status, 403);
            assert_eq!(err.message, "insufficient permissions");
        }
        err => panic!("unexpected error: {err}"),
    }
}

#[test]
fn an_expired_sync_token_is_told_by_its_code() {
    // NOTE: the answer People gives a sync token older than seven days.
    let response = json_response(
        "HTTP/1.1 400 Bad Request",
        r#"{"error":{"code":400,"message":"Sync token is expired. Clear local cache and retry call without the sync token.","status":"FAILED_PRECONDITION","details":[{"@type":"type.googleapis.com/google.rpc.ErrorInfo","reason":"EXPIRED_SYNC_TOKEN","domain":"people.googleapis.com"}]}}"#,
    );
    let mut coroutine = GpeopleConnectionsList::new(
        &auth(),
        &[GpeoplePersonField::Names],
        &GpeopleConnectionsListParams {
            sync_token: Some("old"),
            ..Default::default()
        },
    )
    .unwrap();
    let (ret, _) = run(&mut coroutine, &response);
    let err = ret.unwrap_err();

    assert!(err.is_sync_token_expired());
    let api = err.api().unwrap();
    assert_eq!(api.status, 400);
    assert_eq!(api.google_status.as_deref(), Some("FAILED_PRECONDITION"));
    assert_eq!(api.detail_reasons, ["EXPIRED_SYNC_TOKEN"]);
    assert!(!err.is_retryable());

    // NOTE: the older answer, a bare 410.
    let gone = GpeopleApiError::parse(410, br#"{"error":{"code":410,"message":"Gone"}}"#);
    assert!(gone.is_sync_token_expired());

    // NOTE: another 400 is not an expired token, whatever its text.
    let other = GpeopleApiError::parse(
        400,
        br#"{"error":{"code":400,"message":"Sync token is expired","status":"INVALID_ARGUMENT"}}"#,
    );
    assert!(!other.is_sync_token_expired());
}

#[test]
fn rate_limits_are_told_by_their_codes() {
    let minute = GpeopleApiError::parse(
        403,
        br#"{"error":{"code":403,"message":"Quota exceeded for quota metric 'Read requests' and limit 'Read requests per minute per user'","errors":[{"reason":"rateLimitExceeded"}],"status":"PERMISSION_DENIED","details":[{"reason":"RATE_LIMIT_EXCEEDED"}]}}"#,
    );
    assert!(minute.is_rate_limited());
    assert!(minute.is_retryable());
    assert_eq!(minute.reasons, ["rateLimitExceeded"]);

    let exhausted = GpeopleApiError::parse(
        429,
        br#"{"error":{"code":429,"message":"slow down","status":"RESOURCE_EXHAUSTED"}}"#,
    );
    assert!(exhausted.is_rate_limited());

    let daily = GpeopleApiError::parse(
        403,
        br#"{"error":{"code":403,"message":"daily","errors":[{"reason":"dailyLimitExceeded"}]}}"#,
    );
    assert!(!daily.is_rate_limited());
    assert!(!daily.is_retryable());

    let gone = GpeopleApiError::parse(404, b"<title>Not Found</title>");
    assert!(gone.is_not_found());
    assert_eq!(gone.message, "Not Found");
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
        to_field_mask(&[
            GpeoplePersonField::Names,
            GpeoplePersonField::EmailAddresses
        ]),
        "names,emailAddresses"
    );
    assert_eq!(to_field_mask::<GpeoplePersonField>(&[]), "");
}

#[test]
fn serializes_params_into_query_pairs() {
    let params = GpeopleConnectionsListParams {
        page_size: Some(10),
        page_token: None,
        request_sync_token: false,
        sync_token: Some("sync-1"),
        sort_order: None,
        sources: &[
            GpeopleReadSourceType::ReadSourceTypeContact,
            GpeopleReadSourceType::ReadSourceTypeProfile,
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
