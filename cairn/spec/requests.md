---
cairn: spec
capability: requests
status: current
---

# Requests

How the coroutines build the People REST requests they send.

### Requirement: A resource name stays in its collection

A coroutine addressing one person, contact group or other contact SHALL accept only a resource name `<collection>/<id>`, the collection being the one its method serves (`people`, `contactGroups` or `otherContacts`) and the id being neither empty, `.` nor `..`, and holding no tab and no line break. Any other name SHALL fail with `GpeopleSendError::InvalidRequest` before a request is built.

#### Scenario: A bare id

- **WHEN** `GpeopleContactDelete` is built for `c1`
- **THEN** it fails with `InvalidRequest`

#### Scenario: Another collection

- **WHEN** `GpeopleContactGroupDelete` is built for `people/c1`
- **THEN** it fails with `InvalidRequest`

#### Scenario: A dot segment hidden by a tab

- **WHEN** `GpeoplePersonGet` is built for `people/.`, a tab, `.`
- **THEN** it fails with `InvalidRequest`

### Requirement: A resource id is one path segment

The id of a resource name SHALL be percent-encoded as a single path segment under `GPEOPLE_API_BASE`, the method suffix (`:deleteContact`, `/members:modify`) following it. An id holding `/`, `?`, `#` or `\` SHALL reach the resource it names verbatim, never another path, a query or a fragment.

#### Scenario: A group name carrying a query

- **WHEN** `GpeopleContactGroupDelete` is built for `contactGroups/abc?deleteContacts=true` with `delete_contacts` false
- **THEN** its request line is `DELETE /v1/contactGroups/abc%3FdeleteContacts=true HTTP/1.1`

#### Scenario: A person name carrying a fragment

- **WHEN** `GpeopleContactDelete` is built for `people/c1#x`
- **THEN** its request path is `/v1/people/c1%23x:deleteContact`

### Requirement: A request that would split is never written

A coroutine whose request head cannot be written as built (a bearer token, a content type or another header value holding CR, LF or NUL, a method or a header name that is not a token) SHALL complete its first resume with `GpeopleSendError::Send` carrying `Http11SendError::InvalidRequest`, without having yielded a write.

#### Scenario: A token holding a line break

- **WHEN** `GpeopleContactDelete` is built with the token `fake-token`, CR, LF, `X-Injected: 1` and resumed
- **THEN** it completes with `Send(InvalidRequest(InvalidHeaderValue("Authorization")))` and no byte was written
