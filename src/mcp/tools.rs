//! MCP tool types and helpers for Medulla.
//!
//! This module contains parameter types, result types, and validation helpers
//! for MCP tools. The actual tool implementations are in mod.rs within the
//! #[tool_router] impl block.

use crate::entity::{
    Component, ComponentStatus, Decision, DecisionStatus, Link, Note, Prompt, Relation, Task,
    TaskPriority, TaskStatus,
};
use crate::mcp::error::{validation, McpError, VALID_ENTITY_TYPES};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// ============================================================================
// Parameter and Result Types
// ============================================================================

/// Parameters for entity_create tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EntityCreateParams {
    /// Entity type: decision, task, note, prompt, component, or link
    #[serde(rename = "type")]
    pub entity_type: String,
    /// Title for the entity (required, 1-500 characters)
    pub title: String,
    /// Optional content/description
    pub content: Option<String>,
    /// Optional tags
    pub tags: Option<Vec<String>>,
    /// Type-specific properties
    pub properties: Option<serde_json::Value>,
}

/// Parameters for entity_get tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EntityGetParams {
    /// Entity ID (sequence number like "1" or UUID prefix like "abc123")
    pub id: String,
    /// Optional entity type hint for faster lookup
    #[serde(rename = "type")]
    pub entity_type: Option<String>,
}

/// Parameters for entity_list tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EntityListParams {
    /// Filter by entity type
    #[serde(rename = "type")]
    pub entity_type: Option<String>,
    /// Filter by status (for decision/task/component)
    pub status: Option<String>,
    /// Filter by tag
    pub tag: Option<String>,
    /// Maximum results (default 50, max 100)
    pub limit: Option<u32>,
    /// Offset for pagination
    pub offset: Option<u32>,
}

/// Parameters for entity_update tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EntityUpdateParams {
    /// Entity ID to update
    pub id: String,
    /// New title
    pub title: Option<String>,
    /// New content
    pub content: Option<String>,
    /// Tags to add
    pub add_tags: Option<Vec<String>>,
    /// Tags to remove
    pub remove_tags: Option<Vec<String>>,
    /// Type-specific properties to update
    pub properties: Option<serde_json::Value>,
}

/// Parameters for entity_delete tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EntityDeleteParams {
    /// Entity ID to delete
    pub id: String,
}

/// A single operation in a batch
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "op")]
pub enum BatchOperation {
    #[serde(rename = "create")]
    Create(EntityCreateParams),
    #[serde(rename = "update")]
    Update(EntityUpdateParams),
    #[serde(rename = "delete")]
    Delete(EntityDeleteParams),
}

/// Parameters for entity_batch tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EntityBatchParams {
    /// Operations to perform (max 100)
    pub operations: Vec<BatchOperation>,
}

/// Result of a single batch operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchOperationResult {
    pub index: usize,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<BatchError>,
}

/// Error in a batch operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchError {
    pub code: String,
    pub message: String,
}

/// Result of entity_batch tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchResult {
    pub results: Vec<BatchOperationResult>,
    pub succeeded: usize,
    pub failed: usize,
}

/// Parameters for search_fulltext tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SearchFulltextParams {
    /// Search query
    pub query: String,
    /// Optional entity type filter
    #[serde(rename = "type")]
    pub entity_type: Option<String>,
    /// Maximum results (default 50, max 100)
    pub limit: Option<u32>,
}

/// Parameters for graph_relations tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GraphRelationsParams {
    /// Entity ID
    pub id: String,
    /// Direction: "from", "to", or "both" (default: "both")
    pub direction: Option<String>,
}

/// Parameters for graph_path tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GraphPathParams {
    /// Source entity ID
    pub from_id: String,
    /// Target entity ID
    pub to_id: String,
    /// Maximum traversal depth (default 10)
    pub max_depth: Option<u32>,
}

/// Parameters for graph_orphans tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GraphOrphansParams {
    /// Filter by entity type
    #[serde(rename = "type")]
    pub entity_type: Option<String>,
    /// Maximum results (default 50, max 100)
    pub limit: Option<u32>,
}

// ============================================================================
// Task Queue Tool Parameters (Beads Parity)
// ============================================================================

/// Parameters for task_ready tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TaskReadyParams {
    /// Maximum results (default 50, max 100)
    pub limit: Option<u32>,
}

/// Parameters for task_blocked tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TaskBlockedParams {
    /// Optional task ID to get blockers for a specific task
    pub id: Option<String>,
    /// Maximum results when listing all blocked tasks (default 50, max 100)
    pub limit: Option<u32>,
}

/// Parameters for task_complete tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TaskCompleteParams {
    /// Task ID to mark as done
    pub id: String,
}

/// Parameters for task_reschedule tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TaskRescheduleParams {
    /// Task ID to reschedule
    pub id: String,
    /// New due date in ISO 8601 format (YYYY-MM-DD)
    pub due_date: String,
}

/// Parameters for decision_supersede tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DecisionSupersedeParams {
    /// ID of the old decision to supersede
    pub old_id: String,
    /// ID of the new decision that supersedes it
    pub new_id: String,
}

/// Parameters for relation_create tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RelationCreateParams {
    /// Source entity ID (sequence number or UUID prefix)
    pub source_id: String,
    /// Target entity ID (sequence number or UUID prefix)
    pub target_id: String,
    /// Relation type: implements, blocks, supersedes, references, belongs_to, documents
    pub relation_type: String,
}

/// Parameters for relation_delete tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RelationDeleteParams {
    /// Source entity ID (sequence number or UUID prefix)
    pub source_id: String,
    /// Target entity ID (sequence number or UUID prefix)
    pub target_id: String,
    /// Relation type: implements, blocks, supersedes, references, belongs_to, documents
    pub relation_type: String,
}

/// A serializable entity response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityResponse {
    pub id: String,
    pub sequence_number: u32,
    #[serde(rename = "type")]
    pub entity_type: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    pub tags: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_by: Option<String>,
    pub properties: serde_json::Value,
}

/// A relation in response format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationResponse {
    pub source_id: String,
    pub source_type: String,
    pub target_id: String,
    pub target_type: String,
    pub relation_type: String,
    pub created_at: String,
}

// ============================================================================
// Validation Helpers
// ============================================================================

pub fn validate_title(title: &str) -> Result<(), McpError> {
    let trimmed = title.trim();
    if trimmed.is_empty() {
        return Err(McpError::TitleRequired);
    }
    if trimmed.len() > validation::MAX_TITLE_LENGTH {
        return Err(McpError::TitleTooLong {
            max: validation::MAX_TITLE_LENGTH,
            actual: trimmed.len(),
        });
    }
    Ok(())
}

pub fn validate_content(content: &Option<String>) -> Result<(), McpError> {
    if let Some(c) = content {
        if c.len() > validation::MAX_CONTENT_SIZE {
            return Err(McpError::ContentTooLarge {
                max: validation::MAX_CONTENT_SIZE,
                actual: c.len(),
            });
        }
    }
    Ok(())
}

pub fn validate_entity_type(entity_type: &str) -> Result<(), McpError> {
    if !VALID_ENTITY_TYPES.contains(&entity_type) {
        return Err(McpError::EntityTypeInvalid {
            provided: entity_type.to_string(),
            valid: VALID_ENTITY_TYPES.iter().map(|s| s.to_string()).collect(),
        });
    }
    Ok(())
}

pub fn validate_tags(tags: &Option<Vec<String>>) -> Result<(), McpError> {
    if let Some(t) = tags {
        if t.len() > validation::MAX_TAGS_COUNT {
            return Err(McpError::ValidationFailed {
                field: "tags".to_string(),
                message: format!("Maximum {} tags allowed", validation::MAX_TAGS_COUNT),
            });
        }
        for tag in t {
            if tag.len() > validation::MAX_TAG_LENGTH {
                return Err(McpError::ValidationFailed {
                    field: "tags".to_string(),
                    message: format!(
                        "Tag '{}' exceeds maximum length of {}",
                        tag,
                        validation::MAX_TAG_LENGTH
                    ),
                });
            }
        }
    }
    Ok(())
}

pub fn parse_decision_status(s: &str) -> Result<DecisionStatus, McpError> {
    s.parse().map_err(|_| McpError::InvalidEnumValue {
        field: "status".to_string(),
        value: s.to_string(),
        valid: vec![
            "proposed".to_string(),
            "accepted".to_string(),
            "deprecated".to_string(),
            "superseded".to_string(),
        ],
    })
}

pub fn parse_task_status(s: &str) -> Result<TaskStatus, McpError> {
    s.parse().map_err(|_| McpError::InvalidEnumValue {
        field: "status".to_string(),
        value: s.to_string(),
        valid: vec![
            "todo".to_string(),
            "in_progress".to_string(),
            "done".to_string(),
            "blocked".to_string(),
        ],
    })
}

pub fn parse_task_priority(s: &str) -> Result<TaskPriority, McpError> {
    s.parse().map_err(|_| McpError::InvalidEnumValue {
        field: "priority".to_string(),
        value: s.to_string(),
        valid: vec![
            "low".to_string(),
            "normal".to_string(),
            "high".to_string(),
            "urgent".to_string(),
        ],
    })
}

pub fn parse_component_status(s: &str) -> Result<ComponentStatus, McpError> {
    s.parse().map_err(|_| McpError::InvalidEnumValue {
        field: "status".to_string(),
        value: s.to_string(),
        valid: vec![
            "active".to_string(),
            "deprecated".to_string(),
            "planned".to_string(),
        ],
    })
}

pub fn parse_date(field: &str, value: &str) -> Result<chrono::NaiveDate, McpError> {
    chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d").map_err(|_| McpError::InvalidDateFormat {
        field: field.to_string(),
        value: value.to_string(),
    })
}

pub fn validate_url(url: &str) -> Result<(), McpError> {
    if url.len() > validation::MAX_URL_SIZE {
        return Err(McpError::ValidationFailed {
            field: "url".to_string(),
            message: format!("URL exceeds maximum length of {}", validation::MAX_URL_SIZE),
        });
    }
    // Basic URL validation - check for scheme
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err(McpError::InvalidUrl {
            value: url.to_string(),
        });
    }
    Ok(())
}

// ============================================================================
// Entity Conversion Helpers
// ============================================================================

pub fn decision_to_response(d: &Decision) -> EntityResponse {
    let props = serde_json::json!({
        "status": d.status.to_string(),
        "context": d.context,
        "consequences": d.consequences,
        "superseded_by": d.superseded_by,
    });
    EntityResponse {
        id: d.base.id.to_string(),
        sequence_number: d.base.sequence_number,
        entity_type: "decision".to_string(),
        title: d.base.title.clone(),
        content: d.base.content.clone(),
        tags: d.base.tags.clone(),
        created_at: d.base.created_at.to_rfc3339(),
        updated_at: d.base.updated_at.to_rfc3339(),
        created_by: d.base.created_by.clone(),
        properties: props,
    }
}

pub fn task_to_response(t: &Task) -> EntityResponse {
    let props = serde_json::json!({
        "status": t.status.to_string(),
        "priority": t.priority.to_string(),
        "due_date": t.due_date.map(|d| d.to_string()),
        "assignee": t.assignee,
    });
    EntityResponse {
        id: t.base.id.to_string(),
        sequence_number: t.base.sequence_number,
        entity_type: "task".to_string(),
        title: t.base.title.clone(),
        content: t.base.content.clone(),
        tags: t.base.tags.clone(),
        created_at: t.base.created_at.to_rfc3339(),
        updated_at: t.base.updated_at.to_rfc3339(),
        created_by: t.base.created_by.clone(),
        properties: props,
    }
}

pub fn note_to_response(n: &Note) -> EntityResponse {
    let props = serde_json::json!({
        "note_type": n.note_type,
    });
    EntityResponse {
        id: n.base.id.to_string(),
        sequence_number: n.base.sequence_number,
        entity_type: "note".to_string(),
        title: n.base.title.clone(),
        content: n.base.content.clone(),
        tags: n.base.tags.clone(),
        created_at: n.base.created_at.to_rfc3339(),
        updated_at: n.base.updated_at.to_rfc3339(),
        created_by: n.base.created_by.clone(),
        properties: props,
    }
}

pub fn prompt_to_response(p: &Prompt) -> EntityResponse {
    let props = serde_json::json!({
        "template": p.template,
        "variables": p.variables,
        "output_schema": p.output_schema,
    });
    EntityResponse {
        id: p.base.id.to_string(),
        sequence_number: p.base.sequence_number,
        entity_type: "prompt".to_string(),
        title: p.base.title.clone(),
        content: p.base.content.clone(),
        tags: p.base.tags.clone(),
        created_at: p.base.created_at.to_rfc3339(),
        updated_at: p.base.updated_at.to_rfc3339(),
        created_by: p.base.created_by.clone(),
        properties: props,
    }
}

pub fn component_to_response(c: &Component) -> EntityResponse {
    let props = serde_json::json!({
        "component_type": c.component_type,
        "status": c.status.to_string(),
        "owner": c.owner,
    });
    EntityResponse {
        id: c.base.id.to_string(),
        sequence_number: c.base.sequence_number,
        entity_type: "component".to_string(),
        title: c.base.title.clone(),
        content: c.base.content.clone(),
        tags: c.base.tags.clone(),
        created_at: c.base.created_at.to_rfc3339(),
        updated_at: c.base.updated_at.to_rfc3339(),
        created_by: c.base.created_by.clone(),
        properties: props,
    }
}

pub fn link_to_response(l: &Link) -> EntityResponse {
    let props = serde_json::json!({
        "url": l.url,
        "link_type": l.link_type,
    });
    EntityResponse {
        id: l.base.id.to_string(),
        sequence_number: l.base.sequence_number,
        entity_type: "link".to_string(),
        title: l.base.title.clone(),
        content: l.base.content.clone(),
        tags: l.base.tags.clone(),
        created_at: l.base.created_at.to_rfc3339(),
        updated_at: l.base.updated_at.to_rfc3339(),
        created_by: l.base.created_by.clone(),
        properties: props,
    }
}

pub fn relation_to_response(r: &Relation) -> RelationResponse {
    RelationResponse {
        source_id: r.source_id.to_string(),
        source_type: r.source_type.clone(),
        target_id: r.target_id.to_string(),
        target_type: r.target_type.clone(),
        relation_type: r.relation_type.to_string(),
        created_at: r.created_at.to_rfc3339(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_title() {
        assert!(validate_title("Valid title").is_ok());
        assert!(validate_title("").is_err());
        assert!(validate_title("   ").is_err());
        assert!(validate_title(&"x".repeat(501)).is_err());
    }

    #[test]
    fn test_validate_entity_type() {
        assert!(validate_entity_type("decision").is_ok());
        assert!(validate_entity_type("task").is_ok());
        assert!(validate_entity_type("invalid").is_err());
    }

    #[test]
    fn test_validate_url() {
        assert!(validate_url("https://example.com").is_ok());
        assert!(validate_url("http://example.com").is_ok());
        assert!(validate_url("ftp://example.com").is_err());
        assert!(validate_url("not-a-url").is_err());
    }

    #[test]
    fn test_parse_decision_status() {
        assert!(parse_decision_status("proposed").is_ok());
        assert!(parse_decision_status("accepted").is_ok());
        assert!(parse_decision_status("invalid").is_err());
    }

    #[test]
    fn test_parse_task_status() {
        assert!(parse_task_status("todo").is_ok());
        assert!(parse_task_status("in_progress").is_ok());
        assert!(parse_task_status("invalid").is_err());
    }

    #[test]
    fn test_parse_date() {
        assert!(parse_date("due_date", "2025-01-31").is_ok());
        assert!(parse_date("due_date", "not-a-date").is_err());
        assert!(parse_date("due_date", "01-31-2025").is_err());
    }
}
