//! People persons (`people`): get, getBatchGet, createContact,
//! updateContact, deleteContact, updateContactPhoto,
//! deleteContactPhoto, batchCreateContacts, batchUpdateContacts,
//! batchDeleteContacts, searchContacts, listDirectoryPeople,
//! searchDirectoryPeople.
//!
//! <https://developers.google.com/people/api/rest/v1/people>
//!
//! Fields the reference marks as deprecated with "no data will be
//! returned" (ageRange, braggingRights, relationshipInterests,
//! relationshipStatuses, residences, taglines) are omitted.

use alloc::{string::String, vec::Vec};

use serde::{Deserialize, Serialize};

pub mod batch_create_contacts;
pub mod batch_delete_contacts;
pub mod batch_update_contacts;
pub mod connections;
pub mod create_contact;
pub mod delete_contact;
pub mod delete_contact_photo;
pub mod get;
pub mod get_batch_get;
pub mod list_directory_people;
pub mod search_contacts;
pub mod search_directory_people;
pub mod update_contact;
pub mod update_contact_photo;
#[cfg(feature = "vcard")]
pub mod vcard;

/// Information about a person merged from various data sources such as
/// the authenticated user's contacts and profile data.
#[derive(Debug, Clone, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GpeoplePerson {
    /// The resource name of the person, assigned by the server.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub resource_name: String,
    /// The HTTP entity tag of the resource, used for caching and
    /// optimistic concurrency control.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub etag: String,
    /// Metadata about the person.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<GpeoplePersonMetadata>,
    /// The person's street addresses.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub addresses: Vec<GpeopleAddress>,
    /// The person's age ranges.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub age_ranges: Vec<GpeopleAgeRangeType>,
    /// The person's biographies.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub biographies: Vec<GpeopleBiography>,
    /// The person's birthdays.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub birthdays: Vec<GpeopleBirthday>,
    /// The person's calendar URLs.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub calendar_urls: Vec<GpeopleCalendarUrl>,
    /// The person's client data entries.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub client_data: Vec<GpeopleClientData>,
    /// The person's cover photos.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cover_photos: Vec<GpeopleCoverPhoto>,
    /// The person's email addresses.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub email_addresses: Vec<GpeopleEmailAddress>,
    /// The person's events, such as anniversaries.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub events: Vec<GpeopleEvent>,
    /// The person's external identifiers.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub external_ids: Vec<GpeopleExternalId>,
    /// The person's file-as names, used for sorting.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub file_ases: Vec<GpeopleFileAs>,
    /// The person's genders.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub genders: Vec<GpeopleGender>,
    /// The person's instant messaging clients.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub im_clients: Vec<GpeopleImClient>,
    /// The person's interests.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub interests: Vec<GpeopleInterest>,
    /// The person's locale preferences.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub locales: Vec<GpeopleLocale>,
    /// The person's locations.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub locations: Vec<GpeopleLocation>,
    /// The person's group memberships.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub memberships: Vec<GpeopleMembership>,
    /// The person's miscellaneous keywords.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub misc_keywords: Vec<GpeopleMiscKeyword>,
    /// The person's names.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub names: Vec<GpeopleName>,
    /// The person's nicknames.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub nicknames: Vec<GpeopleNickname>,
    /// The person's occupations.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub occupations: Vec<GpeopleOccupation>,
    /// The person's past or current organizations.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub organizations: Vec<GpeopleOrganization>,
    /// The person's phone numbers.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub phone_numbers: Vec<GpeoplePhoneNumber>,
    /// The person's photos.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub photos: Vec<GpeoplePhoto>,
    /// The person's relations to other people.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub relations: Vec<GpeopleRelation>,
    /// The person's SIP addresses, used for VoIP calls.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sip_addresses: Vec<GpeopleSipAddress>,
    /// The person's skills.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub skills: Vec<GpeopleSkill>,
    /// The person's associated URLs.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub urls: Vec<GpeopleUrl>,
    /// The person's user-defined data entries.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub user_defined: Vec<GpeopleUserDefined>,
}

impl GpeoplePerson {
    /// The bare person id behind the `people/<id>` resource name.
    pub fn id(&self) -> &str {
        self.resource_name
            .strip_prefix("people/")
            .unwrap_or(&self.resource_name)
    }
}

/// Metadata about a person.
#[derive(Debug, Clone, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GpeoplePersonMetadata {
    /// The sources of data for the person.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sources: Vec<GpeopleSource>,
    /// Any former resource names this person had.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub previous_resource_names: Vec<String>,
    /// Resource names of people linked to this person's data.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub linked_people_resource_names: Vec<String>,
    /// Whether this person is deleted; only set on sync requests.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deleted: Option<bool>,
    /// The type of the person object.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub object_type: Option<GpeopleObjectType>,
}

/// Source of a person field.
#[derive(Debug, Clone, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GpeopleSource {
    /// The source type (wire field: `type`).
    #[serde(default, rename = "type", skip_serializing_if = "Option::is_none")]
    pub source_type: Option<GpeopleSourceType>,
    /// The unique identifier of the source within its type.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub id: String,
    /// The HTTP entity tag of the source.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub etag: String,
    /// The last-update timestamp of the source, in RFC 3339 format.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub update_time: Option<String>,
    /// Metadata about the profile source; only set when the source
    /// type is `Profile`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile_metadata: Option<GpeopleProfileMetadata>,
}

/// Type of a person field source.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GpeopleSourceType {
    /// Unspecified source type.
    SourceTypeUnspecified,
    /// Google Account.
    Account,
    /// Google profile.
    Profile,
    /// Google Workspace domain profile.
    DomainProfile,
    /// Google contact.
    Contact,
    /// Other contact (not in the authenticated user's contacts).
    OtherContact,
    /// Google Workspace domain-shared contact.
    DomainContact,
}

/// Metadata about a profile.
#[derive(Debug, Clone, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GpeopleProfileMetadata {
    /// The type of the profile object.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub object_type: Option<GpeopleObjectType>,
    /// The user types of the profile.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub user_types: Vec<String>,
}

/// Type of a person object: person or (Currents) page.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GpeopleObjectType {
    /// Unspecified object type.
    ObjectTypeUnspecified,
    /// Person object.
    Person,
    /// Currents (Google+) page object.
    Page,
}

/// Metadata about a person field: its source and primary/verified flags.
#[derive(Debug, Clone, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GpeopleFieldMetadata {
    /// Whether the field is the primary field across all sources.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub primary: Option<bool>,
    /// Whether the field is the primary field for its source.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_primary: Option<bool>,
    /// Whether the field value was verified; only meaningful for
    /// email addresses and phone numbers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verified: Option<bool>,
    /// The source of the field.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<GpeopleSource>,
}

/// Whole or partial calendar date, such as a birthday.
#[derive(Debug, Clone, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GpeopleDate {
    /// The year of the date; 0 means the year is unspecified.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub year: Option<i32>,
    /// The month of the date (1–12); 0 means the month is unspecified.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub month: Option<i32>,
    /// The day of the date (1–31); 0 means the day is unspecified.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub day: Option<i32>,
}

/// Person's physical address.
#[derive(Debug, Clone, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GpeopleAddress {
    /// Metadata about the address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<GpeopleFieldMetadata>,
    /// The type of the address (wire field: `type`), e.g. `home` or
    /// `work`.
    #[serde(default, rename = "type", skip_serializing_if = "Option::is_none")]
    pub address_type: Option<String>,
    /// The read-only type formatted for display.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub formatted_type: Option<String>,
    /// The read-only full address formatted for display.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub formatted_value: Option<String>,
    /// The P.O. box of the address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub po_box: Option<String>,
    /// The street address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub street_address: Option<String>,
    /// The extended street address (e.g. apartment or suite number).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extended_address: Option<String>,
    /// The city of the address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,
    /// The region (state or province) of the address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    /// The postal code of the address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub postal_code: Option<String>,
    /// The country of the address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
    /// The ISO 3166-1 alpha-2 country code of the address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub country_code: Option<String>,
}

/// Person's age range.
#[derive(Debug, Clone, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GpeopleAgeRangeType {
    /// Metadata about the age range.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<GpeopleFieldMetadata>,
    /// The age range of the person.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub age_range: Option<GpeopleAgeRange>,
}

/// Age range bucket of a person.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GpeopleAgeRange {
    /// Unspecified age range.
    AgeRangeUnspecified,
    /// Younger than 18 years.
    LessThanEighteen,
    /// Between 18 and 20 years (inclusive).
    EighteenToTwenty,
    /// 21 years or older.
    TwentyOneOrOlder,
}

/// Person's short biography.
#[derive(Debug, Clone, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GpeopleBiography {
    /// Metadata about the biography.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<GpeopleFieldMetadata>,
    /// The short biography text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    /// The content type of the biography.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_type: Option<GpeopleContentType>,
}

/// Content type of a biography.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GpeopleContentType {
    /// Unspecified content type.
    ContentTypeUnspecified,
    /// Plain text content.
    TextPlain,
    /// HTML content.
    TextHtml,
}

/// Person's birthday, as a structured date and/or free-form text.
#[derive(Debug, Clone, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GpeopleBirthday {
    /// Metadata about the birthday.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<GpeopleFieldMetadata>,
    /// The structured date of the birthday.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date: Option<GpeopleDate>,
    /// The free-form birthday text, used when a structured date
    /// cannot represent it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

/// Person's calendar URL.
#[derive(Debug, Clone, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GpeopleCalendarUrl {
    /// Metadata about the calendar URL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<GpeopleFieldMetadata>,
    /// The calendar URL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// The type of the calendar URL (wire field: `type`), e.g.
    /// `HomeCal` or `FreeBusy`.
    #[serde(default, rename = "type", skip_serializing_if = "Option::is_none")]
    pub calendar_type: Option<String>,
    /// The read-only type formatted for display.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub formatted_type: Option<String>,
}

/// Arbitrary client data attached to a person.
#[derive(Debug, Clone, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GpeopleClientData {
    /// Metadata about the client data entry.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<GpeopleFieldMetadata>,
    /// The client-specified key of the data entry.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    /// The client-specified value of the data entry.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

/// Person's cover photo.
#[derive(Debug, Clone, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GpeopleCoverPhoto {
    /// Metadata about the cover photo.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<GpeopleFieldMetadata>,
    /// The URL of the cover photo.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Whether the cover photo is the default cover photo.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<bool>,
}

/// Person's email address.
#[derive(Debug, Clone, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GpeopleEmailAddress {
    /// Metadata about the email address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<GpeopleFieldMetadata>,
    /// The email address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    /// The type of the email address (wire field: `type`), e.g.
    /// `home` or `work`.
    #[serde(default, rename = "type", skip_serializing_if = "Option::is_none")]
    pub email_type: Option<String>,
    /// The read-only type formatted for display.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub formatted_type: Option<String>,
    /// The display name associated with the email address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
}

/// Event related to the person, such as an anniversary.
#[derive(Debug, Clone, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GpeopleEvent {
    /// Metadata about the event.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<GpeopleFieldMetadata>,
    /// The date of the event.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date: Option<GpeopleDate>,
    /// The type of the event (wire field: `type`), e.g.
    /// `anniversary`.
    #[serde(default, rename = "type", skip_serializing_if = "Option::is_none")]
    pub event_type: Option<String>,
    /// The read-only type formatted for display.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub formatted_type: Option<String>,
}

/// Identifier from an external entity related to the person.
#[derive(Debug, Clone, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GpeopleExternalId {
    /// Metadata about the external identifier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<GpeopleFieldMetadata>,
    /// The value of the external identifier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    /// The type of the external identifier (wire field: `type`),
    /// e.g. `account` or `customer`.
    #[serde(default, rename = "type", skip_serializing_if = "Option::is_none")]
    pub id_type: Option<String>,
    /// The read-only type formatted for display.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub formatted_type: Option<String>,
}

/// Name that should be used to sort the person in a list.
#[derive(Debug, Clone, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GpeopleFileAs {
    /// Metadata about the file-as name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<GpeopleFieldMetadata>,
    /// The name that should be used to sort the person in a list.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

/// Person's gender.
#[derive(Debug, Clone, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GpeopleGender {
    /// Metadata about the gender.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<GpeopleFieldMetadata>,
    /// The gender value, e.g. `male`, `female`, or `unspecified`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    /// The read-only value formatted for display.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub formatted_value: Option<String>,
    /// The preferred pronoun or form of address, e.g. `she` or
    /// `they`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address_me_as: Option<String>,
}

/// Person's instant messaging client.
#[derive(Debug, Clone, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GpeopleImClient {
    /// Metadata about the IM client.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<GpeopleFieldMetadata>,
    /// The username used in the IM client.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    /// The type of the IM client (wire field: `type`), e.g. `home`
    /// or `work`.
    #[serde(default, rename = "type", skip_serializing_if = "Option::is_none")]
    pub im_type: Option<String>,
    /// The read-only type formatted for display.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub formatted_type: Option<String>,
    /// The protocol of the IM client, e.g. `aim` or `msn`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub protocol: Option<String>,
    /// The read-only protocol formatted for display.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub formatted_protocol: Option<String>,
}

/// One of the person's interests.
#[derive(Debug, Clone, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GpeopleInterest {
    /// Metadata about the interest.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<GpeopleFieldMetadata>,
    /// The interest value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

/// Person's locale preference, as an IETF BCP 47 language tag.
#[derive(Debug, Clone, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GpeopleLocale {
    /// Metadata about the locale preference.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<GpeopleFieldMetadata>,
    /// The IETF BCP 47 language tag representing the locale
    /// preference.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

/// Person's location.
#[derive(Debug, Clone, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GpeopleLocation {
    /// Metadata about the location.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<GpeopleFieldMetadata>,
    /// The free-form location value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    /// The type of the location (wire field: `type`), e.g. `desk`
    /// or `grewUp`.
    #[serde(default, rename = "type", skip_serializing_if = "Option::is_none")]
    pub location_type: Option<String>,
    /// Whether the location is the current location.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current: Option<bool>,
    /// The building identifier within the location.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub building_id: Option<String>,
    /// The floor name or number within the building.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub floor: Option<String>,
    /// The floor section within the floor.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub floor_section: Option<String>,
    /// The desk code within the floor section.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub desk_code: Option<String>,
}

/// Person's membership in a group.
#[derive(Debug, Clone, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GpeopleMembership {
    /// Metadata about the membership.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<GpeopleFieldMetadata>,
    /// The contact group membership.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contact_group_membership: Option<GpeopleContactGroupMembership>,
    /// The Google Workspace domain membership.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub domain_membership: Option<GpeopleDomainMembership>,
}

/// Contact group membership of a person.
#[derive(Debug, Clone, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GpeopleContactGroupMembership {
    /// The read-only ID of the contact group.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contact_group_id: Option<String>,
    /// The resource name of the contact group.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contact_group_resource_name: Option<String>,
}

/// Google Workspace domain membership of a person.
#[derive(Debug, Clone, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GpeopleDomainMembership {
    /// Whether the person is in the viewer's Google Workspace domain.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub in_viewer_domain: Option<bool>,
}

/// Person's miscellaneous keyword.
#[derive(Debug, Clone, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GpeopleMiscKeyword {
    /// Metadata about the miscellaneous keyword.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<GpeopleFieldMetadata>,
    /// The value of the miscellaneous keyword.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    /// The type of the miscellaneous keyword (wire field: `type`).
    #[serde(default, rename = "type", skip_serializing_if = "Option::is_none")]
    pub keyword_type: Option<GpeopleMiscKeywordType>,
    /// The read-only type formatted for display.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub formatted_type: Option<String>,
}

/// Type of a miscellaneous keyword.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GpeopleMiscKeywordType {
    /// Unspecified keyword type.
    TypeUnspecified,
    /// Outlook billing information field.
    OutlookBillingInformation,
    /// Outlook directory server field.
    OutlookDirectoryServer,
    /// Outlook keyword field.
    OutlookKeyword,
    /// Outlook mileage field.
    OutlookMileage,
    /// Outlook priority field.
    OutlookPriority,
    /// Outlook sensitivity field.
    OutlookSensitivity,
    /// Outlook subject field.
    OutlookSubject,
    /// Outlook user field.
    OutlookUser,
    /// Home keyword.
    Home,
    /// Work keyword.
    Work,
    /// Other keyword.
    Other,
}

/// Person's name.
#[derive(Debug, Clone, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GpeopleName {
    /// Metadata about the name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<GpeopleFieldMetadata>,
    /// The free-form full name value provided by the user.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unstructured_name: Option<String>,
    /// The read-only display name formatted for the locale, assembled
    /// from the name components.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    /// The read-only display name formatted with the last name first,
    /// assembled from the name components.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name_last_first: Option<String>,
    /// The family (last) name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub family_name: Option<String>,
    /// The given (first) name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub given_name: Option<String>,
    /// The middle name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub middle_name: Option<String>,
    /// The honorific prefix, e.g. `Mrs.` or `Dr.`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub honorific_prefix: Option<String>,
    /// The honorific suffix, e.g. `Jr.` or `Sr.`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub honorific_suffix: Option<String>,
    /// The full name spelled as it sounds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phonetic_full_name: Option<String>,
    /// The family name spelled as it sounds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phonetic_family_name: Option<String>,
    /// The given name spelled as it sounds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phonetic_given_name: Option<String>,
    /// The middle name spelled as it sounds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phonetic_middle_name: Option<String>,
    /// The honorific prefix spelled as it sounds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phonetic_honorific_prefix: Option<String>,
    /// The honorific suffix spelled as it sounds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phonetic_honorific_suffix: Option<String>,
}

/// Person's nickname.
#[derive(Debug, Clone, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GpeopleNickname {
    /// Metadata about the nickname.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<GpeopleFieldMetadata>,
    /// The nickname value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    /// The type of the nickname (wire field: `type`).
    #[serde(default, rename = "type", skip_serializing_if = "Option::is_none")]
    pub nickname_type: Option<GpeopleNicknameType>,
}

/// Type of a nickname.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GpeopleNicknameType {
    /// Generic nickname.
    Default,
    /// Maiden name (name before marriage).
    MaidenName,
    /// Initials.
    Initials,
    /// Google+ profile nickname.
    Gplus,
    /// Other name.
    OtherName,
    /// Alternate name.
    AlternateName,
    /// Short name.
    ShortName,
}

/// Person's occupation.
#[derive(Debug, Clone, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GpeopleOccupation {
    /// Metadata about the occupation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<GpeopleFieldMetadata>,
    /// The occupation value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

/// Person's past or current organization.
#[derive(Debug, Clone, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GpeopleOrganization {
    /// Metadata about the organization.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<GpeopleFieldMetadata>,
    /// The type of the organization (wire field: `type`), e.g.
    /// `work` or `school`.
    #[serde(default, rename = "type", skip_serializing_if = "Option::is_none")]
    pub org_type: Option<String>,
    /// The read-only type formatted for display.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub formatted_type: Option<String>,
    /// The start date at which the person joined the organization.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_date: Option<GpeopleDate>,
    /// The end date at which the person left the organization.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_date: Option<GpeopleDate>,
    /// Whether the organization is the person's current organization.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current: Option<bool>,
    /// The name of the organization.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The phonetic name of the organization.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phonetic_name: Option<String>,
    /// The department within the organization.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub department: Option<String>,
    /// The person's job title within the organization.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// The person's job description within the organization.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub job_description: Option<String>,
    /// The ticker symbol of the organization.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
    /// The domain of the organization.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    /// The location of the organization.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    /// The cost center of the organization.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_center: Option<String>,
    /// The person's full-time-equivalent percentage at the
    /// organization in millipercent (e.g. 75000 = 75%).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub full_time_equivalent_millipercent: Option<i32>,
}

/// Person's phone number.
#[derive(Debug, Clone, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GpeoplePhoneNumber {
    /// Metadata about the phone number.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<GpeopleFieldMetadata>,
    /// The phone number.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    /// The read-only canonicalized ITU-T E.164 form of the phone
    /// number.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub canonical_form: Option<String>,
    /// The type of the phone number (wire field: `type`), e.g.
    /// `home` or `mobile`.
    #[serde(default, rename = "type", skip_serializing_if = "Option::is_none")]
    pub phone_type: Option<String>,
    /// The read-only type formatted for display.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub formatted_type: Option<String>,
}

/// Person's photo.
#[derive(Debug, Clone, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GpeoplePhoto {
    /// Metadata about the photo.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<GpeopleFieldMetadata>,
    /// The URL of the photo.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Whether the photo is the default photo.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<bool>,
}

/// Person's relation to another person.
#[derive(Debug, Clone, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GpeopleRelation {
    /// Metadata about the relation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<GpeopleFieldMetadata>,
    /// The name of the related person.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub person: Option<String>,
    /// The type of the relation (wire field: `type`), e.g. `spouse`
    /// or `parent`.
    #[serde(default, rename = "type", skip_serializing_if = "Option::is_none")]
    pub relation_type: Option<String>,
    /// The read-only type formatted for display.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub formatted_type: Option<String>,
}

/// Person's SIP address, for VoIP calls.
#[derive(Debug, Clone, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GpeopleSipAddress {
    /// Metadata about the SIP address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<GpeopleFieldMetadata>,
    /// The SIP address in the RFC 3261 URI form.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    /// The type of the SIP address (wire field: `type`), e.g. `home`
    /// or `work`.
    #[serde(default, rename = "type", skip_serializing_if = "Option::is_none")]
    pub sip_type: Option<String>,
    /// The read-only type formatted for display.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub formatted_type: Option<String>,
}

/// Skill that the person has.
#[derive(Debug, Clone, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GpeopleSkill {
    /// Metadata about the skill.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<GpeopleFieldMetadata>,
    /// The skill value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

/// Person's associated URL.
#[derive(Debug, Clone, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GpeopleUrl {
    /// Metadata about the URL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<GpeopleFieldMetadata>,
    /// The URL value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    /// The type of the URL (wire field: `type`), e.g. `home` or
    /// `work`.
    #[serde(default, rename = "type", skip_serializing_if = "Option::is_none")]
    pub url_type: Option<String>,
    /// The read-only type formatted for display.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub formatted_type: Option<String>,
}

/// Arbitrary user data attached to a person.
#[derive(Debug, Clone, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GpeopleUserDefined {
    /// Metadata about the user-defined data entry.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<GpeopleFieldMetadata>,
    /// The user-specified key of the data entry.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    /// The user-specified value of the data entry.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

/// Response for a single requested person in a batch method.
#[derive(Debug, Clone, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GpeoplePersonResponse {
    /// The person the request was for, if there are no errors.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub person: Option<GpeoplePerson>,
    /// The original requested resource name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requested_resource_name: Option<String>,
    /// The HTTP status code of the response for this person.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub http_status_code: Option<u16>,
    /// The status of the response for this person, populated only on
    /// error.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<GpeopleStatus>,
}

/// Logical error model of a failed item in a batch method.
#[derive(Debug, Clone, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GpeopleStatus {
    /// The Google API canonical error code.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<i32>,
    /// The developer-facing error message.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// A list of messages carrying error details.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub details: Vec<serde_json::Value>,
}

/// Response to a search request, shared by `people.searchContacts` and
/// `otherContacts.search`.
#[derive(Debug, Clone, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GpeopleSearchResponse {
    /// The search results.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub results: Vec<GpeopleSearchResult>,
}

/// Single result of a search query.
#[derive(Debug, Clone, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GpeopleSearchResult {
    /// The person matching the search query.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub person: Option<GpeoplePerson>,
}

/// Person field selectable in a `personFields`/`readMask` field mask.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum GpeoplePersonField {
    /// Street addresses.
    Addresses,
    /// Age ranges.
    AgeRanges,
    /// Biographies.
    Biographies,
    /// Birthdays.
    Birthdays,
    /// Calendar URLs.
    CalendarUrls,
    /// Client data entries.
    ClientData,
    /// Cover photos.
    CoverPhotos,
    /// Email addresses.
    EmailAddresses,
    /// Events such as anniversaries.
    Events,
    /// External identifiers.
    ExternalIds,
    /// Genders.
    Genders,
    /// Instant messaging clients.
    ImClients,
    /// Interests.
    Interests,
    /// Locale preferences.
    Locales,
    /// Locations.
    Locations,
    /// Group memberships.
    Memberships,
    /// Person metadata.
    Metadata,
    /// Miscellaneous keywords.
    MiscKeywords,
    /// Names.
    Names,
    /// Nicknames.
    Nicknames,
    /// Occupations.
    Occupations,
    /// Organizations.
    Organizations,
    /// Phone numbers.
    PhoneNumbers,
    /// Photos.
    Photos,
    /// Relations to other people.
    Relations,
    /// SIP addresses.
    SipAddresses,
    /// Skills.
    Skills,
    /// Associated URLs.
    Urls,
    /// User-defined data entries.
    UserDefined,
}

/// Source type selectable in a `sources` query parameter.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GpeopleReadSourceType {
    /// Returns the authenticated user's profile data from Google
    /// profile.
    ReadSourceTypeProfile,
    /// Returns the authenticated user's contacts.
    ReadSourceTypeContact,
    /// Returns Google Workspace domain-shared contacts.
    ReadSourceTypeDomainContact,
    /// Returns other contacts (not in the user's contact list).
    ReadSourceTypeOtherContact,
}

/// Order in which connections are sorted (`people.connections.list`).
#[derive(Debug, Clone, Copy, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GpeopleSortOrder {
    /// Sort by last modified time, oldest first.
    LastModifiedAscending,
    /// Sort by last modified time, newest first.
    LastModifiedDescending,
    /// Sort alphabetically by first name.
    FirstNameAscending,
    /// Sort alphabetically by last name.
    LastNameAscending,
}

/// Directory source selectable in the directory methods.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GpeopleDirectorySourceType {
    /// Google Workspace domain-shared contacts.
    DirectorySourceTypeDomainContact,
    /// Google Workspace domain profiles.
    DirectorySourceTypeDomainProfile,
}

/// Additional data merged into the directory sources.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GpeopleDirectoryMergeSourceType {
    /// Merge data from the authenticated user's personal contacts.
    DirectoryMergeSourceTypeContact,
}
