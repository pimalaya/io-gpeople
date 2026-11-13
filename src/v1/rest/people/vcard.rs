//! # vCard projection
//!
//! Projects a Google People person onto a vCard 4.0 document
//! ([`GpeoplePerson::to_vcard`]), and a vCard back onto a person
//! ([`GpeoplePerson::from_vcard`]).
//!
//! The people endpoints only speak the JSON person resource, so a
//! consumer needing a vCard (an address book client, a sync engine)
//! synthesizes the document of record here. Only fields with a
//! well-defined vCard slot are projected:
//!
//! - Google-scoped fields (external ids, keywords, locations) are minted
//!   as read-only `X-GOOGLE-*` properties and dropped on the way back, the
//!   server value staying authoritative;
//! - every other vCard line rides a `clientData` entry, the stash
//!   ([`GPEOPLE_PERSON_STASH_KEY`]), and is spliced back verbatim on read;
//! - People-only fields (fileAses, memberships, events, photos) stay out
//!   of the managed set and of every update mask, and survive untouched.
//!   A read that projects passes [`GPEOPLE_PERSON_VCARD_FIELDS`];
//! - a managed field holds more than its vCard shows (other relations, a
//!   second organization, custom types), and an update replaces it whole:
//!   [`GpeoplePerson::merge_unprojected`] re-attaches the rest first.
//!
//! Unlike Graph's fixed slots, People fields are true lists, so every
//! vCard property projects without truncation.

use core::{
    fmt::{self, Write},
    str::FromStr,
};

use alloc::{
    borrow::Cow,
    boxed::Box,
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};

use crate::v1::rest::people::{
    GpeopleAddress, GpeopleBiography, GpeopleBirthday, GpeopleClientData, GpeopleContentType,
    GpeopleDate, GpeopleEmailAddress, GpeopleImClient, GpeopleName, GpeopleNickname,
    GpeopleOccupation, GpeopleOrganization, GpeoplePerson, GpeoplePersonField, GpeoplePhoneNumber,
    GpeopleRelation, GpeopleUrl,
};
use vcard::{
    param::VcardParam,
    prop::{
        VcardProp, VcardPropKind, VcardPropName, adr::ADR, email::EMAIL, r#fn::FN, impp::IMPP,
        n::N, nickname::NICKNAME, note::NOTE, org::ORG, related::RELATED, role::ROLE, tel::TEL,
        title::TITLE, url::URL,
    },
    tree::{cst::VcardCst, error::VcardParseError, line::VcardLine, prop::lens::VcardPropLens},
    value::{
        VcardValue,
        adr::VcardAdr,
        datetime::VcardDateAndOrTime,
        n::VcardN,
        org::VcardOrg,
        text::{VcardText, VcardTextList},
        uri::VcardUri,
    },
};

/// Person fields the projection reads, the `personFields` of a read whose
/// result becomes a vCard.
///
/// The managed set, plus the Google-scoped fields minted as X-GOOGLE-*
/// properties, read-only projections that stay out of every update
/// mask.
pub const GPEOPLE_PERSON_VCARD_FIELDS: &[GpeoplePersonField] = &[
    GpeoplePersonField::Addresses,
    GpeoplePersonField::Biographies,
    GpeoplePersonField::Birthdays,
    GpeoplePersonField::ClientData,
    GpeoplePersonField::EmailAddresses,
    GpeoplePersonField::ExternalIds,
    GpeoplePersonField::ImClients,
    GpeoplePersonField::Locations,
    GpeoplePersonField::Memberships,
    GpeoplePersonField::MiscKeywords,
    GpeoplePersonField::Names,
    GpeoplePersonField::Nicknames,
    GpeoplePersonField::Occupations,
    GpeoplePersonField::Organizations,
    GpeoplePersonField::PhoneNumbers,
    GpeoplePersonField::Relations,
    GpeoplePersonField::Urls,
];

/// `clientData` key of the entry stashing the vCard remainder.
///
/// Every property the projection neither manages nor mints is preserved
/// verbatim there. The key predates this crate (Cardamum wrote it first)
/// and is kept so persons stashed then still read back.
pub const GPEOPLE_PERSON_STASH_KEY: &str = "cardamum.vcard";

/// Longest raw property line stashed server-side.
///
/// A longer line, a base64 PHOTO blob essentially, stays in the local
/// document of record rather than risking the whole write against an
/// undocumented size limit.
const MAX_STASH_LINE: usize = 8 * 1024;

/// Property names [`GpeoplePerson::to_vcard`] mints from the Google-scoped fields.
///
/// [`GpeoplePerson::from_vcard`] drops them, the server value being authoritative.
/// X-GOOGLE-MEMBERSHIP is no longer minted, memberships having become
/// structural, but stays consumed so older lines are dropped.
const MINTED_PROPS: &[&str] = &[
    "X-GOOGLE-MEMBERSHIP",
    "X-GOOGLE-EXTERNAL-ID",
    "X-GOOGLE-MISC-KEYWORD",
    "X-GOOGLE-LOCATION",
];

impl GpeoplePerson {
    /// Projects an io-gpeople person onto a fresh vCard 4.0 document.
    ///
    /// The UID is the one the stash carries ([`stashed_uid`]), else one minted
    /// from the person id for a person Google created itself. Typed fields
    /// carry their home/work TYPE (phones also mobile as cell), and spouse and
    /// children relations become RELATED names.
    ///
    /// [`stashed_uid`]: Self::stashed_uid
    pub fn to_vcard(&self) -> String {
        let person = self;
        let mut card = VcardCst::v4();

        let id = person.id();
        if person.stashed_uid().is_none() && !id.is_empty() {
            card.push(VcardProp::text(VcardPropKind::Uid, vec![], id));
        }

        card.push(VcardProp::text(
            VcardPropKind::Fn,
            vec![],
            display_name(person),
        ));

        if let Some(name) = person.names.first() {
            let n = VcardN {
                family: component(&name.family_name),
                given: component(&name.given_name),
                additional: component(&name.middle_name),
                prefixes: component(&name.honorific_prefix),
                suffixes: component(&name.honorific_suffix),
            };

            let empty = n.family.is_empty()
                && n.given.is_empty()
                && n.additional.is_empty()
                && n.prefixes.is_empty()
                && n.suffixes.is_empty();
            if !empty {
                card.push(VcardProp {
                    group: None,
                    name: VcardPropName::Kind(VcardPropKind::N),
                    params: vec![],
                    value: VcardValue::N(n),
                });
            }
        }

        for nickname in &person.nicknames {
            if let Some(nick) = opt(&nickname.value) {
                card.push(VcardProp {
                    group: None,
                    name: VcardPropName::Kind(VcardPropKind::Nickname),
                    params: vec![],
                    value: VcardValue::TextList(VcardTextList(vec![Cow::Owned(nick.to_string())])),
                });
            }
        }

        for email in &person.email_addresses {
            if let Some(address) = opt(&email.value) {
                let params = std_type(&email.email_type).map(type_param).into_iter();
                card.push(VcardProp::text(
                    VcardPropKind::Email,
                    params.collect(),
                    address,
                ));
            }
        }

        for im in &person.im_clients {
            if let Some(username) = opt(&im.username) {
                let uri = match opt(&im.protocol) {
                    Some(protocol) => format!("{protocol}:{username}"),
                    None => username.to_string(),
                };
                card.push(VcardProp {
                    group: None,
                    name: VcardPropName::Kind(VcardPropKind::Impp),
                    params: vec![],
                    value: uri_value(&uri),
                });
            }
        }

        for phone in &person.phone_numbers {
            if let Some(number) = opt(&phone.value) {
                let params = tel_type(&phone.phone_type).map(type_param).into_iter();
                card.push(VcardProp::text(
                    VcardPropKind::Tel,
                    params.collect(),
                    number,
                ));
            }
        }

        for address in &person.addresses {
            if let Some(prop) = adr_prop(address) {
                card.push(prop);
            }
        }

        if let Some(org) = person.organizations.first() {
            let company = opt(&org.name);
            let department = opt(&org.department);
            if company.is_some() || department.is_some() {
                let mut components: Vec<Cow<'static, str>> =
                    vec![Cow::Owned(company.unwrap_or_default().to_string())];
                if let Some(department) = department {
                    components.push(Cow::Owned(department.to_string()));
                }
                card.push(VcardProp {
                    group: None,
                    name: VcardPropName::Kind(VcardPropKind::Org),
                    params: vec![],
                    value: VcardValue::Org(VcardOrg(components)),
                });
            }

            if let Some(title) = opt(&org.title) {
                card.push(VcardProp::text(VcardPropKind::Title, vec![], title));
            }
        }

        if let Some(role) = person.occupations.first().and_then(|role| opt(&role.value)) {
            card.push(VcardProp::text(VcardPropKind::Role, vec![], role));
        }

        for url in &person.urls {
            if let Some(page) = opt(&url.value) {
                card.push(VcardProp {
                    group: None,
                    name: VcardPropName::Kind(VcardPropKind::Url),
                    params: vec![],
                    value: uri_value(page),
                });
            }
        }

        // NOTE: People dates can be partial (year-less birthdays); only a
        // full date has a portable vCard slot, see project::full_date.
        if let Some(date) = person.birthdays.first().and_then(|birthday| {
            let date = birthday.date.as_ref()?;
            Some(format!(
                "{:04}-{:02}-{:02}",
                date.year?, date.month?, date.day?
            ))
        }) {
            card.push(VcardProp {
                group: None,
                name: VcardPropName::Kind(VcardPropKind::Bday),
                params: vec![],
                value: VcardValue::DateAndOrTime(VcardDateAndOrTime(Cow::Owned(date))),
            });
        }

        // NOTE: HTML biographies come from Google profiles, not contacts;
        // they have no plain-text slot and are skipped rather than mangled.
        if let Some(notes) = person
            .biographies
            .iter()
            .find(|bio| bio.content_type != Some(GpeopleContentType::TextHtml))
            .and_then(|bio| opt(&bio.value))
        {
            card.push(VcardProp::text(VcardPropKind::Note, vec![], notes));
        }

        for relation in &person.relations {
            if let Some(name) = opt(&relation.person) {
                match opt(&relation.relation_type) {
                    Some("spouse") => {
                        card.push(related_prop("spouse", name));
                    }
                    Some("child") => {
                        card.push(related_prop("child", name));
                    }
                    _ => {}
                }
            }
        }

        for prop in minted_props(person) {
            card.push(prop);
        }

        // NOTE: an empty FN was once stashed for a person without a name.
        let stash = stash_lines(person);
        for line in stash.iter().filter(|line| line.as_str() != "FN:") {
            // NOTE: a line that no longer tokenises restores nothing rather
            // than corrupting the card.
            let _ = card.push_raw(line);
        }

        String::from_utf8_lossy(&card.to_bytes()).into_owned()
    }

    /// Projects a vCard onto an io-gpeople person, in full state.
    ///
    /// Every managed field carries the vCard's values, empty when the vCard
    /// drops the property, which clears the masked field on update. Lines
    /// that do not project are stashed in clientData and restore on read,
    /// the UID among them, People having no UID field: that is what keeps a
    /// person's identity across a sync with another source. A line too long
    /// for the stash is named in [`GpeoplePersonVcard::dropped`].
    pub fn from_vcard(vcard: &str) -> Result<GpeoplePersonVcard, GpeoplePersonVcardError> {
        let card = VcardCst::parse(vcard).map_err(GpeoplePersonVcardError::Parse)?;
        let version = card.version();

        let mut person = GpeoplePerson::default();
        let mut name = GpeopleName::default();
        let mut org = GpeopleOrganization::default();
        let mut notes = Vec::new();
        let mut stash = Vec::new();
        let mut dropped = Vec::new();
        let mut name_seen = false;

        for line in &card.props {
            let consumed = match VcardPropKind::from_str(line.name.get()) {
                // NOTE: the VERSION line is structural and the minted
                // X-GOOGLE-* properties are read-only projections; neither
                // belongs to the remainder.
                Err(_) => {
                    let raw_name = line.name.get();
                    raw_name.eq_ignore_ascii_case("VERSION")
                        || MINTED_PROPS
                            .iter()
                            .any(|prop| raw_name.eq_ignore_ascii_case(prop))
                }
                Ok(VcardPropKind::Fn) => {
                    // NOTE: an empty FN is the one a nameless person gets.
                    let value = FN::decode(line, version);
                    let empty = value.0.trim().is_empty();
                    set_first(&mut name.unstructured_name, &value.0) || empty
                }
                Ok(VcardPropKind::N) => {
                    if !name_seen {
                        name_seen = true;
                        let n = N::decode(line, version);
                        set_first(&mut name.family_name, n.family.join(" "));
                        set_first(&mut name.given_name, n.given.join(" "));
                        set_first(&mut name.middle_name, n.additional.join(" "));
                        set_first(&mut name.honorific_prefix, n.prefixes.join(" "));
                        set_first(&mut name.honorific_suffix, n.suffixes.join(" "));
                        true
                    } else {
                        false
                    }
                }
                Ok(VcardPropKind::Nickname) => {
                    let all = NICKNAME::decode(line, version);
                    let mut pushed = false;
                    for nick in &all.0 {
                        let nick = nick.trim();
                        if !nick.is_empty() {
                            person.nicknames.push(GpeopleNickname {
                                value: Some(nick.to_string()),
                                ..Default::default()
                            });
                            pushed = true;
                        }
                    }
                    pushed
                }
                Ok(VcardPropKind::Email) => {
                    let email = EMAIL::decode(line, version);
                    let address = email.0.trim();
                    if !address.is_empty() {
                        person.email_addresses.push(GpeopleEmailAddress {
                            value: Some(address.to_string()),
                            email_type: std_type_of(&type_values(line)),
                            ..Default::default()
                        });
                        true
                    } else {
                        false
                    }
                }
                Ok(VcardPropKind::Impp) => {
                    let impp = IMPP::decode(line, version);
                    let address = impp.0.trim();
                    if !address.is_empty() {
                        let (protocol, username) = match address.split_once(':') {
                            Some((protocol, username)) => (Some(protocol.to_string()), username),
                            None => (None, address),
                        };
                        person.im_clients.push(GpeopleImClient {
                            username: Some(username.to_string()),
                            protocol,
                            ..Default::default()
                        });
                        true
                    } else {
                        false
                    }
                }
                Ok(VcardPropKind::Tel) => {
                    let tel = TEL::decode(line, version);
                    let number = tel.0.trim();
                    if !number.is_empty() {
                        person.phone_numbers.push(GpeoplePhoneNumber {
                            value: Some(number.to_string()),
                            phone_type: tel_type_of(&type_values(line)),
                            ..Default::default()
                        });
                        true
                    } else {
                        false
                    }
                }
                Ok(VcardPropKind::Adr) => {
                    let adr = ADR::decode(line, version);

                    let address = GpeopleAddress {
                        po_box: joined(&adr.po_box),
                        extended_address: joined(&adr.extended),
                        // NOTE: People's street is one multiline field, so
                        // each street component becomes one of its lines.
                        street_address: {
                            let street: Vec<&str> = adr
                                .street
                                .iter()
                                .map(|component| component.as_ref().trim())
                                .filter(|component| !component.is_empty())
                                .collect();
                            (!street.is_empty()).then(|| street.join("\n"))
                        },
                        city: joined(&adr.locality),
                        region: joined(&adr.region),
                        postal_code: joined(&adr.postal_code),
                        country: joined(&adr.country),
                        address_type: std_type_of(&type_values(line)),
                        ..Default::default()
                    };

                    let empty = address.po_box.is_none()
                        && address.extended_address.is_none()
                        && address.street_address.is_none()
                        && address.city.is_none()
                        && address.region.is_none()
                        && address.postal_code.is_none()
                        && address.country.is_none();
                    if !empty {
                        person.addresses.push(address);
                        true
                    } else {
                        false
                    }
                }
                Ok(VcardPropKind::Org) => {
                    if org.name.is_none() && org.department.is_none() {
                        let value = ORG::decode(line, version);
                        let mut components = value.0.iter().map(|component| component.as_ref());
                        let company = components.next().unwrap_or_default();
                        let rest = components.collect::<Vec<_>>().join(" ");
                        set_first(&mut org.name, company);
                        set_first(&mut org.department, rest);
                        true
                    } else {
                        false
                    }
                }
                Ok(VcardPropKind::Title) => {
                    let value = TITLE::decode(line, version);
                    set_first(&mut org.title, &value.0)
                }
                Ok(VcardPropKind::Role) => {
                    let role = ROLE::decode(line, version);
                    let role = role.0.trim();
                    if person.occupations.is_empty() && !role.is_empty() {
                        person.occupations.push(GpeopleOccupation {
                            value: Some(role.to_string()),
                            ..Default::default()
                        });
                        true
                    } else {
                        false
                    }
                }
                Ok(VcardPropKind::Url) => {
                    let url = URL::decode(line, version);
                    let page = url.0.trim();
                    if !page.is_empty() {
                        person.urls.push(GpeopleUrl {
                            value: Some(page.to_string()),
                            ..Default::default()
                        });
                        true
                    } else {
                        false
                    }
                }
                Ok(VcardPropKind::Bday) => {
                    // NOTE: a partial (year-less) birthday has no People
                    // date it round-trips through; it lands in the stash.
                    if person.birthdays.is_empty()
                        && let Some(date) =
                            VcardDateAndOrTime::from(line.raw_value_str()).full_date()
                    {
                        let mut parts = date.split('-').map(|part| part.parse().ok());
                        person.birthdays.push(GpeopleBirthday {
                            date: Some(GpeopleDate {
                                year: parts.next().flatten(),
                                month: parts.next().flatten(),
                                day: parts.next().flatten(),
                            }),
                            ..Default::default()
                        });
                        true
                    } else {
                        false
                    }
                }
                Ok(VcardPropKind::Note) => {
                    let note = NOTE::decode(line, version);
                    let note = note.0.trim();
                    if !note.is_empty() {
                        notes.push(note.to_string());
                        true
                    } else {
                        false
                    }
                }
                Ok(VcardPropKind::Related) => {
                    // NOTE: only free-form spouse and child names
                    // (VALUE=text) project; other types and URI RELATED
                    // lines land in the stash.
                    let text = line.params.iter().any(|param| {
                        param.name.get().eq_ignore_ascii_case("VALUE")
                            && param
                                .values
                                .iter()
                                .any(|value| value.get().eq_ignore_ascii_case("text"))
                    });
                    let related = RELATED::decode(line, version);
                    let related = related.0.trim();
                    let types = type_values(line);
                    let relation_type = if types.iter().any(|t| t == "spouse") {
                        Some("spouse")
                    } else if types.iter().any(|t| t == "child") {
                        Some("child")
                    } else {
                        None
                    };

                    if let Some(relation_type) = relation_type
                        && text
                        && !related.is_empty()
                    {
                        person.relations.push(GpeopleRelation {
                            person: Some(related.to_string()),
                            relation_type: Some(relation_type.to_string()),
                            ..Default::default()
                        });
                        true
                    } else {
                        false
                    }
                }
                Ok(_) => false,
            };

            if !consumed {
                // NOTE: Display would replay the folds the line arrived with.
                let mut raw = line.name.get().to_string();
                for param in &line.params {
                    let _ = write!(raw, ";{param}");
                }
                let _ = write!(raw, ":{}", line.value);

                if raw.len() <= MAX_STASH_LINE {
                    stash.push(raw);
                } else {
                    dropped.push(property_name(&raw));
                }
            }
        }

        if !stash.is_empty() {
            person.client_data = vec![GpeopleClientData {
                key: Some(GPEOPLE_PERSON_STASH_KEY.to_string()),
                value: Some(stash.join("\n")),
                ..Default::default()
            }];
        }

        let empty = name.unstructured_name.is_none()
            && name.family_name.is_none()
            && name.given_name.is_none()
            && name.middle_name.is_none()
            && name.honorific_prefix.is_none()
            && name.honorific_suffix.is_none();
        if !empty {
            person.names.push(name);
        }

        if org.name.is_some() || org.department.is_some() || org.title.is_some() {
            person.organizations.push(org);
        }

        if !notes.is_empty() {
            person.biographies.push(GpeopleBiography {
                value: Some(notes.join("\n")),
                content_type: Some(GpeopleContentType::TextPlain),
                ..Default::default()
            });
        }

        Ok(GpeoplePersonVcard { person, dropped })
    }

    /// The stashed properties `base` carries and this person cannot remove.
    ///
    /// The stash is a `clientData` entry People refuses to empty: an update
    /// with a new value replaces it, one without leaves the old value
    /// standing, whatever the mask says. Naming them is all a client can do.
    pub fn unremovable_properties(&self, base: &Self) -> Vec<String> {
        let person = self;
        let kept: Vec<String> = stash_lines(person)
            .iter()
            .map(|line| property_name(line))
            .collect();

        stash_lines(base)
            .iter()
            .map(|line| property_name(line))
            .filter(|name| !kept.contains(name))
            .collect()
    }

    /// The managed fields whose projection differs from `base`.
    ///
    /// The update mask shrinks to them, so unchanged fields are neither
    /// replaced nor clobbered by a concurrent edit.
    pub fn changed_fields(&self, base: &Self) -> Vec<GpeoplePersonField> {
        let person = self;
        let mut fields = Vec::new();

        macro_rules! push_changed {
        ($($field:ident => $variant:ident),* $(,)?) => {$(
            if person.$field != base.$field {
                fields.push(GpeoplePersonField::$variant);
            }
        )*};
    }

        push_changed!(
            addresses => Addresses,
            biographies => Biographies,
            birthdays => Birthdays,
            client_data => ClientData,
            email_addresses => EmailAddresses,
            im_clients => ImClients,
            names => Names,
            nicknames => Nicknames,
            occupations => Occupations,
            organizations => Organizations,
            phone_numbers => PhoneNumbers,
            relations => Relations,
            urls => Urls,
        );

        fields
    }

    /// Re-attaches what this person, built from a vCard, cannot hold of
    /// `current`, in each field of the update mask `fields`.
    ///
    /// An update replaces a masked field whole, so the entries and sub-fields
    /// a vCard does not show would be erased. Call it once the mask is known.
    pub fn merge_unprojected(&mut self, current: &Self, fields: &[GpeoplePersonField]) {
        macro_rules! overlay {
            ($($facet:ident),+) => {
                |kept, ours, shown| {$(
                    if ours.$facet != shown.$facet {
                        kept.$facet = ours.$facet.clone();
                    }
                )+}
            };
        }

        macro_rules! merge_list {
            ($list:ident, first: $first:expr, $same:expr, $($facet:ident),+) => {
                merge_list(
                    &mut self.$list,
                    &current.$list,
                    |i, entry| {
                        let hidden = $first && i > 0;
                        (!hidden).then(|| shown(entry, |person| &mut person.$list))?
                    },
                    $same,
                    overlay!($($facet),+),
                )
            };
        }

        macro_rules! merge_one {
            ($list:ident, $theirs:expr, $($facet:ident),+) => {{
                let theirs = $theirs;
                let shown = theirs.and_then(|entry| shown(entry, |person| &mut person.$list));
                merge_one(&mut self.$list, theirs, shown, overlay!($($facet),+))
            }};
        }

        for field in fields {
            match field {
                GpeoplePersonField::Addresses => merge_list!(
                    addresses,
                    first: false,
                    |_, _| false,
                    po_box,
                    extended_address,
                    street_address,
                    city,
                    region,
                    postal_code,
                    country,
                    address_type
                ),
                GpeoplePersonField::Biographies => merge_one!(
                    biographies,
                    current
                        .biographies
                        .iter()
                        .find(|bio| bio.content_type != Some(GpeopleContentType::TextHtml)),
                    value,
                    content_type
                ),
                GpeoplePersonField::Birthdays => {
                    merge_one!(birthdays, current.birthdays.first(), date)
                }
                GpeoplePersonField::ClientData => {
                    let others = current
                        .client_data
                        .iter()
                        .filter(|entry| entry.key.as_deref() != Some(GPEOPLE_PERSON_STASH_KEY));

                    for entry in others {
                        if !self.client_data.contains(entry) {
                            self.client_data.push(entry.clone());
                        }
                    }
                }
                GpeoplePersonField::EmailAddresses => merge_list!(
                    email_addresses,
                    first: false,
                    |ours, shown| ours.value == shown.value,
                    value,
                    email_type
                ),
                GpeoplePersonField::ImClients => merge_list!(
                    im_clients,
                    first: false,
                    |ours, shown| ours.username == shown.username,
                    username,
                    protocol
                ),
                GpeoplePersonField::Names => merge_one!(
                    names,
                    current.names.first(),
                    unstructured_name,
                    family_name,
                    given_name,
                    middle_name,
                    honorific_prefix,
                    honorific_suffix
                ),
                GpeoplePersonField::Nicknames => merge_list!(
                    nicknames,
                    first: false,
                    |ours, shown| ours.value == shown.value,
                    value
                ),
                GpeoplePersonField::Occupations => {
                    merge_list!(occupations, first: true, |_, _| false, value)
                }
                GpeoplePersonField::Organizations => merge_list!(
                    organizations,
                    first: true,
                    |_, _| false,
                    name,
                    department,
                    title
                ),
                GpeoplePersonField::PhoneNumbers => merge_list!(
                    phone_numbers,
                    first: false,
                    |ours, shown| ours.value == shown.value,
                    value,
                    phone_type
                ),
                GpeoplePersonField::Relations => merge_list!(
                    relations,
                    first: false,
                    |ours, shown| ours.person == shown.person,
                    person,
                    relation_type
                ),
                GpeoplePersonField::Urls => merge_list!(
                    urls,
                    first: false,
                    |ours, shown| ours.value == shown.value,
                    value
                ),
                _ => {}
            }
        }
    }
}

/// The property name of a stashed vCard line, up to its first `;` or `:`.
fn property_name(line: &str) -> String {
    let end = line.find([';', ':']).unwrap_or(line.len());
    line[..end].to_uppercase()
}

/// A server entry as its vCard reads back, None when the vCard hides it.
fn shown<T: Clone>(entry: &T, list: fn(&mut GpeoplePerson) -> &mut Vec<T>) -> Option<T> {
    let mut alone = GpeoplePerson::default();
    list(&mut alone).push(entry.clone());

    let mut back = GpeoplePerson::from_vcard(&alone.to_vcard()).ok()?.person;
    list(&mut back).pop()
}

/// Merges the server list `theirs` into `ours`, the list a vCard gave.
///
/// An entry of `ours` is paired with the server entry it shows (the same
/// projection, else the same key, else the only one left on each side) and
/// keeps what that entry holds beyond the vCard. Hidden entries follow.
fn merge_list<T: Clone + PartialEq>(
    ours: &mut Vec<T>,
    theirs: &[T],
    shown: impl Fn(usize, &T) -> Option<T>,
    same: impl Fn(&T, &T) -> bool,
    overlay: impl Fn(&mut T, &T, &T),
) {
    let shown: Vec<Option<T>> = theirs
        .iter()
        .enumerate()
        .map(|(i, entry)| shown(i, entry))
        .collect();

    let mut pairs: Vec<Option<usize>> = vec![None; ours.len()];
    let mut paired = vec![false; theirs.len()];

    for exact in [true, false] {
        for (entry, pair) in ours.iter().zip(&mut pairs) {
            if pair.is_some() {
                continue;
            }

            *pair = shown.iter().enumerate().position(|(i, shown)| {
                let Some(shown) = shown else {
                    return false;
                };

                !paired[i] && (exact && entry == shown || !exact && same(entry, shown))
            });

            if let Some(i) = *pair {
                paired[i] = true;
            }
        }
    }

    let mut left_ours = (0..pairs.len()).filter(|&j| pairs[j].is_none());
    let mut left_theirs = (0..theirs.len()).filter(|&i| shown[i].is_some() && !paired[i]);

    let left = (
        left_ours.next(),
        left_ours.next(),
        left_theirs.next(),
        left_theirs.next(),
    );

    if let (Some(j), None, Some(i), None) = left {
        pairs[j] = Some(i);
    }

    let mut merged = Vec::with_capacity(theirs.len());

    for (entry, pair) in ours.drain(..).zip(pairs) {
        let pair = pair.and_then(|i| Some((&theirs[i], shown[i].as_ref()?)));

        merged.push(match pair {
            Some((theirs, shown)) => {
                let mut kept = theirs.clone();
                overlay(&mut kept, &entry, shown);
                kept
            }
            None => entry,
        });
    }

    for (entry, shown) in theirs.iter().zip(&shown) {
        if shown.is_none() && !merged.contains(entry) {
            merged.push(entry.clone());
        }
    }

    *ours = merged;
}

/// [`merge_list`] for a field People holds one entry of.
///
/// `theirs` is the entry the vCard shows as `shown`, or hides.
fn merge_one<T: Clone + Default>(
    ours: &mut Vec<T>,
    theirs: Option<&T>,
    shown: Option<T>,
    overlay: impl Fn(&mut T, &T, &T),
) {
    let Some(theirs) = theirs else {
        return;
    };

    match ours.first() {
        Some(entry) => {
            let mut kept = theirs.clone();
            overlay(&mut kept, entry, &shown.unwrap_or_default());
            *ours = vec![kept];
        }
        None if shown.is_none() => ours.push(theirs.clone()),
        None => {}
    }
}

/// A vCard projected onto a People person.
#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub struct GpeoplePersonVcard {
    /// The person the vCard projects onto.
    pub person: GpeoplePerson,
    /// Names of the properties too long for the stash, absent from `person`.
    pub dropped: Vec<String>,
}

/// A vCard that cannot become a People person.
#[derive(Debug)]
pub enum GpeoplePersonVcardError {
    /// The vCard does not parse.
    Parse(VcardParseError),
}

impl fmt::Display for GpeoplePersonVcardError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse(err) => write!(f, "Invalid vCard: {err}"),
        }
    }
}

impl core::error::Error for GpeoplePersonVcardError {}

/// The person's display name.
///
/// The server-formatted one, else the unstructured one, else the name
/// parts joined.
fn display_name(person: &GpeoplePerson) -> String {
    let Some(name) = person.names.first() else {
        return String::new();
    };

    if let Some(display) = opt(&name.display_name) {
        return display.to_string();
    }
    if let Some(unstructured) = opt(&name.unstructured_name) {
        return unstructured.to_string();
    }

    let composed: Vec<&str> = [&name.given_name, &name.middle_name, &name.family_name]
        .into_iter()
        .filter_map(opt)
        .collect();
    composed.join(" ")
}

/// An ADR property from a People address, None when it is empty.
///
/// People's street is one multiline field, so each of its lines becomes
/// a vCard street component.
fn adr_prop(address: &GpeopleAddress) -> Option<VcardProp<'static>> {
    let street = address.street_address.as_deref().unwrap_or("");
    let value = VcardAdr {
        po_box: option_component(&address.po_box),
        extended: option_component(&address.extended_address),
        street: street
            .split('\n')
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(|line| Cow::Owned(line.to_string()))
            .collect(),
        locality: option_component(&address.city),
        region: option_component(&address.region),
        postal_code: option_component(&address.postal_code),
        country: option_component(&address.country),
        ..Default::default()
    };

    let empty = value.po_box.is_empty()
        && value.extended.is_empty()
        && value.street.is_empty()
        && value.locality.is_empty()
        && value.region.is_empty()
        && value.postal_code.is_empty()
        && value.country.is_empty();
    if empty {
        return None;
    }

    Some(VcardProp {
        group: None,
        name: VcardPropName::Kind(VcardPropKind::Adr),
        params: std_type(&address.address_type)
            .map(type_param)
            .into_iter()
            .collect(),
        value: VcardValue::Adr(Box::new(value)),
    })
}

/// A RELATED name property (spouse, child).
///
/// The names are free-form text, hence the explicit VALUE=text:
/// RELATED defaults to a URI.
fn related_prop(r#type: &'static str, name: &str) -> VcardProp<'static> {
    VcardProp {
        group: None,
        name: VcardPropName::Kind(VcardPropKind::Related),
        params: vec![type_param(r#type), VcardParam::Value(Cow::Borrowed("text"))],
        value: VcardValue::Text(VcardText(Cow::Owned(name.to_string()))),
    }
}

/// A URI value, its control characters percent-encoded.
///
/// A URI is written unescaped, so a raw line break would end its line.
fn uri_value(uri: &str) -> VcardValue<'static> {
    let mut value = String::with_capacity(uri.len());

    for c in uri.chars() {
        if c.is_control() {
            for byte in c.encode_utf8(&mut [0; 4]).bytes() {
                let _ = write!(value, "%{byte:02X}");
            }
        } else {
            value.push(c);
        }
    }

    VcardValue::Uri(VcardUri(Cow::Owned(value)))
}

/// A single-value TYPE parameter.
fn type_param(value: &'static str) -> VcardParam<'static> {
    VcardParam::Type(vec![Cow::Borrowed(value)])
}

/// The vCard TYPE behind a People home/work field type.
fn std_type(field_type: &Option<String>) -> Option<&'static str> {
    match opt(field_type)? {
        "home" => Some("home"),
        "work" => Some("work"),
        _ => None,
    }
}

/// The People field type behind vCard home/work TYPE values.
fn std_type_of(types: &[String]) -> Option<String> {
    if types.iter().any(|t| t == "home") {
        Some("home".to_string())
    } else if types.iter().any(|t| t == "work") {
        Some("work".to_string())
    } else {
        None
    }
}

/// The vCard TYPE behind a People phone type (mobile maps to cell).
fn tel_type(phone_type: &Option<String>) -> Option<&'static str> {
    match opt(phone_type)? {
        "mobile" => Some("cell"),
        "home" => Some("home"),
        "work" => Some("work"),
        _ => None,
    }
}

/// The People phone type behind vCard TYPE values (cell maps to mobile).
fn tel_type_of(types: &[String]) -> Option<String> {
    if types.iter().any(|t| t == "cell") {
        Some("mobile".to_string())
    } else if types.iter().any(|t| t == "home") {
        Some("home".to_string())
    } else if types.iter().any(|t| t == "work") {
        Some("work".to_string())
    } else {
        None
    }
}

/// The trimmed field, None when unset or blank.
fn opt(field: &Option<String>) -> Option<&str> {
    let value = field.as_deref()?.trim();
    if value.is_empty() { None } else { Some(value) }
}

/// A structured-value component holding the trimmed field, or empty.
fn component(field: &Option<String>) -> Vec<Cow<'static, str>> {
    match opt(field) {
        Some(value) => vec![Cow::Owned(value.to_string())],
        None => Vec::new(),
    }
}

/// Like [`component`], in the People to vCard direction.
fn option_component(field: &Option<String>) -> Vec<Cow<'static, str>> {
    component(field)
}

/// Fills a single-instance People field, true when it took the slot.
fn set_first(slot: &mut Option<String>, value: impl AsRef<str>) -> bool {
    let value = value.as_ref().trim();
    if slot.is_none() && !value.is_empty() {
        *slot = Some(value.to_string());
        true
    } else {
        false
    }
}

/// X-GOOGLE-* lines minted from the read-only Google-scoped fields.
///
/// External ids, keywords and locations mean nothing outside their
/// account, so they ride as vendor properties (cairn/spec/projection.md).
/// Memberships are addressbook data (cairn/spec/addressbooks.md), not minted.
fn minted_props(person: &GpeoplePerson) -> Vec<VcardProp<'static>> {
    let mut lines = Vec::new();

    for id in &person.external_ids {
        if let Some(value) = opt(&id.value) {
            lines.push(typed_line("X-GOOGLE-EXTERNAL-ID", &id.id_type, value));
        }
    }

    for keyword in &person.misc_keywords {
        if let Some(value) = opt(&keyword.value) {
            // NOTE: the serde wire name of the keyword type enum is the
            // canonical People spelling (HOME, OUTLOOK_KEYWORD, ...).
            let keyword_type = keyword
                .keyword_type
                .and_then(|t| serde_json::to_value(t).ok())
                .and_then(|v| v.as_str().map(str::to_string));
            lines.push(typed_line("X-GOOGLE-MISC-KEYWORD", &keyword_type, value));
        }
    }

    for location in &person.locations {
        if let Some(value) = opt(&location.value) {
            lines.push(typed_line(
                "X-GOOGLE-LOCATION",
                &location.location_type,
                value,
            ));
        }
    }

    lines
}

/// A minted property with an optional TYPE parameter.
///
/// The type rides along only when it is a plain token a parameter value
/// holds unquoted.
fn typed_line(name: &'static str, r#type: &Option<String>, value: &str) -> VcardProp<'static> {
    let token = opt(r#type).filter(|t| {
        t.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    });

    let params = match token {
        Some(token) => vec![VcardParam::Type(vec![Cow::Owned(token.to_string())])],
        None => vec![],
    };

    VcardProp::text(name, params, value.to_string())
}

impl GpeoplePerson {
    /// The vCard UID the stash carries, `None` for a person no vCard was
    /// ever written to, People having no UID field of its own.
    ///
    /// A sync engine checks it after a write: a server that dropped the
    /// stash would hand the person back with an identity minted from its
    /// resource name rather than the UID it was written under.
    pub fn stashed_uid(&self) -> Option<String> {
        stash_lines(self).iter().find_map(|line| {
            let mut bytes = line.clone();
            bytes.push_str("\r\n");
            let (line, _) = VcardLine::take(bytes.as_bytes()).ok()?;
            line.bare_name()
                .eq_ignore_ascii_case("UID")
                .then(|| line.raw_value_str().trim().to_string())
                .filter(|uid| !uid.is_empty())
        })
    }
}

/// The stashed logical vCard lines behind the cardamum clientData entry.
///
/// A stash stored with its folds has them joined back, as unfolding does.
fn stash_lines(person: &GpeoplePerson) -> Vec<String> {
    let stashes = person
        .client_data
        .iter()
        .filter(|entry| entry.key.as_deref() == Some(GPEOPLE_PERSON_STASH_KEY))
        .filter_map(|entry| entry.value.as_deref());

    let mut lines: Vec<String> = Vec::new();

    for stash in stashes {
        // NOTE: a blank row ends the line before it, as it does in a card.
        let mut open = false;

        for row in stash.split('\n') {
            match lines.last_mut() {
                Some(line) if open && row.starts_with([' ', '\t']) => {
                    if line.ends_with('\r') {
                        line.pop();
                    }
                    line.push_str(&row[1..]);
                }
                _ => {
                    let row = row.trim_start_matches([' ', '\t']);
                    open = !row.trim_end_matches('\r').is_empty();
                    if open {
                        lines.push(row.to_string());
                    }
                }
            }
        }
    }

    lines
}

/// The structured-value components joined into one People field.
fn joined(components: &[Cow<'_, str>]) -> Option<String> {
    let joined = components
        .iter()
        .map(|component| component.as_ref().trim())
        .filter(|component| !component.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    if joined.is_empty() {
        None
    } else {
        Some(joined)
    }
}

/// Collects every TYPE parameter value, lowercased.
fn type_values(line: &VcardLine) -> Vec<String> {
    let mut types = Vec::new();
    for param in &line.params {
        if param.name.get().eq_ignore_ascii_case("TYPE") {
            types.extend(
                param
                    .values
                    .iter()
                    .map(|value| value.get().to_ascii_lowercase()),
            );
        }
    }
    types
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A person filling every field the projection manages.
    fn full_person() -> GpeoplePerson {
        GpeoplePerson {
            names: vec![GpeopleName {
                unstructured_name: Some("Jane Doe".into()),
                family_name: Some("Doe".into()),
                given_name: Some("Jane".into()),
                middle_name: Some("Q.".into()),
                honorific_prefix: Some("Dr.".into()),
                honorific_suffix: Some("PhD".into()),
                ..Default::default()
            }],
            nicknames: vec![GpeopleNickname {
                value: Some("Janie".into()),
                ..Default::default()
            }],
            email_addresses: vec![GpeopleEmailAddress {
                value: Some("jane@doe.org".into()),
                email_type: Some("home".into()),
                ..Default::default()
            }],
            im_clients: vec![GpeopleImClient {
                username: Some("jane@doe.org".into()),
                protocol: Some("xmpp".into()),
                ..Default::default()
            }],
            phone_numbers: vec![
                GpeoplePhoneNumber {
                    value: Some("+331111".into()),
                    phone_type: Some("work".into()),
                    ..Default::default()
                },
                GpeoplePhoneNumber {
                    value: Some("+333333".into()),
                    phone_type: Some("mobile".into()),
                    ..Default::default()
                },
            ],
            addresses: vec![GpeopleAddress {
                street_address: Some("12 Main St".into()),
                city: Some("Paris".into()),
                region: Some("IDF".into()),
                postal_code: Some("75000".into()),
                country: Some("France".into()),
                address_type: Some("home".into()),
                ..Default::default()
            }],
            organizations: vec![GpeopleOrganization {
                name: Some("ACME".into()),
                department: Some("R&D".into()),
                title: Some("Boss".into()),
                ..Default::default()
            }],
            occupations: vec![GpeopleOccupation {
                value: Some("Engineer".into()),
                ..Default::default()
            }],
            urls: vec![GpeopleUrl {
                value: Some("https://doe.org".into()),
                ..Default::default()
            }],
            birthdays: vec![GpeopleBirthday {
                date: Some(GpeopleDate {
                    year: Some(1983),
                    month: Some(4),
                    day: Some(1),
                }),
                ..Default::default()
            }],
            biographies: vec![GpeopleBiography {
                value: Some("a note".into()),
                content_type: Some(GpeopleContentType::TextPlain),
                ..Default::default()
            }],
            relations: vec![
                GpeopleRelation {
                    person: Some("John Doe".into()),
                    relation_type: Some("spouse".into()),
                    ..Default::default()
                },
                GpeopleRelation {
                    person: Some("Jimmy".into()),
                    relation_type: Some("child".into()),
                    ..Default::default()
                },
            ],
            ..Default::default()
        }
    }

    #[test]
    fn to_vcard_projects_every_mapped_field() {
        let mut person = full_person();
        person.resource_name = "people/c123".into();

        let vcard = person.to_vcard();
        assert!(vcard.contains("VERSION:4.0\r\n"));
        assert!(vcard.contains("UID:c123\r\n"));
        assert!(vcard.contains("FN:Jane Doe\r\n"));
        assert!(vcard.contains("N:Doe;Jane;Q.;Dr.;PhD\r\n"));
        assert!(vcard.contains("NICKNAME:Janie\r\n"));
        assert!(vcard.contains("EMAIL;TYPE=home:jane@doe.org\r\n"));
        assert!(vcard.contains("IMPP:xmpp:jane@doe.org\r\n"));
        assert!(vcard.contains("TEL;TYPE=work:+331111\r\n"));
        assert!(vcard.contains("TEL;TYPE=cell:+333333\r\n"));
        assert!(vcard.contains("ADR;TYPE=home:;;12 Main St;Paris;IDF;75000;France\r\n"));
        assert!(vcard.contains("ORG:ACME;R&D\r\n"));
        assert!(vcard.contains("TITLE:Boss\r\n"));
        assert!(vcard.contains("ROLE:Engineer\r\n"));
        assert!(vcard.contains("URL:https://doe.org\r\n"));
        assert!(vcard.contains("BDAY:1983-04-01\r\n"));
        assert!(vcard.contains("NOTE:a note\r\n"));
        assert!(vcard.contains("RELATED;TYPE=spouse;VALUE=text:John Doe\r\n"));
        assert!(vcard.contains("RELATED;TYPE=child;VALUE=text:Jimmy\r\n"));
    }

    #[test]
    fn from_vcard_reads_managed_props() {
        let vcard = "BEGIN:VCARD\r\nVERSION:4.0\r\nUID:abc\r\nFN:Jane Doe\r\n\
            N:Doe;Jane;Q.;Dr.;PhD\r\nNICKNAME:Janie,JJ\r\n\
            EMAIL;TYPE=home:jane@doe.org\r\nIMPP:xmpp:jane@doe.org\r\n\
            TEL;TYPE=cell:+333333\r\nTEL;TYPE=home,voice:+332222\r\nTEL:+331111\r\n\
            ADR;TYPE=home:box;ext;12 Main St;Paris;IDF;75000;France\r\nORG:ACME;R&D\r\n\
            TITLE:Boss\r\nROLE:Engineer\r\nURL:https://doe.org\r\nBDAY:1983-04-01\r\n\
            NOTE:a note\r\nRELATED;TYPE=spouse;VALUE=text:John Doe\r\n\
            RELATED;TYPE=child;VALUE=text:Jimmy\r\nEND:VCARD\r\n";

        let person = GpeoplePerson::from_vcard(vcard).unwrap().person;
        assert_eq!(person.resource_name, "");

        let name = &person.names[0];
        assert_eq!(name.unstructured_name.as_deref(), Some("Jane Doe"));
        assert_eq!(name.family_name.as_deref(), Some("Doe"));
        assert_eq!(name.given_name.as_deref(), Some("Jane"));
        assert_eq!(name.middle_name.as_deref(), Some("Q."));
        assert_eq!(name.honorific_prefix.as_deref(), Some("Dr."));
        assert_eq!(name.honorific_suffix.as_deref(), Some("PhD"));

        assert_eq!(person.nicknames.len(), 2);
        assert_eq!(person.nicknames[0].value.as_deref(), Some("Janie"));

        let email = &person.email_addresses[0];
        assert_eq!(email.value.as_deref(), Some("jane@doe.org"));
        assert_eq!(email.email_type.as_deref(), Some("home"));

        let im = &person.im_clients[0];
        assert_eq!(im.username.as_deref(), Some("jane@doe.org"));
        assert_eq!(im.protocol.as_deref(), Some("xmpp"));

        assert_eq!(person.phone_numbers.len(), 3);
        assert_eq!(
            person.phone_numbers[0].phone_type.as_deref(),
            Some("mobile")
        );
        assert_eq!(person.phone_numbers[1].phone_type.as_deref(), Some("home"));
        assert_eq!(person.phone_numbers[2].phone_type, None);

        let address = &person.addresses[0];
        assert_eq!(address.po_box.as_deref(), Some("box"));
        assert_eq!(address.extended_address.as_deref(), Some("ext"));
        assert_eq!(address.street_address.as_deref(), Some("12 Main St"));
        assert_eq!(address.city.as_deref(), Some("Paris"));
        assert_eq!(address.region.as_deref(), Some("IDF"));
        assert_eq!(address.postal_code.as_deref(), Some("75000"));
        assert_eq!(address.country.as_deref(), Some("France"));
        assert_eq!(address.address_type.as_deref(), Some("home"));

        let org = &person.organizations[0];
        assert_eq!(org.name.as_deref(), Some("ACME"));
        assert_eq!(org.department.as_deref(), Some("R&D"));
        assert_eq!(org.title.as_deref(), Some("Boss"));

        assert_eq!(person.occupations[0].value.as_deref(), Some("Engineer"));
        assert_eq!(person.urls[0].value.as_deref(), Some("https://doe.org"));

        let date = person.birthdays[0].date.as_ref().unwrap();
        assert_eq!(
            (date.year, date.month, date.day),
            (Some(1983), Some(4), Some(1))
        );

        assert_eq!(person.biographies[0].value.as_deref(), Some("a note"));

        assert_eq!(person.relations.len(), 2);
        assert_eq!(person.relations[0].person.as_deref(), Some("John Doe"));
        assert_eq!(person.relations[0].relation_type.as_deref(), Some("spouse"));
    }

    #[test]
    fn changed_fields_shrinks_to_the_edit() {
        let base = "BEGIN:VCARD\r\nVERSION:4.0\r\nFN:Jane Doe\r\n\
            TEL;TYPE=cell:+333333\r\nEMAIL:jane@doe.org\r\n\
            NOTE:a note\r\nEND:VCARD\r\n";
        let edited = "BEGIN:VCARD\r\nVERSION:4.0\r\nFN:Jane Doe\r\n\
            TEL;TYPE=cell:+444444\r\nEMAIL:jane@doe.org\r\nEND:VCARD\r\n";

        let fields = GpeoplePerson::from_vcard(edited)
            .unwrap()
            .person
            .changed_fields(&GpeoplePerson::from_vcard(base).unwrap().person);
        assert_eq!(
            fields,
            vec![
                GpeoplePersonField::Biographies,
                GpeoplePersonField::PhoneNumbers
            ]
        );
    }

    #[test]
    fn changed_fields_of_identical_cards_is_empty() {
        let vcard = full_person().to_vcard();
        let person = GpeoplePerson::from_vcard(&vcard).unwrap().person;
        assert!(person.changed_fields(&person.clone()).is_empty());
    }

    #[test]
    fn round_trip() {
        let person = full_person();
        assert_eq!(
            GpeoplePerson::from_vcard(&person.to_vcard())
                .unwrap()
                .person,
            person
        );
    }

    #[test]
    fn stash_preserves_unprojected_props() {
        let vcard = "BEGIN:VCARD\r\nVERSION:4.0\r\nFN:X\r\nX-FOO;TYPE=bar:baz\r\n\
            GENDER:F\r\nBDAY:--0412\r\nEND:VCARD\r\n";

        let person = GpeoplePerson::from_vcard(vcard).unwrap().person;
        assert!(person.birthdays.is_empty());

        let stash = &person.client_data[0];
        assert_eq!(stash.key.as_deref(), Some(GPEOPLE_PERSON_STASH_KEY));
        assert_eq!(
            stash.value.as_deref(),
            Some("X-FOO;TYPE=bar:baz\nGENDER:F\nBDAY:--0412")
        );

        let restored = person.to_vcard();
        assert!(restored.contains("X-FOO;TYPE=bar:baz\r\n"));
        assert!(restored.contains("GENDER:F\r\n"));
        assert!(restored.contains("BDAY:--0412\r\n"));
        assert!(restored.ends_with("END:VCARD\r\n"));

        assert_eq!(GpeoplePerson::from_vcard(&restored).unwrap().person, person);
    }

    #[test]
    fn minted_props_project_and_consume() {
        use crate::v1::rest::people::{
            GpeopleContactGroupMembership, GpeopleExternalId, GpeopleMembership,
        };

        let mut person = full_person();
        person.memberships = vec![GpeopleMembership {
            contact_group_membership: Some(GpeopleContactGroupMembership {
                contact_group_resource_name: Some("contactGroups/myContacts".into()),
                ..Default::default()
            }),
            ..Default::default()
        }];
        person.external_ids = vec![GpeopleExternalId {
            value: Some("42".into()),
            id_type: Some("account".into()),
            ..Default::default()
        }];

        // NOTE: memberships are structural addressbook data, not a minted
        // vendor property.
        let vcard = person.to_vcard();
        assert!(!vcard.contains("X-GOOGLE-MEMBERSHIP"));
        assert!(vcard.contains("X-GOOGLE-EXTERNAL-ID;TYPE=account:42\r\n"));

        // NOTE: the minted lines are consumed on the way back, so they
        // neither project nor reach the stash, and a legacy
        // X-GOOGLE-MEMBERSHIP line is dropped the same way.
        let legacy = vcard.replace(
            "END:VCARD",
            "X-GOOGLE-MEMBERSHIP:contactGroups/myContacts\r\nEND:VCARD",
        );
        let back = GpeoplePerson::from_vcard(&legacy).unwrap().person;
        assert!(back.memberships.is_empty());
        assert!(back.external_ids.is_empty());
        assert!(back.client_data.is_empty());
    }

    #[test]
    fn changed_fields_tracks_the_stash() {
        let base = "BEGIN:VCARD\r\nVERSION:4.0\r\nFN:X\r\nEND:VCARD\r\n";
        let edited = "BEGIN:VCARD\r\nVERSION:4.0\r\nFN:X\r\nX-FOO:bar\r\nEND:VCARD\r\n";

        let fields = GpeoplePerson::from_vcard(edited)
            .unwrap()
            .person
            .changed_fields(&GpeoplePerson::from_vcard(base).unwrap().person);
        assert_eq!(fields, vec![GpeoplePersonField::ClientData]);
    }

    #[test]
    fn stash_skips_oversized_lines() {
        let photo = format!("PHOTO:data:image/jpeg;base64,{}", "A".repeat(10_000));
        let vcard =
            format!("BEGIN:VCARD\r\nVERSION:4.0\r\nFN:X\r\n{photo}\r\nX-FOO:bar\r\nEND:VCARD\r\n");

        let person = GpeoplePerson::from_vcard(&vcard).unwrap().person;
        assert_eq!(person.client_data[0].value.as_deref(), Some("X-FOO:bar"));
    }

    #[test]
    fn a_vcard_uid_rides_the_stash_and_wins_over_the_person_id() {
        let vcard = "BEGIN:VCARD\r\nVERSION:4.0\r\nUID:urn:uuid:4fbe8971\r\n\
            FN:Jane Doe\r\nEND:VCARD\r\n";

        let mut person = GpeoplePerson::from_vcard(vcard).unwrap().person;
        // NOTE: what People hands back after the create: its own resource
        // name, and the stash it was given.
        person.resource_name = "people/c123".into();

        assert_eq!(person.stashed_uid().as_deref(), Some("urn:uuid:4fbe8971"));
        let vcard = person.to_vcard();
        assert!(vcard.contains("UID:urn:uuid:4fbe8971\r\n"));
        assert!(!vcard.contains("UID:c123"));
    }

    #[test]
    fn a_person_google_created_mints_its_uid_from_the_person_id() {
        let person = GpeoplePerson {
            resource_name: "people/c123".into(),
            ..Default::default()
        };

        assert_eq!(person.stashed_uid(), None);
        assert!(person.to_vcard().contains("UID:c123\r\n"));
    }

    /// A person holding only the stash `value`, as People hands it back.
    fn stashed(value: String) -> GpeoplePerson {
        GpeoplePerson {
            client_data: vec![GpeopleClientData {
                key: Some(GPEOPLE_PERSON_STASH_KEY.into()),
                value: Some(value),
                ..Default::default()
            }],
            ..Default::default()
        }
    }

    #[test]
    fn stash_keeps_a_folded_property_whole() {
        let long = format!("X-CUSTOM-FIELD:{}", "abcdefghij".repeat(15));
        let vcard = format!(
            "BEGIN:VCARD\r\nVERSION:4.0\r\nFN:X\r\n{}\r\n {}\r\n {}\r\nX-SHORT:kept\r\n\
             END:VCARD\r\n",
            &long[..75],
            &long[75..149],
            &long[149..],
        );

        let person = GpeoplePerson::from_vcard(&vcard).unwrap().person;
        assert_eq!(
            person.client_data[0].value,
            Some(format!("{long}\nX-SHORT:kept"))
        );

        let restored = person.to_vcard().replace("\r\n ", "");
        assert!(restored.contains(&format!("\r\n{long}\r\nX-SHORT:kept\r\n")));
    }

    #[test]
    fn a_folded_uid_rides_the_stash_whole() {
        let uid = "040000008200E00074C5B7101A82E00800000000".repeat(3);
        let vcard = format!(
            "BEGIN:VCARD\r\nVERSION:4.0\r\nUID:{}\r\n {}\r\nFN:X\r\nEND:VCARD\r\n",
            &uid[..71],
            &uid[71..],
        );

        let person = GpeoplePerson::from_vcard(&vcard).unwrap().person;
        assert_eq!(person.stashed_uid(), Some(uid));
    }

    #[test]
    fn a_stash_stored_with_its_folds_reads_back_whole() {
        let long = format!("X-CUSTOM-FIELD:{}", "abcdefghij".repeat(15));
        let stored = format!(
            "{}\r\n {}\r\n\t{}\nGENDER:F",
            &long[..75],
            &long[75..149],
            &long[149..],
        );

        let person = stashed(stored);
        assert_eq!(stash_lines(&person), [long.as_str(), "GENDER:F"]);

        let restored = person.to_vcard().replace("\r\n ", "");
        assert!(restored.contains(&format!("\r\n{long}\r\nGENDER:F\r\n")));
    }

    #[test]
    fn a_stored_blank_line_ends_the_line_before_it() {
        let person = stashed("GENDER:F\n\r\n X-FOO:a\n\n\tX-BAR:b".into());
        assert_eq!(stash_lines(&person), ["GENDER:F", "X-FOO:a", "X-BAR:b"]);
    }

    /// A xorshift generator: random cases, the same on every run.
    struct Cases(u64);

    impl Cases {
        fn below(&mut self, bound: usize) -> usize {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            (self.0 % bound as u64) as usize
        }

        fn pick<T: Copy>(&mut self, items: &[T]) -> T {
            items[self.below(items.len())]
        }

        /// An unmanaged logical line: any name, parameters and value.
        fn line(&mut self) -> String {
            const NAMES: &[&str] = &[
                "UID",
                "GENDER",
                "CATEGORIES",
                "KEY",
                "PHOTO",
                "GEO",
                "x-lower",
                "item1.X-ABLABEL",
            ];
            const PARAMS: &[&str] = &[
                "",
                ";TYPE=a,b",
                ";VALUE=uri",
                ";LANGUAGE=fr;PREF=1",
                ";X-P=\"quoted; with: marks\"",
                ";X-P=é日",
            ];
            const NAME_CHARS: &[char] = &['A', 'z', '0', '9', '-'];
            const VALUE_CHARS: &[char] = &[
                'a', 'Z', '0', ' ', '\t', ';', ',', ':', '\\', '"', '=', '-', 'é', 'ß', '日', '本',
                '😀',
            ];

            let mut line = match self.below(2) {
                0 => self.pick(NAMES).to_string(),
                _ => {
                    let len = 1 + self.below(40);
                    let name = (0..len).map(|_| self.pick(NAME_CHARS));
                    format!("X-{}", name.collect::<String>())
                }
            };

            line.push_str(self.pick(PARAMS));
            line.push(':');

            let len = self.pick(&[0, 1, 8, 60, 75, 150, 400]);
            let len = self.below(len + 1);
            line.extend((0..len).map(|_| self.pick(VALUE_CHARS)));

            line
        }

        /// `line` as it travels: folded at any char, any number of times.
        fn fold(&mut self, line: &str, eol: &str) -> String {
            let every = self.pick(&[1, 3, 20, 75, 1000]);
            let mut folded = String::new();

            for (i, c) in line.chars().enumerate() {
                if i > 0 && self.below(every) == 0 {
                    folded.push_str(eol);
                    folded.push(self.pick(&[' ', '\t']));
                }
                folded.push(c);
            }

            folded
        }
    }

    #[test]
    fn stash_round_trips_any_property() {
        let mut cases = Cases(0x9E37_79B9_7F4A_7C15);

        for case in 0..2000 {
            let eol = cases.pick(&["\r\n", "\n"]);
            let lines: Vec<String> = (0..1 + cases.below(4)).map(|_| cases.line()).collect();
            let folded: Vec<String> = lines.iter().map(|line| cases.fold(line, eol)).collect();
            let vcard = format!(
                "BEGIN:VCARD{eol}VERSION:4.0{eol}FN:X{eol}{}{eol}END:VCARD{eol}",
                folded.join(eol),
            );

            let person = GpeoplePerson::from_vcard(&vcard).unwrap().person;
            assert_eq!(stash_lines(&person), lines, "case {case}: {vcard:?}");

            let restored = person.to_vcard();
            let unfolded = restored.replace("\r\n ", "");
            for line in &lines {
                let line = format!("\r\n{line}\r\n");
                assert!(unfolded.contains(&line), "case {case}: {restored:?}");
            }

            let back = GpeoplePerson::from_vcard(&restored).unwrap().person;
            assert_eq!(back, person, "case {case}: {restored:?}");

            // NOTE: the shape stored before the stash held logical lines.
            let stored: Vec<&str> = folded.iter().map(|line| line.trim_end()).collect();
            let legacy = stashed(stored.join("\n"));
            let trimmed: Vec<&str> = lines.iter().map(|line| line.trim_end()).collect();
            assert_eq!(stash_lines(&legacy), trimmed, "case {case}: {stored:?}");
        }
    }

    #[test]
    fn a_line_break_in_a_url_adds_no_vcard_line() {
        let mut person = full_person();
        person.urls[0].value = Some("https://doe.org/\r\nTEL:+339999\nX-INJECTED:1".into());

        let vcard = person.to_vcard();
        let back = GpeoplePerson::from_vcard(&vcard).unwrap().person;
        assert_eq!(back.phone_numbers, person.phone_numbers);
        assert_eq!(back.client_data, []);

        assert!(vcard.contains("URL:https://doe.org/%0D%0ATEL:+339999%0AX-INJECTED:1\r\n"));
    }

    #[test]
    fn a_line_break_in_an_im_client_adds_no_vcard_line() {
        let mut person = full_person();
        person.im_clients[0].username = Some("jane\r\nTEL:+339999".into());
        person.im_clients[0].protocol = Some("xmpp\nX-INJECTED:1".into());

        let vcard = person.to_vcard();
        let back = GpeoplePerson::from_vcard(&vcard).unwrap().person;
        assert_eq!(back.phone_numbers, person.phone_numbers);
        assert_eq!(back.client_data, []);

        assert!(vcard.contains("IMPP:xmpp%0AX-INJECTED:1:jane%0D%0ATEL:+339999\r\n"));
    }

    #[test]
    fn a_nameless_person_stashes_no_empty_fn() {
        let person = GpeoplePerson::default();

        let once = GpeoplePerson::from_vcard(&person.to_vcard())
            .unwrap()
            .person;
        assert_eq!(once.client_data, []);

        let stored = stashed("FN:\nFN:\nX-FOO:bar".into());
        let vcard = stored.to_vcard();
        assert_eq!(vcard.matches("FN:").count(), 1, "{vcard}");
        assert!(vcard.contains("X-FOO:bar\r\n"));
    }

    /// A person as People holds it, with what a vCard cannot show.
    fn server_person() -> GpeoplePerson {
        use crate::v1::rest::people::{GpeopleFieldMetadata, GpeopleNicknameType};

        let primary = Some(GpeopleFieldMetadata {
            primary: Some(true),
            ..Default::default()
        });

        GpeoplePerson {
            resource_name: "people/c1".into(),
            names: vec![GpeopleName {
                metadata: primary.clone(),
                display_name: Some("Jane Doe".into()),
                unstructured_name: Some("Jane Doe".into()),
                family_name: Some("Doe".into()),
                given_name: Some("Jane".into()),
                phonetic_given_name: Some("Djayn".into()),
                ..Default::default()
            }],
            nicknames: vec![GpeopleNickname {
                value: Some("Janie".into()),
                nickname_type: Some(GpeopleNicknameType::ShortName),
                ..Default::default()
            }],
            email_addresses: vec![
                GpeopleEmailAddress {
                    metadata: primary.clone(),
                    value: Some("jane@doe.org".into()),
                    email_type: Some("home".into()),
                    formatted_type: Some("Home".into()),
                    ..Default::default()
                },
                GpeopleEmailAddress {
                    value: Some("jane@school.edu".into()),
                    email_type: Some("School".into()),
                    display_name: Some("Jane at school".into()),
                    ..Default::default()
                },
            ],
            im_clients: vec![GpeopleImClient {
                username: Some("jane".into()),
                protocol: Some("xmpp".into()),
                im_type: Some("work".into()),
                ..Default::default()
            }],
            phone_numbers: vec![
                GpeoplePhoneNumber {
                    value: Some("+331111".into()),
                    canonical_form: Some("+331111".into()),
                    phone_type: Some("mobile".into()),
                    ..Default::default()
                },
                GpeoplePhoneNumber {
                    value: Some("+332222".into()),
                    phone_type: Some("workFax".into()),
                    ..Default::default()
                },
            ],
            addresses: vec![GpeopleAddress {
                street_address: Some("12 Main St".into()),
                city: Some("Paris".into()),
                country_code: Some("FR".into()),
                address_type: Some("other".into()),
                ..Default::default()
            }],
            organizations: vec![
                GpeopleOrganization {
                    name: Some("ACME".into()),
                    title: Some("Boss".into()),
                    job_description: Some("Runs it".into()),
                    start_date: Some(GpeopleDate {
                        year: Some(2020),
                        month: Some(1),
                        day: Some(1),
                    }),
                    ..Default::default()
                },
                GpeopleOrganization {
                    name: Some("Old Corp".into()),
                    title: Some("Intern".into()),
                    ..Default::default()
                },
            ],
            occupations: vec![
                GpeopleOccupation {
                    value: Some("Engineer".into()),
                    ..Default::default()
                },
                GpeopleOccupation {
                    value: Some("Pilot".into()),
                    ..Default::default()
                },
            ],
            urls: vec![GpeopleUrl {
                value: Some("https://doe.org".into()),
                url_type: Some("blog".into()),
                ..Default::default()
            }],
            birthdays: vec![GpeopleBirthday {
                date: Some(GpeopleDate {
                    year: Some(1983),
                    month: Some(4),
                    day: Some(1),
                }),
                text: Some("April fools".into()),
                ..Default::default()
            }],
            biographies: vec![GpeopleBiography {
                metadata: primary,
                value: Some("a note".into()),
                content_type: Some(GpeopleContentType::TextPlain),
            }],
            relations: vec![
                GpeopleRelation {
                    person: Some("John Doe".into()),
                    relation_type: Some("spouse".into()),
                    formatted_type: Some("Spouse".into()),
                    ..Default::default()
                },
                GpeopleRelation {
                    person: Some("Ann".into()),
                    relation_type: Some("mother".into()),
                    ..Default::default()
                },
                GpeopleRelation {
                    person: Some("Bob".into()),
                    relation_type: Some("assistant".into()),
                    ..Default::default()
                },
            ],
            client_data: vec![
                GpeopleClientData {
                    key: Some("other.app".into()),
                    value: Some("kept".into()),
                    ..Default::default()
                },
                GpeopleClientData {
                    key: Some(GPEOPLE_PERSON_STASH_KEY.into()),
                    value: Some("UID:urn:uuid:4fbe8971\nX-FOO:bar".into()),
                    ..Default::default()
                },
            ],
            ..Default::default()
        }
    }

    /// The update sent for `current` once `from` became `to` in its vCard.
    fn update(
        current: &GpeoplePerson,
        from: &str,
        to: &str,
    ) -> (GpeoplePerson, Vec<GpeoplePersonField>) {
        let vcard = current.to_vcard();
        assert!(vcard.contains(from), "{vcard}");

        let edited = vcard.replace(from, to);
        let mut person = GpeoplePerson::from_vcard(&edited).unwrap().person;
        let base = GpeoplePerson::from_vcard(&vcard).unwrap().person;
        let fields = person.changed_fields(&base);
        person.merge_unprojected(current, &fields);

        (person, fields)
    }

    #[test]
    fn an_update_keeps_the_entries_the_vcard_cannot_show() {
        let current = server_person();

        let (person, fields) = update(&current, ":John Doe\r\n", ":Johnny Doe\r\n");
        assert_eq!(fields, [GpeoplePersonField::Relations]);
        assert_eq!(
            person.relations,
            [
                GpeopleRelation {
                    person: Some("Johnny Doe".into()),
                    ..current.relations[0].clone()
                },
                current.relations[1].clone(),
                current.relations[2].clone(),
            ]
        );

        let (person, fields) = update(&current, "ROLE:Engineer\r\n", "ROLE:Writer\r\n");
        assert_eq!(fields, [GpeoplePersonField::Occupations]);
        assert_eq!(person.occupations[0].value.as_deref(), Some("Writer"));
        assert_eq!(person.occupations[1..], current.occupations[1..]);
    }

    #[test]
    fn an_update_keeps_the_sub_fields_the_vcard_cannot_show() {
        let current = server_person();

        let (person, fields) = update(&current, "TITLE:Boss\r\n", "TITLE:Chief\r\n");
        assert_eq!(fields, [GpeoplePersonField::Organizations]);
        assert_eq!(
            person.organizations,
            [
                GpeopleOrganization {
                    title: Some("Chief".into()),
                    ..current.organizations[0].clone()
                },
                current.organizations[1].clone(),
            ]
        );

        let (person, fields) = update(&current, "FN:Jane Doe\r\n", "FN:Janet Doe\r\n");
        assert_eq!(fields, [GpeoplePersonField::Names]);
        assert_eq!(
            person.names,
            [GpeopleName {
                unstructured_name: Some("Janet Doe".into()),
                ..current.names[0].clone()
            }]
        );

        let (person, fields) = update(&current, "BDAY:1983-04-01", "BDAY:1984-04-01");
        assert_eq!(fields, [GpeoplePersonField::Birthdays]);
        assert_eq!(person.birthdays[0].text, current.birthdays[0].text);
        assert_eq!(person.birthdays[0].date.as_ref().unwrap().year, Some(1984));
    }

    #[test]
    fn an_update_keeps_a_type_the_vcard_cannot_show() {
        let current = server_person();

        let (person, fields) = update(&current, "END:VCARD", "EMAIL:new@doe.org\r\nEND:VCARD");
        assert_eq!(fields, [GpeoplePersonField::EmailAddresses]);
        assert_eq!(person.email_addresses[..2], current.email_addresses[..]);
        assert_eq!(
            person.email_addresses[2],
            GpeopleEmailAddress {
                value: Some("new@doe.org".into()),
                ..Default::default()
            }
        );

        let (person, fields) = update(&current, ":+331111\r\n", ":+339999\r\n");
        assert_eq!(fields, [GpeoplePersonField::PhoneNumbers]);
        assert_eq!(
            person.phone_numbers,
            [
                GpeoplePhoneNumber {
                    value: Some("+339999".into()),
                    ..current.phone_numbers[0].clone()
                },
                current.phone_numbers[1].clone(),
            ]
        );
    }

    #[test]
    fn an_update_takes_the_type_the_vcard_changed() {
        let current = server_person();

        let (person, _) = update(&current, "EMAIL;TYPE=home:", "EMAIL;TYPE=work:");
        assert_eq!(
            person.email_addresses[0].email_type.as_deref(),
            Some("work")
        );
        assert_eq!(person.email_addresses[1], current.email_addresses[1]);

        let (person, _) = update(&current, "EMAIL:jane@school", "EMAIL;TYPE=home:jane@school");
        assert_eq!(
            person.email_addresses,
            [
                current.email_addresses[0].clone(),
                GpeopleEmailAddress {
                    email_type: Some("home".into()),
                    ..current.email_addresses[1].clone()
                },
            ]
        );

        let (person, _) = update(&current, "TEL;TYPE=cell:", "TEL:");
        assert_eq!(person.phone_numbers[0].phone_type, None);
        assert_eq!(person.phone_numbers[1], current.phone_numbers[1]);
    }

    #[test]
    fn an_update_removes_what_the_vcard_removed() {
        let current = server_person();

        let (person, fields) = update(&current, "EMAIL:jane@school.edu\r\n", "");
        assert_eq!(fields, [GpeoplePersonField::EmailAddresses]);
        assert_eq!(person.email_addresses, current.email_addresses[..1]);

        let spouse = "RELATED;TYPE=spouse;VALUE=text:John Doe\r\n";
        let (person, fields) = update(&current, spouse, "");
        assert_eq!(fields, [GpeoplePersonField::Relations]);
        assert_eq!(person.relations, current.relations[1..]);

        let (person, fields) = update(&current, "BDAY:1983-04-01\r\n", "");
        assert_eq!(fields, [GpeoplePersonField::Birthdays]);
        assert_eq!(person.birthdays, []);
    }

    #[test]
    fn the_one_entry_edited_keeps_its_sub_fields() {
        let current = server_person();

        let (person, fields) = update(&current, "URL:https://doe.org", "URL:https://doe.example");
        assert_eq!(fields, [GpeoplePersonField::Urls]);
        assert_eq!(
            person.urls,
            [GpeopleUrl {
                value: Some("https://doe.example".into()),
                ..current.urls[0].clone()
            }]
        );

        let (person, fields) = update(&current, "12 Main St", "13 Main St");
        assert_eq!(fields, [GpeoplePersonField::Addresses]);
        assert_eq!(
            person.addresses,
            [GpeopleAddress {
                street_address: Some("13 Main St".into()),
                ..current.addresses[0].clone()
            }]
        );

        let (person, fields) = update(&current, "IMPP:xmpp:jane", "IMPP:sip:jane");
        assert_eq!(fields, [GpeoplePersonField::ImClients]);
        assert_eq!(
            person.im_clients,
            [GpeopleImClient {
                protocol: Some("sip".into()),
                ..current.im_clients[0].clone()
            }]
        );
    }

    #[test]
    fn two_entries_edited_at_once_are_new_entries() {
        let mut current = server_person();
        current.urls.push(GpeopleUrl {
            value: Some("https://jane.example".into()),
            url_type: Some("profile".into()),
            ..Default::default()
        });

        let (person, _) = update(&current, "URL:https://", "URL:http://");
        let values: Vec<_> = person.urls.iter().map(|url| url.value.as_deref()).collect();
        assert_eq!(
            values,
            [Some("http://doe.org"), Some("http://jane.example")]
        );
        assert_eq!(person.urls[0].url_type, None);
        assert_eq!(person.urls[1].url_type, None);
    }

    #[test]
    fn an_update_keeps_the_client_data_of_others() {
        let current = server_person();

        let (person, fields) = update(&current, "X-FOO:bar", "X-FOO:baz");
        assert_eq!(fields, [GpeoplePersonField::ClientData]);
        assert_eq!(
            person.client_data,
            [
                GpeopleClientData {
                    key: Some(GPEOPLE_PERSON_STASH_KEY.into()),
                    value: Some("UID:urn:uuid:4fbe8971\nX-FOO:baz".into()),
                    ..Default::default()
                },
                current.client_data[0].clone(),
            ]
        );
    }

    /// Every field the projection manages, the widest update mask.
    const MANAGED: &[GpeoplePersonField] = &[
        GpeoplePersonField::Addresses,
        GpeoplePersonField::Biographies,
        GpeoplePersonField::Birthdays,
        GpeoplePersonField::ClientData,
        GpeoplePersonField::EmailAddresses,
        GpeoplePersonField::ImClients,
        GpeoplePersonField::Names,
        GpeoplePersonField::Nicknames,
        GpeoplePersonField::Occupations,
        GpeoplePersonField::Organizations,
        GpeoplePersonField::PhoneNumbers,
        GpeoplePersonField::Relations,
        GpeoplePersonField::Urls,
    ];

    #[test]
    fn an_unedited_vcard_erases_nothing_under_the_widest_mask() {
        let mut current = server_person();
        current.birthdays[0].date.as_mut().unwrap().year = None;
        current.biographies.insert(
            0,
            GpeopleBiography {
                value: Some("<b>a profile</b>".into()),
                content_type: Some(GpeopleContentType::TextHtml),
                ..Default::default()
            },
        );
        current.relations.rotate_left(1);

        let mut person = GpeoplePerson::from_vcard(&current.to_vcard())
            .unwrap()
            .person;
        assert_eq!(person.changed_fields(&person.clone()), []);
        person.merge_unprojected(&current, MANAGED);

        macro_rules! assert_kept {
            ($($list:ident),+) => {$(
                assert_eq!(person.$list.len(), current.$list.len(), stringify!($list));
                for entry in &current.$list {
                    assert!(person.$list.contains(entry), "{entry:?}");
                }
            )+};
        }

        assert_kept!(
            addresses,
            birthdays,
            client_data,
            email_addresses,
            im_clients,
            names,
            nicknames,
            occupations,
            organizations,
            phone_numbers,
            relations,
            urls
        );

        // NOTE: an update cannot touch the HTML one, which a profile owns.
        assert_eq!(person.biographies, current.biographies[1..]);

        let once = person.clone();
        person.merge_unprojected(&current, MANAGED);
        assert_eq!(person, once);
    }

    #[test]
    fn a_field_outside_the_mask_is_left_alone() {
        let current = server_person();
        let vcard = current.to_vcard();

        let mut person = GpeoplePerson::from_vcard(&vcard).unwrap().person;
        let projected = person.clone();
        person.merge_unprojected(&current, &[GpeoplePersonField::Urls]);

        assert_eq!(person.urls, current.urls);
        assert_eq!(person.relations, projected.relations);
        assert_eq!(person.email_addresses, projected.email_addresses);
    }

    #[test]
    fn from_vcard_names_the_properties_it_drops() {
        let blob = "A".repeat(10_000);
        let vcard = format!(
            "BEGIN:VCARD\r\nVERSION:4.0\r\nFN:X\r\nPHOTO;MEDIATYPE=image/jpeg:data:{blob}\r\n\
             item1.key:data:{blob}\r\nX-FOO:bar\r\nEND:VCARD\r\n"
        );

        let projected = GpeoplePerson::from_vcard(&vcard).unwrap();
        assert_eq!(projected.dropped, ["PHOTO", "ITEM1.KEY"]);
        assert_eq!(stash_lines(&projected.person), ["X-FOO:bar"]);

        let small = "BEGIN:VCARD\r\nVERSION:4.0\r\nFN:X\r\nX-FOO:bar\r\nEND:VCARD\r\n";
        assert_eq!(GpeoplePerson::from_vcard(small).unwrap().dropped, [""; 0]);
    }
}
