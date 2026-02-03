//! MCP resource implementations for Medulla.
//!
//! This module provides resource handling for the MCP server, including
//! static resources (schema, stats) and dynamic resources (entities by type,
//! individual entities, task queues, etc.).

use crate::cache::SqliteCache;
use crate::mcp::error::{McpError, VALID_ENTITY_TYPES};
use crate::mcp::tools::*;
use crate::storage::LoroStore;
use rmcp::model::{RawResource, RawResourceTemplate, ReadResourceResult, ResourceContents};
use std::sync::Arc;
use tokio::sync::Mutex;

/// The medulla:// URI scheme prefix.
pub const MEDULLA_SCHEME: &str = "medulla://";

/// MIME type for all resource responses.
pub const RESOURCE_MIME_TYPE: &str = "application/json";

/// Static resource URIs (directly readable without parameters).
pub mod static_resources {
    pub const SCHEMA: &str = "medulla://schema";
    pub const STATS: &str = "medulla://stats";
    pub const ENTITIES: &str = "medulla://entities";
    pub const DECISIONS: &str = "medulla://decisions";
    pub const TASKS: &str = "medulla://tasks";
    pub const TASKS_READY: &str = "medulla://tasks/ready";
    pub const TASKS_BLOCKED: &str = "medulla://tasks/blocked";
    pub const PROMPTS: &str = "medulla://prompts";
    pub const GRAPH: &str = "medulla://graph";
}

/// Resource template URI patterns (require parameter substitution).
pub mod resource_templates {
    pub const ENTITIES_BY_TYPE: &str = "medulla://entities/{type}";
    pub const ENTITY_BY_ID: &str = "medulla://entity/{id}";
    pub const DECISIONS_ACTIVE: &str = "medulla://decisions/active";
    pub const TASKS_ACTIVE: &str = "medulla://tasks/active";
    pub const TASKS_DUE: &str = "medulla://tasks/due/{date}";
}

/// Build the list of static resources.
pub fn build_static_resources() -> Vec<RawResource> {
    vec![
        RawResource {
            uri: static_resources::SCHEMA.to_string(),
            name: "Schema".to_string(),
            title: Some("Entity Type Definitions".to_string()),
            description: Some("JSON schema for all entity types".to_string()),
            mime_type: Some(RESOURCE_MIME_TYPE.to_string()),
            size: None,
            icons: None,
            meta: None,
        },
        RawResource {
            uri: static_resources::STATS.to_string(),
            name: "Stats".to_string(),
            title: Some("Project Statistics".to_string()),
            description: Some("Entity counts, last updated, etc.".to_string()),
            mime_type: Some(RESOURCE_MIME_TYPE.to_string()),
            size: None,
            icons: None,
            meta: None,
        },
        RawResource {
            uri: static_resources::ENTITIES.to_string(),
            name: "All Entities".to_string(),
            title: Some("All Entities".to_string()),
            description: Some("List all entities across all types".to_string()),
            mime_type: Some(RESOURCE_MIME_TYPE.to_string()),
            size: None,
            icons: None,
            meta: None,
        },
        RawResource {
            uri: static_resources::DECISIONS.to_string(),
            name: "Decisions".to_string(),
            title: Some("All Decisions".to_string()),
            description: Some("List all decisions".to_string()),
            mime_type: Some(RESOURCE_MIME_TYPE.to_string()),
            size: None,
            icons: None,
            meta: None,
        },
        RawResource {
            uri: static_resources::TASKS.to_string(),
            name: "Tasks".to_string(),
            title: Some("All Tasks".to_string()),
            description: Some("List all tasks".to_string()),
            mime_type: Some(RESOURCE_MIME_TYPE.to_string()),
            size: None,
            icons: None,
            meta: None,
        },
        RawResource {
            uri: static_resources::TASKS_READY.to_string(),
            name: "Ready Tasks".to_string(),
            title: Some("Ready Tasks".to_string()),
            description: Some("Tasks with no unresolved blockers".to_string()),
            mime_type: Some(RESOURCE_MIME_TYPE.to_string()),
            size: None,
            icons: None,
            meta: None,
        },
        RawResource {
            uri: static_resources::TASKS_BLOCKED.to_string(),
            name: "Blocked Tasks".to_string(),
            title: Some("Blocked Tasks".to_string()),
            description: Some("Tasks with unresolved blockers".to_string()),
            mime_type: Some(RESOURCE_MIME_TYPE.to_string()),
            size: None,
            icons: None,
            meta: None,
        },
        RawResource {
            uri: static_resources::PROMPTS.to_string(),
            name: "Prompts".to_string(),
            title: Some("All Prompts".to_string()),
            description: Some("List all prompts".to_string()),
            mime_type: Some(RESOURCE_MIME_TYPE.to_string()),
            size: None,
            icons: None,
            meta: None,
        },
        RawResource {
            uri: static_resources::GRAPH.to_string(),
            name: "Knowledge Graph".to_string(),
            title: Some("Full Knowledge Graph".to_string()),
            description: Some("All entities and relations".to_string()),
            mime_type: Some(RESOURCE_MIME_TYPE.to_string()),
            size: None,
            icons: None,
            meta: None,
        },
    ]
}

/// Build the list of resource templates (dynamic resources with URI parameters).
pub fn build_resource_templates() -> Vec<RawResourceTemplate> {
    vec![
        RawResourceTemplate {
            uri_template: resource_templates::ENTITIES_BY_TYPE.to_string(),
            name: "Entities by Type".to_string(),
            title: Some("Entities by Type".to_string()),
            description: Some("List entities filtered by type".to_string()),
            mime_type: Some(RESOURCE_MIME_TYPE.to_string()),
            icons: None,
        },
        RawResourceTemplate {
            uri_template: resource_templates::ENTITY_BY_ID.to_string(),
            name: "Entity by ID".to_string(),
            title: Some("Single Entity".to_string()),
            description: Some("Get a single entity by ID".to_string()),
            mime_type: Some(RESOURCE_MIME_TYPE.to_string()),
            icons: None,
        },
        RawResourceTemplate {
            uri_template: resource_templates::DECISIONS_ACTIVE.to_string(),
            name: "Active Decisions".to_string(),
            title: Some("Active Decisions".to_string()),
            description: Some("Non-superseded decisions".to_string()),
            mime_type: Some(RESOURCE_MIME_TYPE.to_string()),
            icons: None,
        },
        RawResourceTemplate {
            uri_template: resource_templates::TASKS_ACTIVE.to_string(),
            name: "Active Tasks".to_string(),
            title: Some("Active Tasks".to_string()),
            description: Some("Incomplete tasks (not done)".to_string()),
            mime_type: Some(RESOURCE_MIME_TYPE.to_string()),
            icons: None,
        },
        RawResourceTemplate {
            uri_template: resource_templates::TASKS_DUE.to_string(),
            name: "Tasks Due".to_string(),
            title: Some("Tasks Due on Date".to_string()),
            description: Some("Tasks due on a specific date".to_string()),
            mime_type: Some(RESOURCE_MIME_TYPE.to_string()),
            icons: None,
        },
    ]
}

/// Parse a resource URI and return the content.
pub async fn read_resource(
    uri: &str,
    store: &Arc<Mutex<LoroStore>>,
    cache: &Arc<Mutex<SqliteCache>>,
) -> Result<ReadResourceResult, McpError> {
    if !uri.starts_with(MEDULLA_SCHEME) {
        return Err(McpError::InvalidResourceUri {
            uri: uri.to_string(),
        });
    }

    let path = &uri[MEDULLA_SCHEME.len()..];

    match path {
        "schema" => read_schema_resource(uri),
        "stats" => read_stats_resource(uri, store).await,
        "entities" => read_all_entities_resource(uri, store).await,
        "decisions" => read_decisions_resource(uri, store, false).await,
        "decisions/active" => read_decisions_resource(uri, store, true).await,
        "tasks" => read_tasks_resource(uri, store, None).await,
        "tasks/active" => read_tasks_resource(uri, store, Some("active")).await,
        "tasks/ready" => read_ready_tasks_resource(uri, cache).await,
        "tasks/blocked" => read_blocked_tasks_resource(uri, cache).await,
        "prompts" => read_prompts_resource(uri, store).await,
        "graph" => read_graph_resource(uri, store).await,
        _ => {
            // Try to match dynamic patterns
            if path.starts_with("entities/") {
                let entity_type = &path["entities/".len()..];
                return read_entities_by_type_resource(uri, store, entity_type).await;
            }
            if path.starts_with("entity/") {
                let id = &path["entity/".len()..];
                return read_entity_by_id_resource(uri, store, id).await;
            }
            if path.starts_with("tasks/due/") {
                let date = &path["tasks/due/".len()..];
                return read_tasks_due_resource(uri, store, date).await;
            }

            Err(McpError::ResourceNotFound {
                uri: uri.to_string(),
            })
        }
    }
}

/// Read the schema resource (static).
fn read_schema_resource(uri: &str) -> Result<ReadResourceResult, McpError> {
    let schema = serde_json::json!({
        "entity_types": VALID_ENTITY_TYPES,
        "decision": {
            "status": ["proposed", "accepted", "deprecated", "superseded"],
            "fields": ["context", "consequences", "superseded_by"]
        },
        "task": {
            "status": ["todo", "in_progress", "done", "blocked"],
            "priority": ["low", "normal", "high", "urgent"],
            "fields": ["due_date", "assignee"]
        },
        "note": {
            "fields": ["note_type"]
        },
        "prompt": {
            "fields": ["template", "variables", "output_schema"]
        },
        "component": {
            "status": ["active", "deprecated", "planned"],
            "fields": ["component_type", "owner"]
        },
        "link": {
            "fields": ["url", "link_type"]
        },
        "relation_types": ["blocks", "relates", "supersedes", "implements", "depends_on"]
    });

    let text = serde_json::to_string_pretty(&schema).map_err(|e| McpError::InternalError {
        message: format!("Failed to serialize schema: {}", e),
    })?;

    Ok(ReadResourceResult {
        contents: vec![ResourceContents::TextResourceContents {
            uri: uri.to_string(),
            mime_type: Some(RESOURCE_MIME_TYPE.to_string()),
            text,
            meta: None,
        }],
    })
}

/// Read the stats resource.
async fn read_stats_resource(
    uri: &str,
    store: &Arc<Mutex<LoroStore>>,
) -> Result<ReadResourceResult, McpError> {
    let store = store.lock().await;

    let decision_count = store.list_decisions().map_err(McpError::from)?.len();
    let task_count = store.list_tasks().map_err(McpError::from)?.len();
    let note_count = store.list_notes().map_err(McpError::from)?.len();
    let prompt_count = store.list_prompts().map_err(McpError::from)?.len();
    let component_count = store.list_components().map_err(McpError::from)?.len();
    let link_count = store.list_links().map_err(McpError::from)?.len();
    let relation_count = store.list_relations().map_err(McpError::from)?.len();

    let stats = serde_json::json!({
        "entity_counts": {
            "decision": decision_count,
            "task": task_count,
            "note": note_count,
            "prompt": prompt_count,
            "component": component_count,
            "link": link_count,
        },
        "relation_count": relation_count,
        "medulla_version": env!("CARGO_PKG_VERSION"),
    });

    let text = serde_json::to_string_pretty(&stats).map_err(|e| McpError::InternalError {
        message: format!("Failed to serialize stats: {}", e),
    })?;

    Ok(ReadResourceResult {
        contents: vec![ResourceContents::TextResourceContents {
            uri: uri.to_string(),
            mime_type: Some(RESOURCE_MIME_TYPE.to_string()),
            text,
            meta: None,
        }],
    })
}

/// Read all entities resource.
async fn read_all_entities_resource(
    uri: &str,
    store: &Arc<Mutex<LoroStore>>,
) -> Result<ReadResourceResult, McpError> {
    let store = store.lock().await;
    let mut entities: Vec<EntityResponse> = Vec::new();

    // Collect all entities
    for d in store.list_decisions().map_err(McpError::from)? {
        entities.push(decision_to_response(&d));
    }
    for t in store.list_tasks().map_err(McpError::from)? {
        entities.push(task_to_response(&t));
    }
    for n in store.list_notes().map_err(McpError::from)? {
        entities.push(note_to_response(&n));
    }
    for p in store.list_prompts().map_err(McpError::from)? {
        entities.push(prompt_to_response(&p));
    }
    for c in store.list_components().map_err(McpError::from)? {
        entities.push(component_to_response(&c));
    }
    for l in store.list_links().map_err(McpError::from)? {
        entities.push(link_to_response(&l));
    }

    let response = serde_json::json!({
        "entities": entities,
        "total": entities.len(),
    });

    let text = serde_json::to_string_pretty(&response).map_err(|e| McpError::InternalError {
        message: format!("Failed to serialize entities: {}", e),
    })?;

    Ok(ReadResourceResult {
        contents: vec![ResourceContents::TextResourceContents {
            uri: uri.to_string(),
            mime_type: Some(RESOURCE_MIME_TYPE.to_string()),
            text,
            meta: None,
        }],
    })
}

/// Read entities by type resource.
async fn read_entities_by_type_resource(
    uri: &str,
    store: &Arc<Mutex<LoroStore>>,
    entity_type: &str,
) -> Result<ReadResourceResult, McpError> {
    if !VALID_ENTITY_TYPES.contains(&entity_type) {
        return Err(McpError::InvalidResourceUri {
            uri: uri.to_string(),
        });
    }

    let store = store.lock().await;
    let entities: Vec<EntityResponse> = match entity_type {
        "decision" => store
            .list_decisions()
            .map_err(McpError::from)?
            .iter()
            .map(decision_to_response)
            .collect(),
        "task" => store
            .list_tasks()
            .map_err(McpError::from)?
            .iter()
            .map(task_to_response)
            .collect(),
        "note" => store
            .list_notes()
            .map_err(McpError::from)?
            .iter()
            .map(note_to_response)
            .collect(),
        "prompt" => store
            .list_prompts()
            .map_err(McpError::from)?
            .iter()
            .map(prompt_to_response)
            .collect(),
        "component" => store
            .list_components()
            .map_err(McpError::from)?
            .iter()
            .map(component_to_response)
            .collect(),
        "link" => store
            .list_links()
            .map_err(McpError::from)?
            .iter()
            .map(link_to_response)
            .collect(),
        _ => {
            return Err(McpError::InvalidResourceUri {
                uri: uri.to_string(),
            })
        }
    };

    let response = serde_json::json!({
        "entities": entities,
        "total": entities.len(),
    });

    let text = serde_json::to_string_pretty(&response).map_err(|e| McpError::InternalError {
        message: format!("Failed to serialize entities: {}", e),
    })?;

    Ok(ReadResourceResult {
        contents: vec![ResourceContents::TextResourceContents {
            uri: uri.to_string(),
            mime_type: Some(RESOURCE_MIME_TYPE.to_string()),
            text,
            meta: None,
        }],
    })
}

/// Read a single entity by ID.
async fn read_entity_by_id_resource(
    uri: &str,
    store: &Arc<Mutex<LoroStore>>,
    id: &str,
) -> Result<ReadResourceResult, McpError> {
    let store = store.lock().await;
    let is_sequence = id.chars().all(|c| c.is_ascii_digit());

    // Helper to match ID
    let matches_id = |base: &crate::entity::EntityBase| -> bool {
        if is_sequence {
            base.sequence_number.to_string() == id
        } else {
            let uuid_str = base.id.to_string().replace('-', "");
            let search_id = id.replace('-', "").to_lowercase();
            uuid_str.to_lowercase().starts_with(&search_id)
        }
    };

    // Search all entity types
    for d in store.list_decisions().map_err(McpError::from)? {
        if matches_id(&d.base) {
            let response = decision_to_response(&d);
            let text =
                serde_json::to_string_pretty(&response).map_err(|e| McpError::InternalError {
                    message: format!("Failed to serialize entity: {}", e),
                })?;
            return Ok(ReadResourceResult {
                contents: vec![ResourceContents::TextResourceContents {
                    uri: uri.to_string(),
                    mime_type: Some(RESOURCE_MIME_TYPE.to_string()),
                    text,
                    meta: None,
                }],
            });
        }
    }
    for t in store.list_tasks().map_err(McpError::from)? {
        if matches_id(&t.base) {
            let response = task_to_response(&t);
            let text =
                serde_json::to_string_pretty(&response).map_err(|e| McpError::InternalError {
                    message: format!("Failed to serialize entity: {}", e),
                })?;
            return Ok(ReadResourceResult {
                contents: vec![ResourceContents::TextResourceContents {
                    uri: uri.to_string(),
                    mime_type: Some(RESOURCE_MIME_TYPE.to_string()),
                    text,
                    meta: None,
                }],
            });
        }
    }
    for n in store.list_notes().map_err(McpError::from)? {
        if matches_id(&n.base) {
            let response = note_to_response(&n);
            let text =
                serde_json::to_string_pretty(&response).map_err(|e| McpError::InternalError {
                    message: format!("Failed to serialize entity: {}", e),
                })?;
            return Ok(ReadResourceResult {
                contents: vec![ResourceContents::TextResourceContents {
                    uri: uri.to_string(),
                    mime_type: Some(RESOURCE_MIME_TYPE.to_string()),
                    text,
                    meta: None,
                }],
            });
        }
    }
    for p in store.list_prompts().map_err(McpError::from)? {
        if matches_id(&p.base) {
            let response = prompt_to_response(&p);
            let text =
                serde_json::to_string_pretty(&response).map_err(|e| McpError::InternalError {
                    message: format!("Failed to serialize entity: {}", e),
                })?;
            return Ok(ReadResourceResult {
                contents: vec![ResourceContents::TextResourceContents {
                    uri: uri.to_string(),
                    mime_type: Some(RESOURCE_MIME_TYPE.to_string()),
                    text,
                    meta: None,
                }],
            });
        }
    }
    for c in store.list_components().map_err(McpError::from)? {
        if matches_id(&c.base) {
            let response = component_to_response(&c);
            let text =
                serde_json::to_string_pretty(&response).map_err(|e| McpError::InternalError {
                    message: format!("Failed to serialize entity: {}", e),
                })?;
            return Ok(ReadResourceResult {
                contents: vec![ResourceContents::TextResourceContents {
                    uri: uri.to_string(),
                    mime_type: Some(RESOURCE_MIME_TYPE.to_string()),
                    text,
                    meta: None,
                }],
            });
        }
    }
    for l in store.list_links().map_err(McpError::from)? {
        if matches_id(&l.base) {
            let response = link_to_response(&l);
            let text =
                serde_json::to_string_pretty(&response).map_err(|e| McpError::InternalError {
                    message: format!("Failed to serialize entity: {}", e),
                })?;
            return Ok(ReadResourceResult {
                contents: vec![ResourceContents::TextResourceContents {
                    uri: uri.to_string(),
                    mime_type: Some(RESOURCE_MIME_TYPE.to_string()),
                    text,
                    meta: None,
                }],
            });
        }
    }

    Err(McpError::ResourceNotFound {
        uri: uri.to_string(),
    })
}

/// Read decisions resource.
async fn read_decisions_resource(
    uri: &str,
    store: &Arc<Mutex<LoroStore>>,
    active_only: bool,
) -> Result<ReadResourceResult, McpError> {
    let store = store.lock().await;
    let decisions = store.list_decisions().map_err(McpError::from)?;

    let filtered: Vec<EntityResponse> = if active_only {
        decisions
            .iter()
            .filter(|d| {
                d.status != crate::entity::DecisionStatus::Superseded
                    && d.status != crate::entity::DecisionStatus::Deprecated
            })
            .map(decision_to_response)
            .collect()
    } else {
        decisions.iter().map(decision_to_response).collect()
    };

    let response = serde_json::json!({
        "decisions": filtered,
        "total": filtered.len(),
    });

    let text = serde_json::to_string_pretty(&response).map_err(|e| McpError::InternalError {
        message: format!("Failed to serialize decisions: {}", e),
    })?;

    Ok(ReadResourceResult {
        contents: vec![ResourceContents::TextResourceContents {
            uri: uri.to_string(),
            mime_type: Some(RESOURCE_MIME_TYPE.to_string()),
            text,
            meta: None,
        }],
    })
}

/// Read tasks resource.
async fn read_tasks_resource(
    uri: &str,
    store: &Arc<Mutex<LoroStore>>,
    filter: Option<&str>,
) -> Result<ReadResourceResult, McpError> {
    let store = store.lock().await;
    let tasks = store.list_tasks().map_err(McpError::from)?;

    let filtered: Vec<EntityResponse> = match filter {
        Some("active") => tasks
            .iter()
            .filter(|t| t.status != crate::entity::TaskStatus::Done)
            .map(task_to_response)
            .collect(),
        _ => tasks.iter().map(task_to_response).collect(),
    };

    let response = serde_json::json!({
        "tasks": filtered,
        "total": filtered.len(),
    });

    let text = serde_json::to_string_pretty(&response).map_err(|e| McpError::InternalError {
        message: format!("Failed to serialize tasks: {}", e),
    })?;

    Ok(ReadResourceResult {
        contents: vec![ResourceContents::TextResourceContents {
            uri: uri.to_string(),
            mime_type: Some(RESOURCE_MIME_TYPE.to_string()),
            text,
            meta: None,
        }],
    })
}

/// Read ready tasks resource (uses cache for blocker queries).
async fn read_ready_tasks_resource(
    uri: &str,
    cache: &Arc<Mutex<SqliteCache>>,
) -> Result<ReadResourceResult, McpError> {
    let cache = cache.lock().await;
    let ready_tasks = cache.get_ready_tasks(None).map_err(McpError::from)?;

    let tasks: Vec<serde_json::Value> = ready_tasks
        .iter()
        .map(|t| {
            serde_json::json!({
                "id": t.id,
                "sequence_number": t.sequence_number,
                "title": t.title,
                "status": t.status,
                "priority": t.priority,
                "due_date": t.due_date,
                "assignee": t.assignee,
            })
        })
        .collect();

    let response = serde_json::json!({
        "tasks": tasks,
        "total": tasks.len(),
    });

    let text = serde_json::to_string_pretty(&response).map_err(|e| McpError::InternalError {
        message: format!("Failed to serialize ready tasks: {}", e),
    })?;

    Ok(ReadResourceResult {
        contents: vec![ResourceContents::TextResourceContents {
            uri: uri.to_string(),
            mime_type: Some(RESOURCE_MIME_TYPE.to_string()),
            text,
            meta: None,
        }],
    })
}

/// Read blocked tasks resource (uses cache for blocker queries).
async fn read_blocked_tasks_resource(
    uri: &str,
    cache: &Arc<Mutex<SqliteCache>>,
) -> Result<ReadResourceResult, McpError> {
    let cache = cache.lock().await;
    let blocked_tasks = cache.get_blocked_tasks(None).map_err(McpError::from)?;

    let tasks: Vec<serde_json::Value> = blocked_tasks
        .iter()
        .map(|t| {
            let blockers: Vec<serde_json::Value> = t
                .blockers
                .iter()
                .map(|b| {
                    serde_json::json!({
                        "id": b.id,
                        "sequence_number": b.sequence_number,
                        "title": b.title,
                        "status": b.status,
                    })
                })
                .collect();

            serde_json::json!({
                "id": t.id,
                "sequence_number": t.sequence_number,
                "title": t.title,
                "status": t.status,
                "priority": t.priority,
                "due_date": t.due_date,
                "assignee": t.assignee,
                "blockers": blockers,
            })
        })
        .collect();

    let response = serde_json::json!({
        "blocked_tasks": tasks,
        "total": tasks.len(),
    });

    let text = serde_json::to_string_pretty(&response).map_err(|e| McpError::InternalError {
        message: format!("Failed to serialize blocked tasks: {}", e),
    })?;

    Ok(ReadResourceResult {
        contents: vec![ResourceContents::TextResourceContents {
            uri: uri.to_string(),
            mime_type: Some(RESOURCE_MIME_TYPE.to_string()),
            text,
            meta: None,
        }],
    })
}

/// Read tasks due on a specific date.
async fn read_tasks_due_resource(
    uri: &str,
    store: &Arc<Mutex<LoroStore>>,
    date_str: &str,
) -> Result<ReadResourceResult, McpError> {
    // Parse the date
    let target_date = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d").map_err(|_| {
        McpError::InvalidResourceUri {
            uri: uri.to_string(),
        }
    })?;

    let store = store.lock().await;
    let tasks = store.list_tasks().map_err(McpError::from)?;

    let filtered: Vec<EntityResponse> = tasks
        .iter()
        .filter(|t| t.due_date == Some(target_date))
        .map(task_to_response)
        .collect();

    let response = serde_json::json!({
        "tasks": filtered,
        "total": filtered.len(),
        "due_date": date_str,
    });

    let text = serde_json::to_string_pretty(&response).map_err(|e| McpError::InternalError {
        message: format!("Failed to serialize tasks: {}", e),
    })?;

    Ok(ReadResourceResult {
        contents: vec![ResourceContents::TextResourceContents {
            uri: uri.to_string(),
            mime_type: Some(RESOURCE_MIME_TYPE.to_string()),
            text,
            meta: None,
        }],
    })
}

/// Read prompts resource.
async fn read_prompts_resource(
    uri: &str,
    store: &Arc<Mutex<LoroStore>>,
) -> Result<ReadResourceResult, McpError> {
    let store = store.lock().await;
    let prompts = store.list_prompts().map_err(McpError::from)?;
    let entities: Vec<EntityResponse> = prompts.iter().map(prompt_to_response).collect();

    let response = serde_json::json!({
        "prompts": entities,
        "total": entities.len(),
    });

    let text = serde_json::to_string_pretty(&response).map_err(|e| McpError::InternalError {
        message: format!("Failed to serialize prompts: {}", e),
    })?;

    Ok(ReadResourceResult {
        contents: vec![ResourceContents::TextResourceContents {
            uri: uri.to_string(),
            mime_type: Some(RESOURCE_MIME_TYPE.to_string()),
            text,
            meta: None,
        }],
    })
}

/// Read full knowledge graph resource.
async fn read_graph_resource(
    uri: &str,
    store: &Arc<Mutex<LoroStore>>,
) -> Result<ReadResourceResult, McpError> {
    let store = store.lock().await;

    // Collect all entities
    let mut entities: Vec<EntityResponse> = Vec::new();
    for d in store.list_decisions().map_err(McpError::from)? {
        entities.push(decision_to_response(&d));
    }
    for t in store.list_tasks().map_err(McpError::from)? {
        entities.push(task_to_response(&t));
    }
    for n in store.list_notes().map_err(McpError::from)? {
        entities.push(note_to_response(&n));
    }
    for p in store.list_prompts().map_err(McpError::from)? {
        entities.push(prompt_to_response(&p));
    }
    for c in store.list_components().map_err(McpError::from)? {
        entities.push(component_to_response(&c));
    }
    for l in store.list_links().map_err(McpError::from)? {
        entities.push(link_to_response(&l));
    }

    // Collect all relations
    let relations = store.list_relations().map_err(McpError::from)?;
    let relation_responses: Vec<RelationResponse> =
        relations.iter().map(relation_to_response).collect();

    let response = serde_json::json!({
        "entities": entities,
        "relations": relation_responses,
        "entity_count": entities.len(),
        "relation_count": relation_responses.len(),
    });

    let text = serde_json::to_string_pretty(&response).map_err(|e| McpError::InternalError {
        message: format!("Failed to serialize graph: {}", e),
    })?;

    Ok(ReadResourceResult {
        contents: vec![ResourceContents::TextResourceContents {
            uri: uri.to_string(),
            mime_type: Some(RESOURCE_MIME_TYPE.to_string()),
            text,
            meta: None,
        }],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::SqliteCache;
    use crate::entity::{Decision, Task};
    use crate::storage::LoroStore;
    use tempfile::TempDir;

    async fn setup_test_env() -> (Arc<Mutex<LoroStore>>, Arc<Mutex<SqliteCache>>, TempDir) {
        let tmp = TempDir::new().unwrap();

        // Use init to create a new medulla project in the temp directory
        // This creates .medulla directory with loro.db
        let store = LoroStore::init(tmp.path()).unwrap();

        // SqliteCache::open expects the .medulla directory, it will create cache.db inside
        let medulla_dir = tmp.path().join(".medulla");
        let cache = SqliteCache::open(&medulla_dir).unwrap();

        (
            Arc::new(Mutex::new(store)),
            Arc::new(Mutex::new(cache)),
            tmp,
        )
    }

    #[test]
    fn test_build_static_resources() {
        let resources = build_static_resources();
        assert_eq!(resources.len(), 9);
        assert!(resources.iter().any(|r| r.uri == "medulla://schema"));
        assert!(resources.iter().any(|r| r.uri == "medulla://stats"));
        assert!(resources.iter().any(|r| r.uri == "medulla://entities"));
        assert!(resources.iter().any(|r| r.uri == "medulla://decisions"));
        assert!(resources.iter().any(|r| r.uri == "medulla://tasks"));
        assert!(resources.iter().any(|r| r.uri == "medulla://tasks/ready"));
        assert!(resources.iter().any(|r| r.uri == "medulla://tasks/blocked"));
        assert!(resources.iter().any(|r| r.uri == "medulla://prompts"));
        assert!(resources.iter().any(|r| r.uri == "medulla://graph"));
    }

    #[test]
    fn test_build_resource_templates() {
        let templates = build_resource_templates();
        assert_eq!(templates.len(), 5);
        assert!(templates
            .iter()
            .any(|t| t.uri_template == "medulla://entities/{type}"));
        assert!(templates
            .iter()
            .any(|t| t.uri_template == "medulla://entity/{id}"));
        assert!(templates
            .iter()
            .any(|t| t.uri_template == "medulla://decisions/active"));
        assert!(templates
            .iter()
            .any(|t| t.uri_template == "medulla://tasks/active"));
        assert!(templates
            .iter()
            .any(|t| t.uri_template == "medulla://tasks/due/{date}"));
    }

    #[tokio::test]
    async fn test_read_schema_resource() {
        let (store, cache, _tmp) = setup_test_env().await;
        let result = read_resource("medulla://schema", &store, &cache)
            .await
            .unwrap();

        assert_eq!(result.contents.len(), 1);
        if let ResourceContents::TextResourceContents { uri, text, .. } = &result.contents[0] {
            assert_eq!(uri, "medulla://schema");
            let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
            assert!(parsed["entity_types"].is_array());
        } else {
            panic!("Expected TextResourceContents");
        }
    }

    #[tokio::test]
    async fn test_read_stats_resource() {
        let (store, cache, _tmp) = setup_test_env().await;

        // Add some test data
        {
            let store = store.lock().await;
            let decision = Decision::new("Test Decision".to_string(), 1);
            store.add_decision(&decision).unwrap();
            store.save().unwrap();
        }

        let result = read_resource("medulla://stats", &store, &cache)
            .await
            .unwrap();

        assert_eq!(result.contents.len(), 1);
        if let ResourceContents::TextResourceContents { text, .. } = &result.contents[0] {
            let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
            assert_eq!(parsed["entity_counts"]["decision"], 1);
            assert!(parsed["medulla_version"].is_string());
        } else {
            panic!("Expected TextResourceContents");
        }
    }

    #[tokio::test]
    async fn test_read_entities_resource() {
        let (store, cache, _tmp) = setup_test_env().await;

        // Add test data
        {
            let store = store.lock().await;
            let decision = Decision::new("Test Decision".to_string(), 1);
            let task = Task::new("Test Task".to_string(), 1);
            store.add_decision(&decision).unwrap();
            store.add_task(&task).unwrap();
            store.save().unwrap();
        }

        let result = read_resource("medulla://entities", &store, &cache)
            .await
            .unwrap();

        if let ResourceContents::TextResourceContents { text, .. } = &result.contents[0] {
            let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
            assert_eq!(parsed["total"], 2);
        } else {
            panic!("Expected TextResourceContents");
        }
    }

    #[tokio::test]
    async fn test_read_entities_by_type_resource() {
        let (store, cache, _tmp) = setup_test_env().await;

        // Add test data
        {
            let store = store.lock().await;
            let decision = Decision::new("Test Decision".to_string(), 1);
            let task = Task::new("Test Task".to_string(), 1);
            store.add_decision(&decision).unwrap();
            store.add_task(&task).unwrap();
            store.save().unwrap();
        }

        let result = read_resource("medulla://entities/decision", &store, &cache)
            .await
            .unwrap();

        if let ResourceContents::TextResourceContents { text, .. } = &result.contents[0] {
            let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
            assert_eq!(parsed["total"], 1);
            assert_eq!(parsed["entities"][0]["type"], "decision");
        } else {
            panic!("Expected TextResourceContents");
        }
    }

    #[tokio::test]
    async fn test_read_entity_by_id_resource() {
        let (store, cache, _tmp) = setup_test_env().await;

        // Add test data
        {
            let store = store.lock().await;
            let decision = Decision::new("Test Decision".to_string(), 1);
            store.add_decision(&decision).unwrap();
            store.save().unwrap();
        }

        // Get by sequence number
        let result = read_resource("medulla://entity/1", &store, &cache)
            .await
            .unwrap();

        if let ResourceContents::TextResourceContents { text, .. } = &result.contents[0] {
            let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
            assert_eq!(parsed["title"], "Test Decision");
        } else {
            panic!("Expected TextResourceContents");
        }
    }

    #[tokio::test]
    async fn test_read_decisions_active_resource() {
        let (store, cache, _tmp) = setup_test_env().await;

        // Add test data
        {
            let store = store.lock().await;
            let decision1 = Decision::new("Active Decision".to_string(), 1);
            let mut decision2 = Decision::new("Superseded Decision".to_string(), 2);
            decision2.status = crate::entity::DecisionStatus::Superseded;
            store.add_decision(&decision1).unwrap();
            store.add_decision(&decision2).unwrap();
            store.save().unwrap();
        }

        let result = read_resource("medulla://decisions/active", &store, &cache)
            .await
            .unwrap();

        if let ResourceContents::TextResourceContents { text, .. } = &result.contents[0] {
            let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
            assert_eq!(parsed["total"], 1);
            assert_eq!(parsed["decisions"][0]["title"], "Active Decision");
        } else {
            panic!("Expected TextResourceContents");
        }
    }

    #[tokio::test]
    async fn test_read_tasks_active_resource() {
        let (store, cache, _tmp) = setup_test_env().await;

        // Add test data
        {
            let store = store.lock().await;
            let task1 = Task::new("Active Task".to_string(), 1);
            let mut task2 = Task::new("Done Task".to_string(), 2);
            task2.status = crate::entity::TaskStatus::Done;
            store.add_task(&task1).unwrap();
            store.add_task(&task2).unwrap();
            store.save().unwrap();
        }

        let result = read_resource("medulla://tasks/active", &store, &cache)
            .await
            .unwrap();

        if let ResourceContents::TextResourceContents { text, .. } = &result.contents[0] {
            let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
            assert_eq!(parsed["total"], 1);
            assert_eq!(parsed["tasks"][0]["title"], "Active Task");
        } else {
            panic!("Expected TextResourceContents");
        }
    }

    #[tokio::test]
    async fn test_read_tasks_due_resource() {
        let (store, cache, _tmp) = setup_test_env().await;

        // Add test data
        {
            let store = store.lock().await;
            let mut task1 = Task::new("Task Due Today".to_string(), 1);
            task1.due_date = Some(chrono::NaiveDate::from_ymd_opt(2025, 2, 1).unwrap());
            let mut task2 = Task::new("Task Due Tomorrow".to_string(), 2);
            task2.due_date = Some(chrono::NaiveDate::from_ymd_opt(2025, 2, 2).unwrap());
            store.add_task(&task1).unwrap();
            store.add_task(&task2).unwrap();
            store.save().unwrap();
        }

        let result = read_resource("medulla://tasks/due/2025-02-01", &store, &cache)
            .await
            .unwrap();

        if let ResourceContents::TextResourceContents { text, .. } = &result.contents[0] {
            let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
            assert_eq!(parsed["total"], 1);
            assert_eq!(parsed["tasks"][0]["title"], "Task Due Today");
        } else {
            panic!("Expected TextResourceContents");
        }
    }

    #[tokio::test]
    async fn test_read_graph_resource() {
        let (store, cache, _tmp) = setup_test_env().await;

        // Add test data
        {
            let store = store.lock().await;
            let decision = Decision::new("Test Decision".to_string(), 1);
            let task = Task::new("Test Task".to_string(), 1);
            store.add_decision(&decision).unwrap();
            store.add_task(&task).unwrap();
            store.save().unwrap();
        }

        let result = read_resource("medulla://graph", &store, &cache)
            .await
            .unwrap();

        if let ResourceContents::TextResourceContents { text, .. } = &result.contents[0] {
            let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
            assert_eq!(parsed["entity_count"], 2);
            assert!(parsed["relations"].is_array());
        } else {
            panic!("Expected TextResourceContents");
        }
    }

    #[tokio::test]
    async fn test_invalid_resource_uri() {
        let (store, cache, _tmp) = setup_test_env().await;

        // Invalid scheme
        let result = read_resource("invalid://schema", &store, &cache).await;
        assert!(result.is_err());

        // Invalid path
        let result = read_resource("medulla://nonexistent", &store, &cache).await;
        assert!(result.is_err());

        // Invalid entity type
        let result = read_resource("medulla://entities/invalid_type", &store, &cache).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_entity_not_found_resource() {
        let (store, cache, _tmp) = setup_test_env().await;

        let result = read_resource("medulla://entity/999", &store, &cache).await;
        assert!(result.is_err());
    }
}
