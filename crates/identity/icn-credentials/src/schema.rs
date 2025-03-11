//! Credential schema types for verifiable credentials

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A schema property definition
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SchemaProperty {
    /// The type of this property (string, number, boolean, etc.)
    #[serde(rename = "type")]
    pub type_: String,
    
    /// Description of this property
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    
    /// Whether this property is required
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
    
    /// Format of this property (e.g., email, date, url)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    
    /// Pattern for string properties
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
    
    /// Minimum value for number properties
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimum: Option<f64>,
    
    /// Maximum value for number properties
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maximum: Option<f64>,
    
    /// Minimum length for string properties
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_length: Option<usize>,
    
    /// Maximum length for string properties
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_length: Option<usize>,
    
    /// Enum values for string properties
    #[serde(rename = "enum", skip_serializing_if = "Option::is_none")]
    pub enum_values: Option<Vec<String>>,
}

impl SchemaProperty {
    /// Create a new schema property
    pub fn new(type_: &str) -> Self {
        SchemaProperty {
            type_: type_.to_string(),
            description: None,
            required: None,
            format: None,
            pattern: None,
            minimum: None,
            maximum: None,
            min_length: None,
            max_length: None,
            enum_values: None,
        }
    }
    
    /// Set a description
    pub fn with_description(mut self, description: &str) -> Self {
        self.description = Some(description.to_string());
        self
    }
    
    /// Set required flag
    pub fn with_required(mut self, required: bool) -> Self {
        self.required = Some(required);
        self
    }
    
    /// Set a format
    pub fn with_format(mut self, format: &str) -> Self {
        self.format = Some(format.to_string());
        self
    }
    
    /// Set a pattern
    pub fn with_pattern(mut self, pattern: &str) -> Self {
        self.pattern = Some(pattern.to_string());
        self
    }
    
    /// Set minimum value
    pub fn with_minimum(mut self, minimum: f64) -> Self {
        self.minimum = Some(minimum);
        self
    }
    
    /// Set maximum value
    pub fn with_maximum(mut self, maximum: f64) -> Self {
        self.maximum = Some(maximum);
        self
    }
    
    /// Set minimum length
    pub fn with_min_length(mut self, min_length: usize) -> Self {
        self.min_length = Some(min_length);
        self
    }
    
    /// Set maximum length
    pub fn with_max_length(mut self, max_length: usize) -> Self {
        self.max_length = Some(max_length);
        self
    }
    
    /// Set enum values
    pub fn with_enum_values(mut self, enum_values: Vec<String>) -> Self {
        self.enum_values = Some(enum_values);
        self
    }
}

/// A credential schema
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CredentialSchema {
    /// Schema identifier
    pub id: String,
    
    /// Schema type
    #[serde(rename = "type")]
    pub type_: String,
    
    /// Properties of this schema
    pub properties: HashMap<String, SchemaProperty>,
    
    /// Required properties
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub required: Vec<String>,
    
    /// Additional properties allowed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_properties: Option<bool>,
}

impl CredentialSchema {
    /// Create a new credential schema
    pub fn new(id: &str, type_: &str) -> Self {
        CredentialSchema {
            id: id.to_string(),
            type_: type_.to_string(),
            properties: HashMap::new(),
            required: Vec::new(),
            additional_properties: None,
        }
    }
    
    /// Add a property to the schema
    pub fn add_property(&mut self, name: &str, property: SchemaProperty) {
        if let Some(true) = property.required {
            self.required.push(name.to_string());
        }
        self.properties.insert(name.to_string(), property);
    }
    
    /// Set whether additional properties are allowed
    pub fn allow_additional_properties(&mut self, allow: bool) {
        self.additional_properties = Some(allow);
    }
    
    /// Validate a credential subject against this schema
    pub fn validate_subject(&self, subject: &super::CredentialSubject) -> Result<(), String> {
        // Check required properties
        for required_prop in &self.required {
            if !subject.properties.contains_key(required_prop) {
                return Err(format!("Missing required property: {}", required_prop));
            }
        }
        
        // Check each property against its schema
        for (prop_name, prop_value) in &subject.properties {
            // Skip if additional properties are allowed and this property is not in the schema
            if !self.properties.contains_key(prop_name) {
                if let Some(false) = self.additional_properties {
                    return Err(format!("Unknown property not allowed: {}", prop_name));
                }
                continue;
            }
            
            let property_schema = &self.properties[prop_name];
            
            // Type validation
            match property_schema.type_.as_str() {
                "string" => {
                    if !prop_value.is_string() {
                        return Err(format!("Property '{}' must be a string", prop_name));
                    }
                    
                    if let Some(value) = prop_value.as_str() {
                        // String length validation
                        if let Some(min_length) = property_schema.min_length {
                            if value.len() < min_length {
                                return Err(format!("Property '{}' too short (min {})", prop_name, min_length));
                            }
                        }
                        
                        if let Some(max_length) = property_schema.max_length {
                            if value.len() > max_length {
                                return Err(format!("Property '{}' too long (max {})", prop_name, max_length));
                            }
                        }
                        
                        // Pattern validation
                        if let Some(pattern) = &property_schema.pattern {
                            // TODO: Use regex crate to validate pattern
                            // For now, just report that we'd validate if implemented
                            println!("Pattern validation for '{}' would check pattern: {}", prop_name, pattern);
                        }
                        
                        // Enum validation
                        if let Some(enum_values) = &property_schema.enum_values {
                            if !enum_values.contains(&value.to_string()) {
                                return Err(format!("Property '{}' not in allowed values", prop_name));
                            }
                        }
                    }
                },
                "number" | "integer" => {
                    if !prop_value.is_number() {
                        return Err(format!("Property '{}' must be a number", prop_name));
                    }
                    
                    if property_schema.type_ == "integer" && !prop_value.as_i64().is_some() {
                        return Err(format!("Property '{}' must be an integer", prop_name));
                    }
                    
                    if let Some(value) = prop_value.as_f64() {
                        // Range validation
                        if let Some(minimum) = property_schema.minimum {
                            if value < minimum {
                                return Err(format!("Property '{}' below minimum {}", prop_name, minimum));
                            }
                        }
                        
                        if let Some(maximum) = property_schema.maximum {
                            if value > maximum {
                                return Err(format!("Property '{}' above maximum {}", prop_name, maximum));
                            }
                        }
                    }
                },
                "boolean" => {
                    if !prop_value.is_boolean() {
                        return Err(format!("Property '{}' must be a boolean", prop_name));
                    }
                },
                "array" => {
                    if !prop_value.is_array() {
                        return Err(format!("Property '{}' must be an array", prop_name));
                    }
                    // TODO: Implement array validation (items, minItems, maxItems)
                },
                "object" => {
                    if !prop_value.is_object() {
                        return Err(format!("Property '{}' must be an object", prop_name));
                    }
                    // TODO: Implement object validation (properties)
                },
                _ => {
                    return Err(format!("Unknown property type: {}", property_schema.type_));
                }
            }
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::CredentialSubject;
    
    #[test]
    fn test_schema_property() {
        let property = SchemaProperty::new("string")
            .with_description("A test property")
            .with_required(true)
            .with_min_length(3)
            .with_max_length(50)
            .with_pattern("^[A-Za-z]+$");
            
        assert_eq!(property.type_, "string");
        assert_eq!(property.description, Some("A test property".to_string()));
        assert_eq!(property.required, Some(true));
        assert_eq!(property.min_length, Some(3));
        assert_eq!(property.max_length, Some(50));
        assert_eq!(property.pattern, Some("^[A-Za-z]+$".to_string()));
    }
    
    #[test]
    fn test_credential_schema() {
        let mut schema = CredentialSchema::new(
            "https://icn.coop/schemas/MembershipCredential",
            "JsonSchemaValidator2023",
        );
        
        schema.add_property("name", SchemaProperty::new("string")
            .with_description("Full name of the member")
            .with_required(true)
            .with_min_length(2));
            
        schema.add_property("membershipId", SchemaProperty::new("string")
            .with_description("Unique membership identifier")
            .with_required(true)
            .with_pattern("^M[0-9]{6}$"));
            
        schema.add_property("joinDate", SchemaProperty::new("string")
            .with_description("Date when the member joined")
            .with_required(true)
            .with_format("date"));
            
        schema.add_property("membershipType", SchemaProperty::new("string")
            .with_description("Type of membership")
            .with_required(true)
            .with_enum_values(vec![
                "Full".to_string(),
                "Associate".to_string(),
                "Honorary".to_string(),
            ]));
            
        schema.add_property("active", SchemaProperty::new("boolean")
            .with_description("Whether the membership is active")
            .with_required(true));
            
        schema.add_property("contributionHours", SchemaProperty::new("number")
            .with_description("Monthly contribution hours")
            .with_minimum(0.0)
            .with_maximum(100.0));
            
        assert_eq!(schema.id, "https://icn.coop/schemas/MembershipCredential");
        assert_eq!(schema.type_, "JsonSchemaValidator2023");
        assert_eq!(schema.properties.len(), 6);
        assert_eq!(schema.required.len(), 5);
        
        // Create a valid subject
        let mut subject = CredentialSubject::new(Some("did:icn:test:123".to_string()));
        subject.add_property("name", "Jane Smith");
        subject.add_property("membershipId", "M123456");
        subject.add_property("joinDate", "2023-01-15");
        subject.add_property("membershipType", "Full");
        subject.add_property("active", true);
        subject.add_property("contributionHours", 20);
        
        // This should validate
        assert!(schema.validate_subject(&subject).is_ok());
        
        // Create an invalid subject (missing required property)
        let mut invalid_subject = CredentialSubject::new(Some("did:icn:test:456".to_string()));
        invalid_subject.add_property("name", "John Doe");
        invalid_subject.add_property("membershipId", "M654321");
        invalid_subject.add_property("joinDate", "2023-02-20");
        // Missing "membershipType"
        invalid_subject.add_property("active", true);
        
        // This should fail validation
        assert!(schema.validate_subject(&invalid_subject).is_err());
    }
} 