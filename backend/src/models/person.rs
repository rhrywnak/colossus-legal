//! Person model v2 — Represents people involved in legal cases.
//!
//! This module defines the Person struct and PersonRole enum.
//! People can be plaintiffs, defendants, witnesses, judges, attorneys, etc.

use chrono::{DateTime, NaiveDate, Utc};
use neo4rs::{DeError, Node};
use serde::{Deserialize, Serialize};

// =============================================================================
// ENUMS
// =============================================================================

/// Roles that a person can have in a legal case.
/// Uses snake_case serialization for JSON compatibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PersonRole {
    Plaintiff,
    Defendant,
    Witness,
    Judge,
    Attorney,
    InterestedParty,
    PersonalRepresentative,
    Decedent,
    Expert,
    Clerk,
    Mediator,
    Guardian,
    Conservator,
    Other,
}

// =============================================================================
// DISPLAY IMPLEMENTATION
// =============================================================================

/// Display impl for PersonRole — returns human-readable string.
impl std::fmt::Display for PersonRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Plaintiff => "Plaintiff",
            Self::Defendant => "Defendant",
            Self::Witness => "Witness",
            Self::Judge => "Judge",
            Self::Attorney => "Attorney",
            Self::InterestedParty => "Interested Party",
            Self::PersonalRepresentative => "Personal Representative",
            Self::Decedent => "Decedent",
            Self::Expert => "Expert",
            Self::Clerk => "Clerk",
            Self::Mediator => "Mediator",
            Self::Guardian => "Guardian",
            Self::Conservator => "Conservator",
            Self::Other => "Other",
        };
        write!(f, "{s}")
    }
}

// =============================================================================
// PERSON STRUCT
// =============================================================================

/// A person involved in a legal case.
///
/// People can be parties (plaintiff, defendant), legal professionals
/// (attorney, judge), or other participants (witness, expert).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Person {
    // --- Identity ---
    pub id: String,
    pub name: String,

    // --- Role ---
    /// Role in the case (plaintiff, defendant, witness, etc.)
    pub role: PersonRole,

    /// Additional role details (e.g., "Attorney for Plaintiff")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role_description: Option<String>,

    // --- Affiliation ---
    /// Associated organization (law firm, company, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization: Option<String>,

    /// Legacy affiliation field (preserved from v1)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub affiliation: Option<String>,

    // --- Flags ---
    /// Is this person a party to the case?
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_party: Option<bool>,

    // --- Contact (preserved from v1) ---
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,

    // --- Other (preserved from v1) ---
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_of_birth: Option<NaiveDate>,

    // --- Timestamps ---
    pub created_at: DateTime<Utc>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
}

// =============================================================================
// ERROR TYPES
// =============================================================================

/// Error type for converting Neo4j Node to Person.
#[derive(Debug)]
pub enum PersonConversionError {
    MissingField(&'static str),
    Value(DeError),
    Neo4j(neo4rs::Error),
}

impl From<neo4rs::Error> for PersonConversionError {
    fn from(value: neo4rs::Error) -> Self {
        PersonConversionError::Neo4j(value)
    }
}

impl From<DeError> for PersonConversionError {
    fn from(value: DeError) -> Self {
        PersonConversionError::Value(value)
    }
}

// =============================================================================
// NEO4J CONVERSION
// =============================================================================

/// Parse a role string from Neo4j into PersonRole enum.
/// Returns PersonRole::Other for unknown values.
fn parse_role(s: &str) -> PersonRole {
    serde_json::from_value(serde_json::Value::String(s.to_string()))
        .unwrap_or(PersonRole::Other)
}

/// Parse created_at which may be stored as String or DateTime in Neo4j.
/// Returns current time if parsing fails.
fn parse_created_at(node: &Node) -> DateTime<Utc> {
    // Try as DateTime first
    if let Ok(dt) = node.get::<DateTime<Utc>>("created_at") {
        return dt;
    }

    // Try as String (ISO-8601 / RFC-3339)
    if let Ok(s) = node.get::<String>("created_at") {
        if let Ok(dt) = DateTime::parse_from_rfc3339(&s) {
            return dt.with_timezone(&Utc);
        }
    }

    // Fallback to now
    Utc::now()
}

impl TryFrom<Node> for Person {
    type Error = PersonConversionError;

    fn try_from(node: Node) -> Result<Self, Self::Error> {
        // Required fields
        let id: String = node.get("id").map_err(PersonConversionError::from)?;
        let name: String = node
            .get("name")
            .map_err(|_| PersonConversionError::MissingField("name"))?;

        // role: parse from string with fallback to Other
        let role_str: String = node.get("role").unwrap_or_else(|_| "other".to_string());
        let role = parse_role(&role_str);

        // created_at: handle both DateTime and String formats
        let created_at = parse_created_at(&node);

        // Optional fields
        let role_description: Option<String> = node.get("role_description").ok();
        let organization: Option<String> = node.get("organization").ok();
        let affiliation: Option<String> = node.get("affiliation").ok();
        let is_party: Option<bool> = node.get("is_party").ok();
        let email: Option<String> = node.get("email").ok();
        let phone: Option<String> = node.get("phone").ok();
        let notes: Option<String> = node.get("notes").ok();
        let date_of_birth: Option<NaiveDate> = node.get("date_of_birth").ok();
        let updated_at: Option<DateTime<Utc>> = node.get("updated_at").ok();

        Ok(Self {
            id,
            name,
            role,
            role_description,
            organization,
            affiliation,
            is_party,
            email,
            phone,
            notes,
            date_of_birth,
            created_at,
            updated_at,
        })
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to create a minimal valid Person for testing.
    fn make_test_person() -> Person {
        Person {
            id: "person-001".to_string(),
            name: "John Smith".to_string(),
            role: PersonRole::Attorney,
            role_description: Some("Attorney for Plaintiff".to_string()),
            organization: Some("Smith & Associates".to_string()),
            affiliation: None,
            is_party: Some(false),
            email: Some("john@smithlaw.com".to_string()),
            phone: Some("555-1234".to_string()),
            notes: None,
            date_of_birth: None,
            created_at: Utc::now(),
            updated_at: None,
        }
    }

    #[test]
    fn test_person_serialization_roundtrip() {
        let person = make_test_person();

        // Serialize to JSON
        let json = serde_json::to_string(&person).expect("Failed to serialize Person");

        // Deserialize back
        let restored: Person =
            serde_json::from_str(&json).expect("Failed to deserialize Person");

        // Verify key fields match
        assert_eq!(restored.id, person.id);
        assert_eq!(restored.name, person.name);
        assert_eq!(restored.role, person.role);
    }

    #[test]
    fn test_person_role_serializes_snake_case() {
        // Test InterestedParty becomes "interested_party"
        let role = PersonRole::InterestedParty;
        let json = serde_json::to_string(&role).expect("Failed to serialize");

        assert_eq!(json, "\"interested_party\"");

        // Test deserialization
        let restored: PersonRole =
            serde_json::from_str("\"interested_party\"").expect("Failed to deserialize");
        assert_eq!(restored, PersonRole::InterestedParty);

        // Test PersonalRepresentative
        let pr = PersonRole::PersonalRepresentative;
        let json2 = serde_json::to_string(&pr).expect("Failed to serialize");
        assert_eq!(json2, "\"personal_representative\"");
    }

    #[test]
    fn test_person_optional_fields_skip_null() {
        // Create person with minimal fields
        let person = Person {
            id: "person-002".to_string(),
            name: "Jane Doe".to_string(),
            role: PersonRole::Witness,
            role_description: None,
            organization: None,
            affiliation: None,
            is_party: None,
            email: None,
            phone: None,
            notes: None,
            date_of_birth: None,
            created_at: Utc::now(),
            updated_at: None,
        };

        let json = serde_json::to_string(&person).expect("Failed to serialize");

        // Verify None fields are not present
        assert!(!json.contains("\"role_description\""));
        assert!(!json.contains("\"organization\""));
        assert!(!json.contains("\"email\""));
        assert!(!json.contains("\"updated_at\""));

        // Required fields should be present
        assert!(json.contains("\"id\""));
        assert!(json.contains("\"name\""));
        assert!(json.contains("\"role\""));
        assert!(json.contains("\"created_at\""));
    }

    #[test]
    fn test_person_role_display() {
        assert_eq!(PersonRole::Plaintiff.to_string(), "Plaintiff");
        assert_eq!(PersonRole::InterestedParty.to_string(), "Interested Party");
        assert_eq!(PersonRole::PersonalRepresentative.to_string(), "Personal Representative");
        assert_eq!(PersonRole::Other.to_string(), "Other");
    }

    #[test]
    fn test_parse_role_unknown_returns_other() {
        // Unknown values should return Other
        assert_eq!(parse_role("unknown_role"), PersonRole::Other);
        assert_eq!(parse_role("lawyer"), PersonRole::Other);
        assert_eq!(parse_role(""), PersonRole::Other);

        // Known values should parse correctly
        assert_eq!(parse_role("plaintiff"), PersonRole::Plaintiff);
        assert_eq!(parse_role("interested_party"), PersonRole::InterestedParty);
        assert_eq!(parse_role("personal_representative"), PersonRole::PersonalRepresentative);
    }
}
