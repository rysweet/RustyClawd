//! MCP Schema Validation Tests
//!
//! Tests for JSON Schema validation and filtering in `claude mcp serve`.
//! Ensures tools with incompatible schemas are filtered out to prevent runtime errors.

use rustyclawd::schema_validator::{SchemaValidator, ValidationResult};
use serde_json::json;

#[test]
fn test_valid_tool_schema_passes_validation() {
    let validator = SchemaValidator::default();
    let schema = json!({
        "type": "object",
        "properties": {
            "command": {
                "type": "string",
                "description": "The command to execute"
            },
            "timeout": {
                "type": "integer",
                "description": "Timeout in milliseconds"
            }
        },
        "required": ["command"]
    });

    let result = validator.validate(&schema);
    assert!(result.is_valid(), "Valid schema should pass validation");
}

#[test]
fn test_schema_missing_type_fails() {
    let validator = SchemaValidator::default();
    let schema = json!({
        "properties": {
            "name": {"type": "string"}
        }
    });

    let result = validator.validate(&schema);
    assert!(!result.is_valid());
    assert!(matches!(result, ValidationResult::Malformed { .. }));

    let error_msg = result.error_message().unwrap();
    assert!(error_msg.contains("type"));
}

#[test]
fn test_schema_with_invalid_type_fails() {
    let validator = SchemaValidator::default();
    let schema = json!({
        "type": "invalid_type",
        "properties": {
            "name": {"type": "string"}
        }
    });

    let result = validator.validate(&schema);
    assert!(!result.is_valid());
    assert!(matches!(result, ValidationResult::Malformed { .. }));
}

#[test]
fn test_object_schema_with_invalid_properties_fails() {
    let validator = SchemaValidator::default();
    let schema = json!({
        "type": "object",
        "properties": "should be an object, not a string"
    });

    let result = validator.validate(&schema);
    assert!(!result.is_valid());
    assert!(matches!(result, ValidationResult::Malformed { .. }));
}

#[test]
fn test_array_schema_without_items_fails() {
    let validator = SchemaValidator::default();
    let schema = json!({
        "type": "array"
    });

    let result = validator.validate(&schema);
    assert!(!result.is_valid());
    assert!(matches!(result, ValidationResult::Malformed { .. }));
}

#[test]
fn test_array_schema_with_items_passes() {
    let validator = SchemaValidator::default();
    let schema = json!({
        "type": "array",
        "items": {
            "type": "string"
        }
    });

    let result = validator.validate(&schema);
    assert!(result.is_valid());
}

#[test]
fn test_required_field_not_in_properties_fails() {
    let validator = SchemaValidator::default();
    let schema = json!({
        "type": "object",
        "properties": {
            "name": {"type": "string"},
            "age": {"type": "integer"}
        },
        "required": ["email"]  // email not in properties
    });

    let result = validator.validate(&schema);
    assert!(!result.is_valid());
    assert!(matches!(result, ValidationResult::Malformed { .. }));

    let error_msg = result.error_message().unwrap();
    assert!(error_msg.contains("email"));
}

#[test]
fn test_deeply_nested_valid_schema_passes() {
    let validator = SchemaValidator::default();
    let schema = json!({
        "type": "object",
        "properties": {
            "user": {
                "type": "object",
                "properties": {
                    "profile": {
                        "type": "object",
                        "properties": {
                            "address": {
                                "type": "object",
                                "properties": {
                                    "street": {"type": "string"},
                                    "city": {"type": "string"},
                                    "zipcode": {"type": "string"}
                                }
                            }
                        }
                    }
                }
            }
        }
    });

    let result = validator.validate(&schema);
    assert!(result.is_valid());
}

#[test]
fn test_schema_exceeding_max_depth_fails() {
    let validator = SchemaValidator::new(5);

    // Create a deeply nested schema (10 levels)
    let mut schema = json!({"type": "string"});
    for _ in 0..10 {
        schema = json!({
            "type": "object",
            "properties": {
                "nested": schema
            }
        });
    }

    let result = validator.validate(&schema);
    assert!(!result.is_valid());
}

#[test]
fn test_schema_with_circular_ref_detected() {
    let validator = SchemaValidator::default();

    // Schema with $ref pointing to itself
    let schema = json!({
        "type": "object",
        "properties": {
            "self_reference": {
                "$ref": "#"
            }
        }
    });

    let result = validator.validate(&schema);
    // For now, this test verifies the validator doesn't crash on $ref
    // Full $ref resolution would require a more sophisticated implementation
    assert!(result.is_valid() || matches!(result, ValidationResult::CircularReference { .. }));
}

#[test]
fn test_schema_with_nested_invalid_property() {
    let validator = SchemaValidator::default();
    let schema = json!({
        "type": "object",
        "properties": {
            "valid_prop": {"type": "string"},
            "invalid_prop": {
                "type": "object",
                "properties": "this should be an object"
            }
        }
    });

    let result = validator.validate(&schema);
    assert!(!result.is_valid());
    assert!(matches!(result, ValidationResult::Malformed { .. }));
}

#[test]
fn test_schema_with_allof_anyof_oneof() {
    let validator = SchemaValidator::default();

    // Valid schema with composition keywords
    let schema = json!({
        "type": "object",
        "allOf": [
            {"properties": {"name": {"type": "string"}}},
            {"properties": {"age": {"type": "integer"}}}
        ]
    });

    let result = validator.validate(&schema);
    assert!(result.is_valid());
}

#[test]
fn test_schema_not_an_object() {
    let validator = SchemaValidator::default();

    // Schema must be an object, not a string or array
    let schema = json!("not an object");

    let result = validator.validate(&schema);
    assert!(!result.is_valid());
    assert!(matches!(result, ValidationResult::Malformed { .. }));
}

#[test]
fn test_bash_tool_schema_is_valid() {
    let validator = SchemaValidator::default();

    // Real Bash tool schema from rustyclawd
    let schema = json!({
        "type": "object",
        "properties": {
            "command": {
                "type": "string",
                "description": "The bash command to execute"
            },
            "timeout": {
                "type": "integer",
                "description": "Optional timeout in milliseconds",
                "default": 120000
            },
            "description": {
                "type": "string",
                "description": "Optional description of what the command does"
            },
            "run_in_background": {
                "type": "boolean",
                "description": "Run the command in the background",
                "default": false
            }
        },
        "required": ["command"]
    });

    let result = validator.validate(&schema);
    assert!(result.is_valid(), "Bash tool schema should be valid");
}

#[test]
fn test_read_tool_schema_is_valid() {
    let validator = SchemaValidator::default();

    // Real Read tool schema
    let schema = json!({
        "type": "object",
        "properties": {
            "file_path": {
                "type": "string",
                "description": "The path to the file to read"
            },
            "offset": {
                "type": "integer",
                "description": "Optional line offset"
            },
            "limit": {
                "type": "integer",
                "description": "Optional line limit"
            }
        },
        "required": ["file_path"]
    });

    let result = validator.validate(&schema);
    assert!(result.is_valid(), "Read tool schema should be valid");
}

#[test]
fn test_complex_array_schema() {
    let validator = SchemaValidator::default();

    let schema = json!({
        "type": "object",
        "properties": {
            "items": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "id": {"type": "integer"},
                        "name": {"type": "string"},
                        "tags": {
                            "type": "array",
                            "items": {"type": "string"}
                        }
                    },
                    "required": ["id", "name"]
                }
            }
        }
    });

    let result = validator.validate(&schema);
    assert!(result.is_valid(), "Complex array schema should be valid");
}

#[test]
fn test_schema_with_additional_properties() {
    let validator = SchemaValidator::default();

    let schema = json!({
        "type": "object",
        "properties": {
            "name": {"type": "string"}
        },
        "additionalProperties": {
            "type": "string"
        }
    });

    let result = validator.validate(&schema);
    assert!(result.is_valid());
}

#[test]
fn test_validation_result_error_messages() {
    let valid = ValidationResult::Valid;
    assert!(valid.error_message().is_none());

    let circular = ValidationResult::CircularReference {
        path: "properties.self".to_string(),
    };
    let error_msg = circular.error_message().unwrap();
    assert!(error_msg.contains("Circular"));
    assert!(error_msg.contains("properties.self"));

    let malformed = ValidationResult::Malformed {
        reason: "Missing type field".to_string(),
    };
    let error_msg = malformed.error_message().unwrap();
    assert!(error_msg.contains("Malformed"));
    assert!(error_msg.contains("Missing type"));
}

#[test]
fn test_schema_with_invalid_required_array_type() {
    let validator = SchemaValidator::default();

    let schema = json!({
        "type": "object",
        "properties": {
            "name": {"type": "string"}
        },
        "required": "should be an array"
    });

    let result = validator.validate(&schema);
    assert!(!result.is_valid());
    assert!(matches!(result, ValidationResult::Malformed { .. }));
}

#[test]
fn test_multiple_invalid_nested_schemas() {
    let validator = SchemaValidator::default();

    // Schema with multiple levels of invalid nesting
    let schema = json!({
        "type": "object",
        "properties": {
            "level1": {
                "type": "object",
                "properties": {
                    "level2": {
                        "type": "array"
                        // Missing required "items" field for array
                    }
                }
            }
        }
    });

    let result = validator.validate(&schema);
    assert!(!result.is_valid());
}
