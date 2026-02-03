//! MCP-specific error types and mapping to JSON-RPC error codes.

use crate::error::MedullaError;
use rmcp::model::ErrorCode;
use rmcp::ErrorData as RmcpError;
use serde::{Deserialize, Serialize};
use serde_json::json;
use thiserror::Error;

/// Custom MCP error codes (in the -32000 to -32099 range for server errors)
pub mod error_codes {
    pub const ENTITY_NOT_FOUND: i32 = -32001;
    pub const ENTITY_TYPE_INVALID: i32 = -32002;
    pub const VALIDATION_FAILED: i32 = -32003;
    pub const RELATION_TARGET_NOT_FOUND: i32 = -32004;
    pub const RESOURCE_NOT_FOUND: i32 = -32005;
    pub const INVALID_RESOURCE_URI: i32 = -32006;
    pub const STORAGE_ERROR: i32 = -32010;
    pub const INTERNAL_ERROR: i32 = -32011;
}

/// MCP-specific error types with detailed context.
#[derive(Error, Debug, Clone, Serialize, Deserialize)]
pub enum McpError {
    // Entity errors
    #[error("Entity not found: {id}")]
    EntityNotFound { id: String },

    #[error("Invalid entity type '{provided}'. Valid types: {}", valid.join(", "))]
    EntityTypeInvalid { provided: String, valid: Vec<String> },

    // Validation errors
    #[error("Validation failed for field '{field}': {message}")]
    ValidationFailed { field: String, message: String },

    #[error("Title is required")]
    TitleRequired,

    #[error("Title too long: {actual} characters (max {max})")]
    TitleTooLong { max: usize, actual: usize },

    #[error("Content too large: {actual} bytes (max {max})")]
    ContentTooLarge { max: usize, actual: usize },

    #[error("Invalid value '{value}' for field '{field}'. Valid values: {}", valid.join(", "))]
    InvalidEnumValue {
        field: String,
        value: String,
        valid: Vec<String>,
    },

    #[error("Invalid date format for field '{field}': '{value}'. Expected ISO 8601 (YYYY-MM-DD)")]
    InvalidDateFormat { field: String, value: String },

    #[error("Invalid URL: {value}")]
    InvalidUrl { value: String },

    // Relation errors
    #[error("Relation target not found: {target_id}")]
    RelationTargetNotFound { target_id: String },

    #[error("Self-referential relation not allowed for entity: {id}")]
    SelfReferentialRelation { id: String },

    // Graph errors
    #[error("No path found from '{from}' to '{to}'")]
    PathNotFound { from: String, to: String },

    #[error("Maximum depth {max} exceeded")]
    MaxDepthExceeded { max: usize },

    // Resource errors
    #[error("Resource not found: {uri}")]
    ResourceNotFound { uri: String },

    #[error("Invalid resource URI: {uri}")]
    InvalidResourceUri { uri: String },

    // Server errors
    #[error("Storage error: {message}")]
    StorageError { message: String },

    #[error("Internal error: {message}")]
    InternalError { message: String },
}

impl McpError {
    /// Get the JSON-RPC error code for this error type.
    pub fn error_code(&self) -> i32 {
        match self {
            McpError::EntityNotFound { .. } => error_codes::ENTITY_NOT_FOUND,
            McpError::EntityTypeInvalid { .. } => error_codes::ENTITY_TYPE_INVALID,
            McpError::ValidationFailed { .. }
            | McpError::TitleRequired
            | McpError::TitleTooLong { .. }
            | McpError::ContentTooLarge { .. }
            | McpError::InvalidEnumValue { .. }
            | McpError::InvalidDateFormat { .. }
            | McpError::InvalidUrl { .. } => error_codes::VALIDATION_FAILED,
            McpError::RelationTargetNotFound { .. } | McpError::SelfReferentialRelation { .. } => {
                error_codes::RELATION_TARGET_NOT_FOUND
            }
            McpError::PathNotFound { .. } | McpError::MaxDepthExceeded { .. } => {
                error_codes::ENTITY_NOT_FOUND
            }
            McpError::ResourceNotFound { .. } => error_codes::RESOURCE_NOT_FOUND,
            McpError::InvalidResourceUri { .. } => error_codes::INVALID_RESOURCE_URI,
            McpError::StorageError { .. } => error_codes::STORAGE_ERROR,
            McpError::InternalError { .. } => error_codes::INTERNAL_ERROR,
        }
    }

    /// Get the error type name for the data payload.
    pub fn error_type(&self) -> &'static str {
        match self {
            McpError::EntityNotFound { .. } => "EntityNotFound",
            McpError::EntityTypeInvalid { .. } => "EntityTypeInvalid",
            McpError::ValidationFailed { .. } => "ValidationFailed",
            McpError::TitleRequired => "TitleRequired",
            McpError::TitleTooLong { .. } => "TitleTooLong",
            McpError::ContentTooLarge { .. } => "ContentTooLarge",
            McpError::InvalidEnumValue { .. } => "InvalidEnumValue",
            McpError::InvalidDateFormat { .. } => "InvalidDateFormat",
            McpError::InvalidUrl { .. } => "InvalidUrl",
            McpError::RelationTargetNotFound { .. } => "RelationTargetNotFound",
            McpError::SelfReferentialRelation { .. } => "SelfReferentialRelation",
            McpError::PathNotFound { .. } => "PathNotFound",
            McpError::MaxDepthExceeded { .. } => "MaxDepthExceeded",
            McpError::ResourceNotFound { .. } => "ResourceNotFound",
            McpError::InvalidResourceUri { .. } => "InvalidResourceUri",
            McpError::StorageError { .. } => "StorageError",
            McpError::InternalError { .. } => "InternalError",
        }
    }

    /// Convert to rmcp ErrorData for JSON-RPC response.
    pub fn to_rmcp_error(&self) -> RmcpError {
        RmcpError {
            code: ErrorCode(self.error_code()),
            message: self.to_string().into(),
            data: Some(json!({
                "error_type": self.error_type(),
                "details": self.clone()
            })),
        }
    }
}

impl From<McpError> for RmcpError {
    fn from(err: McpError) -> Self {
        err.to_rmcp_error()
    }
}

impl From<MedullaError> for McpError {
    fn from(err: MedullaError) -> Self {
        match err {
            MedullaError::NotInitialized => McpError::StorageError {
                message: "Not in a medulla project. Run 'medulla init' first.".to_string(),
            },
            MedullaError::AlreadyInitialized => McpError::StorageError {
                message: "Already initialized".to_string(),
            },
            MedullaError::EntityNotFound(id) => McpError::EntityNotFound { id },
            MedullaError::InvalidEntityType(t) => McpError::EntityTypeInvalid {
                provided: t,
                valid: vec![
                    "decision".to_string(),
                    "task".to_string(),
                    "note".to_string(),
                    "prompt".to_string(),
                    "component".to_string(),
                    "link".to_string(),
                ],
            },
            MedullaError::Storage(msg) => McpError::StorageError { message: msg },
            MedullaError::Io(e) => McpError::StorageError {
                message: format!("IO error: {}", e),
            },
            MedullaError::Json(e) => McpError::InternalError {
                message: format!("JSON error: {}", e),
            },
            MedullaError::Loro(e) => McpError::StorageError {
                message: format!("Loro error: {}", e),
            },
            MedullaError::LoroEncode(e) => McpError::StorageError {
                message: format!("Loro encode error: {}", e),
            },
        }
    }
}

/// Valid entity types for validation.
pub const VALID_ENTITY_TYPES: &[&str] = &[
    "decision",
    "task",
    "note",
    "prompt",
    "component",
    "link",
];

/// Validation constants.
pub mod validation {
    pub const MAX_TITLE_LENGTH: usize = 500;
    pub const MAX_CONTENT_SIZE: usize = 102_400; // 100KB
    pub const MAX_TAG_LENGTH: usize = 100;
    pub const MAX_TAGS_COUNT: usize = 50;
    pub const MAX_CONTEXT_SIZE: usize = 51_200; // 50KB
    pub const MAX_CONSEQUENCE_SIZE: usize = 1024; // 1KB
    pub const MAX_TEMPLATE_SIZE: usize = 51_200; // 50KB
    pub const MAX_OUTPUT_SCHEMA_SIZE: usize = 10_240; // 10KB
    pub const MAX_URL_SIZE: usize = 2048; // 2KB
    pub const MIN_ID_PREFIX_LENGTH: usize = 4;
    pub const DEFAULT_LIMIT: usize = 50;
    pub const MAX_LIMIT: usize = 100;
    pub const DEFAULT_MAX_DEPTH: usize = 10;
    pub const MAX_BATCH_SIZE: usize = 100;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_code_mapping() {
        let err = McpError::EntityNotFound {
            id: "abc".to_string(),
        };
        assert_eq!(err.error_code(), error_codes::ENTITY_NOT_FOUND);

        let err = McpError::ValidationFailed {
            field: "title".to_string(),
            message: "too short".to_string(),
        };
        assert_eq!(err.error_code(), error_codes::VALIDATION_FAILED);
    }

    #[test]
    fn test_from_medulla_error() {
        let medulla_err = MedullaError::EntityNotFound("xyz".to_string());
        let mcp_err: McpError = medulla_err.into();
        assert!(matches!(mcp_err, McpError::EntityNotFound { id } if id == "xyz"));
    }

    #[test]
    fn test_to_rmcp_error() {
        let err = McpError::TitleRequired;
        let rmcp_err = err.to_rmcp_error();
        assert_eq!(rmcp_err.code, ErrorCode(error_codes::VALIDATION_FAILED));
        assert!(rmcp_err.message.contains("Title is required"));
    }
}
