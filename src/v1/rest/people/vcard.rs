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
//!   A read that projects passes [`GPEOPLE_PERSON_VCARD_FIELDS`].
//!
//! Unlike Graph's fixed slots, People fields are true lists, so every
//! vCard property projects without truncation.

use core::{fmt, str::FromStr};

use alloc::{
    borrow::Cow,
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
                    value: VcardValue::Uri(VcardUri(Cow::Owned(uri))),
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
                    value: VcardValue::Uri(VcardUri(Cow::Owned(page.to_string()))),
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

        let stash = stash_lines(person);
        for line in &stash {
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
    /// person's identity across a sync with another source.
    pub fn from_vcard(vcard: &str) -> Result<Self, GpeoplePersonVcardError> {
        let card = VcardCst::parse(vcard).map_err(GpeoplePersonVcardError::Parse)?;
        let version = card.version();

        let mut person = GpeoplePerson::default();
        let mut name = GpeopleName::default();
        let mut org = GpeopleOrganization::default();
        let mut notes = Vec::new();
        let mut stash = Vec::new();
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
                    let value = FN::decode(line, version);
                    set_first(&mut name.unstructured_name, &value.0)
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
                let raw = line.to_string();
                let raw = raw.trim_end().to_string();
                if raw.len() <= MAX_STASH_LINE {
                    stash.push(raw);
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

        Ok(person)
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
}

/// The property name of a stashed vCard line, up to its first `;` or `:`.
fn property_name(line: &str) -> String {
    let end = line.find([';', ':']).unwrap_or(line.len());
    line[..end].to_uppercase()
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
        value: VcardValue::Adr(value),
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

/// The stashed vCard lines behind the cardamum clientData entry.
fn stash_lines(person: &GpeoplePerson) -> Vec<String> {
    person
        .client_data
        .iter()
        .filter(|entry| entry.key.as_deref() == Some(GPEOPLE_PERSON_STASH_KEY))
        .filter_map(|entry| entry.value.as_deref())
        .flat_map(|value| value.split('\n'))
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect()
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

        let person = GpeoplePerson::from_vcard(vcard).unwrap();
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
            .changed_fields(&GpeoplePerson::from_vcard(base).unwrap());
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
        let person = GpeoplePerson::from_vcard(&vcard).unwrap();
        assert!(person.changed_fields(&person.clone()).is_empty());
    }

    #[test]
    fn round_trip() {
        let person = full_person();
        assert_eq!(
            GpeoplePerson::from_vcard(&person.to_vcard()).unwrap(),
            person
        );
    }

    #[test]
    fn stash_preserves_unprojected_props() {
        let vcard = "BEGIN:VCARD\r\nVERSION:4.0\r\nFN:X\r\nX-FOO;TYPE=bar:baz\r\n\
            GENDER:F\r\nBDAY:--0412\r\nEND:VCARD\r\n";

        let person = GpeoplePerson::from_vcard(vcard).unwrap();
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

        assert_eq!(GpeoplePerson::from_vcard(&restored).unwrap(), person);
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
        let back = GpeoplePerson::from_vcard(&legacy).unwrap();
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
            .changed_fields(&GpeoplePerson::from_vcard(base).unwrap());
        assert_eq!(fields, vec![GpeoplePersonField::ClientData]);
    }

    #[test]
    fn stash_skips_oversized_lines() {
        let photo = format!("PHOTO:data:image/jpeg;base64,{}", "A".repeat(10_000));
        let vcard =
            format!("BEGIN:VCARD\r\nVERSION:4.0\r\nFN:X\r\n{photo}\r\nX-FOO:bar\r\nEND:VCARD\r\n");

        let person = GpeoplePerson::from_vcard(&vcard).unwrap();
        assert_eq!(person.client_data[0].value.as_deref(), Some("X-FOO:bar"));
    }

    #[test]
    fn a_vcard_uid_rides_the_stash_and_wins_over_the_person_id() {
        let vcard = "BEGIN:VCARD\r\nVERSION:4.0\r\nUID:urn:uuid:4fbe8971\r\n\
            FN:Jane Doe\r\nEND:VCARD\r\n";

        let mut person = GpeoplePerson::from_vcard(vcard).unwrap();
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
}
