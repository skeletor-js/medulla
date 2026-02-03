//! MCP (Model Context Protocol) server implementation for Medulla.
//!
//! This module provides an MCP server that exposes Medulla's knowledge engine
//! to AI tools like Claude Desktop, Cursor, and Copilot.

pub mod error;
pub mod resources;
pub mod tools;

use crate::cache::SqliteCache;
use crate::entity::{
    Component, Decision, EntityBase, Link, Note, Prompt, Task,
};
use crate::storage::{
    ComponentUpdate, DecisionUpdate, LinkUpdate, LoroStore, NoteUpdate, PromptUpdate, TaskUpdate,
};
use error::{validation, McpError, VALID_ENTITY_TYPES};
use rmcp::{
    handler::server::wrapper::Parameters,
    model::*,
    service::{RequestContext, RoleServer},
    tool, tool_handler, tool_router, ErrorData as McpErrorData, ServerHandler,
};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use tokio::sync::Mutex;
use tools::*;

/// Subscription identifier type.
pub type SubscriptionId = String;

/// Manages active resource subscriptions.
#[derive(Debug, Default)]
pub struct SubscriptionState {
    /// Map of resource URI to list of subscription IDs.
    pub by_resource: HashMap<String, Vec<SubscriptionId>>,
    /// Counter for generating unique subscription IDs.
    next_id: u64,
}

impl SubscriptionState {
    /// Create a new empty subscription state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a subscription for a resource URI.
    pub fn subscribe(&mut self, uri: &str) -> SubscriptionId {
        let id = format!("sub_{}", self.next_id);
        self.next_id += 1;
        self.by_resource
            .entry(uri.to_string())
            .or_default()
            .push(id.clone());
        id
    }

    /// Remove a subscription by ID.
    pub fn unsubscribe(&mut self, id: &str) -> bool {
        for subs in self.by_resource.values_mut() {
            if let Some(pos) = subs.iter().position(|s| s == id) {
                subs.remove(pos);
                return true;
            }
        }
        false
    }

    /// Get all subscription IDs for a resource URI.
    pub fn get_subscribers(&self, uri: &str) -> Vec<SubscriptionId> {
        self.by_resource.get(uri).cloned().unwrap_or_default()
    }

    /// Clear all subscriptions (for disconnect cleanup).
    pub fn clear(&mut self) {
        self.by_resource.clear();
    }
}

/// The main MCP server for Medulla.
///
/// Holds thread-safe references to the storage layer and manages
/// resource subscriptions.
#[derive(Clone)]
pub struct MedullaServer {
    /// The Loro CRDT store for entities.
    pub store: Arc<Mutex<LoroStore>>,
    /// The SQLite cache for full-text search (wrapped in Mutex for thread safety).
    pub cache: Arc<Mutex<SqliteCache>>,
    /// Active resource subscriptions.
    pub subscriptions: Arc<Mutex<SubscriptionState>>,
    /// Tool router for MCP tool handling.
    pub tool_router: rmcp::handler::server::tool::ToolRouter<Self>,
}

/// Server information for MCP initialization.
#[allow(dead_code)]
const SERVER_NAME: &str = "medulla";
#[allow(dead_code)]
const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

// All tool implementations in the tool_router impl block
#[tool_router]
impl MedullaServer {
    /// Create a new MedullaServer instance.
    pub fn new(store: LoroStore, cache: SqliteCache) -> Self {
        Self {
            store: Arc::new(Mutex::new(store)),
            cache: Arc::new(Mutex::new(cache)),
            subscriptions: Arc::new(Mutex::new(SubscriptionState::new())),
            tool_router: Self::tool_router(),
        }
    }

    /// Start the MCP server on the given transport.
    ///
    /// This method runs the server until the transport is closed or an error occurs.
    pub async fn serve<T, E, A>(self, transport: T) -> Result<(), Box<dyn std::error::Error + Send + Sync>>
    where
        T: rmcp::transport::IntoTransport<RoleServer, E, A>,
        E: std::error::Error + Send + Sync + 'static,
    {
        use rmcp::service::ServiceExt;
        let running = ServiceExt::serve(self, transport).await.map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })?;
        running.waiting().await.map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })?;
        Ok(())
    }

    /// Ping tool for health checks.
    #[tool(description = "Check if the server is running")]
    async fn ping(&self) -> Result<CallToolResult, McpErrorData> {
        Ok(CallToolResult::success(vec![Content::text("pong")]))
    }

    // ========================================================================
    // entity_create
    // ========================================================================

    /// Create a new entity of any type.
    #[tool(description = "Create a new entity (decision, task, note, prompt, component, or link)")]
    pub async fn entity_create(
        &self,
        Parameters(params): Parameters<EntityCreateParams>,
    ) -> Result<CallToolResult, McpErrorData> {
        // Validate common fields
        validate_entity_type(&params.entity_type)?;
        validate_title(&params.title)?;
        validate_content(&params.content)?;
        validate_tags(&params.tags)?;

        let store = self.store.lock().await;
        let cache = self.cache.lock().await;

        let response = match params.entity_type.as_str() {
            "decision" => {
                let seq = store.next_sequence_number();
                let mut decision = Decision::new(params.title.trim().to_string(), seq);
                decision.base.content = params.content;
                decision.base.tags = params.tags.unwrap_or_default();

                // Parse decision-specific properties
                if let Some(props) = params.properties {
                    if let Some(status) = props.get("status").and_then(|v| v.as_str()) {
                        decision.status = parse_decision_status(status)?;
                    }
                    if let Some(context) = props.get("context").and_then(|v| v.as_str()) {
                        if context.len() > validation::MAX_CONTEXT_SIZE {
                            return Err(McpError::ValidationFailed {
                                field: "context".to_string(),
                                message: format!(
                                    "Context exceeds maximum size of {}",
                                    validation::MAX_CONTEXT_SIZE
                                ),
                            }
                            .into());
                        }
                        decision.context = Some(context.to_string());
                    }
                    if let Some(consequences) = props.get("consequences").and_then(|v| v.as_array())
                    {
                        decision.consequences = consequences
                            .iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect();
                    }
                    if let Some(superseded_by) =
                        props.get("superseded_by").and_then(|v| v.as_str())
                    {
                        decision.superseded_by = Some(superseded_by.to_string());
                    }
                }

                store
                    .add_decision(&decision)
                    .map_err(|e| McpError::from(e))?;
                store.save().map_err(|e| McpError::from(e))?;
                cache
                    .index_decision(&decision)
                    .map_err(|e| McpError::from(e))?;

                decision_to_response(&decision)
            }
            "task" => {
                let seq = store.next_sequence_number();
                let mut task = Task::new(params.title.trim().to_string(), seq);
                task.base.content = params.content;
                task.base.tags = params.tags.unwrap_or_default();

                if let Some(props) = params.properties {
                    if let Some(status) = props.get("status").and_then(|v| v.as_str()) {
                        task.status = parse_task_status(status)?;
                    }
                    if let Some(priority) = props.get("priority").and_then(|v| v.as_str()) {
                        task.priority = parse_task_priority(priority)?;
                    }
                    if let Some(due_date) = props.get("due_date").and_then(|v| v.as_str()) {
                        task.due_date = Some(parse_date("due_date", due_date)?);
                    }
                    if let Some(assignee) = props.get("assignee").and_then(|v| v.as_str()) {
                        task.assignee = Some(assignee.to_string());
                    }
                }

                store.add_task(&task).map_err(|e| McpError::from(e))?;
                store.save().map_err(|e| McpError::from(e))?;
                cache.index_task(&task).map_err(|e| McpError::from(e))?;

                task_to_response(&task)
            }
            "note" => {
                let seq = store.next_sequence_number();
                let mut note = Note::new(params.title.trim().to_string(), seq);
                note.base.content = params.content;
                note.base.tags = params.tags.unwrap_or_default();

                if let Some(props) = params.properties {
                    if let Some(note_type) = props.get("note_type").and_then(|v| v.as_str()) {
                        note.note_type = Some(note_type.to_string());
                    }
                }

                store.add_note(&note).map_err(|e| McpError::from(e))?;
                store.save().map_err(|e| McpError::from(e))?;
                cache.index_note(&note).map_err(|e| McpError::from(e))?;

                note_to_response(&note)
            }
            "prompt" => {
                let seq = store.next_sequence_number();
                let mut prompt = Prompt::new(params.title.trim().to_string(), seq);
                prompt.base.content = params.content;
                prompt.base.tags = params.tags.unwrap_or_default();

                if let Some(props) = params.properties {
                    if let Some(template) = props.get("template").and_then(|v| v.as_str()) {
                        if template.len() > validation::MAX_TEMPLATE_SIZE {
                            return Err(McpError::ValidationFailed {
                                field: "template".to_string(),
                                message: format!(
                                    "Template exceeds maximum size of {}",
                                    validation::MAX_TEMPLATE_SIZE
                                ),
                            }
                            .into());
                        }
                        prompt.template = Some(template.to_string());
                    }
                    if let Some(variables) = props.get("variables").and_then(|v| v.as_array()) {
                        prompt.variables = variables
                            .iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect();
                    }
                    if let Some(output_schema) =
                        props.get("output_schema").and_then(|v| v.as_str())
                    {
                        if output_schema.len() > validation::MAX_OUTPUT_SCHEMA_SIZE {
                            return Err(McpError::ValidationFailed {
                                field: "output_schema".to_string(),
                                message: format!(
                                    "Output schema exceeds maximum size of {}",
                                    validation::MAX_OUTPUT_SCHEMA_SIZE
                                ),
                            }
                            .into());
                        }
                        prompt.output_schema = Some(output_schema.to_string());
                    }
                }

                store.add_prompt(&prompt).map_err(|e| McpError::from(e))?;
                store.save().map_err(|e| McpError::from(e))?;
                cache.index_prompt(&prompt).map_err(|e| McpError::from(e))?;

                prompt_to_response(&prompt)
            }
            "component" => {
                let seq = store.next_sequence_number();
                let mut component = Component::new(params.title.trim().to_string(), seq);
                component.base.content = params.content;
                component.base.tags = params.tags.unwrap_or_default();

                if let Some(props) = params.properties {
                    if let Some(component_type) =
                        props.get("component_type").and_then(|v| v.as_str())
                    {
                        component.component_type = Some(component_type.to_string());
                    }
                    if let Some(status) = props.get("status").and_then(|v| v.as_str()) {
                        component.status = parse_component_status(status)?;
                    }
                    if let Some(owner) = props.get("owner").and_then(|v| v.as_str()) {
                        component.owner = Some(owner.to_string());
                    }
                }

                store
                    .add_component(&component)
                    .map_err(|e| McpError::from(e))?;
                store.save().map_err(|e| McpError::from(e))?;
                cache
                    .index_component(&component)
                    .map_err(|e| McpError::from(e))?;

                component_to_response(&component)
            }
            "link" => {
                // URL is required for links
                let url = params
                    .properties
                    .as_ref()
                    .and_then(|p| p.get("url"))
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| McpError::ValidationFailed {
                        field: "url".to_string(),
                        message: "URL is required for link entities".to_string(),
                    })?;

                validate_url(url)?;

                let seq = store.next_sequence_number();
                let mut link = Link::new(params.title.trim().to_string(), url.to_string(), seq);
                link.base.content = params.content;
                link.base.tags = params.tags.unwrap_or_default();

                if let Some(props) = params.properties {
                    if let Some(link_type) = props.get("link_type").and_then(|v| v.as_str()) {
                        link.link_type = Some(link_type.to_string());
                    }
                }

                store.add_link(&link).map_err(|e| McpError::from(e))?;
                store.save().map_err(|e| McpError::from(e))?;
                cache.index_link(&link).map_err(|e| McpError::from(e))?;

                link_to_response(&link)
            }
            _ => unreachable!(), // Already validated
        };

        let json = serde_json::to_string_pretty(&response)
            .map_err(|e| McpError::InternalError {
                message: format!("Failed to serialize response: {}", e),
            })?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    // ========================================================================
    // entity_get
    // ========================================================================

    /// Get an entity by ID (sequence number or UUID prefix).
    #[tool(description = "Get an entity by ID (sequence number like '1' or UUID prefix like 'abc123')")]
    pub async fn entity_get(
        &self,
        Parameters(params): Parameters<EntityGetParams>,
    ) -> Result<CallToolResult, McpErrorData> {
        let store = self.store.lock().await;

        // Determine if ID is a sequence number or UUID prefix
        let is_sequence = params.id.chars().all(|c| c.is_ascii_digit());

        // If we have a type hint, search only that type
        if let Some(ref entity_type) = params.entity_type {
            validate_entity_type(entity_type)?;

            let response = self
                .find_entity_by_id(&store, entity_type, &params.id, is_sequence)?;

            if let Some(resp) = response {
                let json = serde_json::to_string_pretty(&resp).map_err(|e| {
                    McpError::InternalError {
                        message: format!("Failed to serialize response: {}", e),
                    }
                })?;
                return Ok(CallToolResult::success(vec![Content::text(json)]));
            }

            return Err(McpError::EntityNotFound {
                id: params.id.clone(),
            }
            .into());
        }

        // Search all entity types
        for entity_type in VALID_ENTITY_TYPES {
            if let Some(response) = self
                .find_entity_by_id(&store, entity_type, &params.id, is_sequence)?
            {
                let json = serde_json::to_string_pretty(&response).map_err(|e| {
                    McpError::InternalError {
                        message: format!("Failed to serialize response: {}", e),
                    }
                })?;
                return Ok(CallToolResult::success(vec![Content::text(json)]));
            }
        }

        Err(McpError::EntityNotFound {
            id: params.id.clone(),
        }
        .into())
    }

    // ========================================================================
    // entity_list
    // ========================================================================

    /// List entities with optional filters.
    #[tool(
        description = "List entities with optional filters by type, status, tag, with pagination"
    )]
    pub async fn entity_list(
        &self,
        Parameters(params): Parameters<EntityListParams>,
    ) -> Result<CallToolResult, McpErrorData> {
        let store = self.store.lock().await;

        let limit = params
            .limit
            .unwrap_or(validation::DEFAULT_LIMIT as u32)
            .min(validation::MAX_LIMIT as u32) as usize;
        let offset = params.offset.unwrap_or(0) as usize;

        let mut all_entities: Vec<EntityResponse> = Vec::new();

        // Determine which types to fetch
        let types_to_fetch: Vec<&str> = if let Some(ref t) = params.entity_type {
            validate_entity_type(t)?;
            vec![t.as_str()]
        } else {
            VALID_ENTITY_TYPES.to_vec()
        };

        for entity_type in types_to_fetch {
            match entity_type {
                "decision" => {
                    let decisions = store.list_decisions().map_err(McpError::from)?;
                    for d in decisions {
                        if self.matches_filters(&d.base, &params, Some(&d.status.to_string())) {
                            all_entities.push(decision_to_response(&d));
                        }
                    }
                }
                "task" => {
                    let tasks = store.list_tasks().map_err(McpError::from)?;
                    for t in tasks {
                        if self.matches_filters(&t.base, &params, Some(&t.status.to_string())) {
                            all_entities.push(task_to_response(&t));
                        }
                    }
                }
                "note" => {
                    let notes = store.list_notes().map_err(McpError::from)?;
                    for n in notes {
                        if self.matches_filters(&n.base, &params, None) {
                            all_entities.push(note_to_response(&n));
                        }
                    }
                }
                "prompt" => {
                    let prompts = store.list_prompts().map_err(McpError::from)?;
                    for p in prompts {
                        if self.matches_filters(&p.base, &params, None) {
                            all_entities.push(prompt_to_response(&p));
                        }
                    }
                }
                "component" => {
                    let components = store.list_components().map_err(McpError::from)?;
                    for c in components {
                        if self.matches_filters(&c.base, &params, Some(&c.status.to_string())) {
                            all_entities.push(component_to_response(&c));
                        }
                    }
                }
                "link" => {
                    let links = store.list_links().map_err(McpError::from)?;
                    for l in links {
                        if self.matches_filters(&l.base, &params, None) {
                            all_entities.push(link_to_response(&l));
                        }
                    }
                }
                _ => {}
            }
        }

        let total = all_entities.len();

        // Apply pagination
        let paginated: Vec<EntityResponse> =
            all_entities.into_iter().skip(offset).take(limit).collect();

        let response = serde_json::json!({
            "entities": paginated,
            "total": total,
            "limit": limit,
            "offset": offset,
        });

        let json =
            serde_json::to_string_pretty(&response).map_err(|e| McpError::InternalError {
                message: format!("Failed to serialize response: {}", e),
            })?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    // ========================================================================
    // entity_update
    // ========================================================================

    /// Update an existing entity.
    #[tool(description = "Update an existing entity's title, content, tags, or properties")]
    pub async fn entity_update(
        &self,
        Parameters(params): Parameters<EntityUpdateParams>,
    ) -> Result<CallToolResult, McpErrorData> {
        // Validate title if provided
        if let Some(ref title) = params.title {
            validate_title(title)?;
        }
        validate_content(&params.content)?;
        validate_tags(&params.add_tags)?;

        let store = self.store.lock().await;
        let cache = self.cache.lock().await;

        // Find the entity by ID
        let is_sequence = params.id.chars().all(|c| c.is_ascii_digit());

        for entity_type in VALID_ENTITY_TYPES {
            let response =
                self.try_update_entity(&store, &cache, entity_type, &params, is_sequence)?;
            if let Some(resp) = response {
                let json = serde_json::to_string_pretty(&resp).map_err(|e| {
                    McpError::InternalError {
                        message: format!("Failed to serialize response: {}", e),
                    }
                })?;
                return Ok(CallToolResult::success(vec![Content::text(json)]));
            }
        }

        Err(McpError::EntityNotFound {
            id: params.id.clone(),
        }
        .into())
    }

    // ========================================================================
    // entity_delete
    // ========================================================================

    /// Delete an entity by ID.
    #[tool(description = "Delete an entity by ID")]
    pub async fn entity_delete(
        &self,
        Parameters(params): Parameters<EntityDeleteParams>,
    ) -> Result<CallToolResult, McpErrorData> {
        let store = self.store.lock().await;
        let cache = self.cache.lock().await;

        let is_sequence = params.id.chars().all(|c| c.is_ascii_digit());

        for entity_type in VALID_ENTITY_TYPES {
            let deleted = self.try_delete_entity(&store, &cache, entity_type, &params.id, is_sequence)?;
            if deleted {
                return Ok(CallToolResult::success(vec![Content::text(format!(
                    "Entity {} deleted successfully",
                    params.id
                ))]));
            }
        }

        Err(McpError::EntityNotFound {
            id: params.id.clone(),
        }
        .into())
    }

    // ========================================================================
    // entity_batch
    // ========================================================================

    /// Execute multiple entity operations in a batch (best-effort semantics).
    #[tool(
        description = "Execute multiple entity operations in a batch. Operations run sequentially with best-effort semantics."
    )]
    pub async fn entity_batch(
        &self,
        Parameters(params): Parameters<EntityBatchParams>,
    ) -> Result<CallToolResult, McpErrorData> {
        if params.operations.len() > validation::MAX_BATCH_SIZE {
            return Err(McpError::ValidationFailed {
                field: "operations".to_string(),
                message: format!(
                    "Maximum {} operations allowed per batch",
                    validation::MAX_BATCH_SIZE
                ),
            }
            .into());
        }

        let mut results = Vec::new();
        let mut succeeded = 0;
        let mut failed = 0;

        for (index, op) in params.operations.into_iter().enumerate() {
            let result = match op {
                BatchOperation::Create(create_params) => {
                    match self.entity_create(Parameters(create_params)).await {
                        Ok(tool_result) => {
                            // Extract ID from the result
                            let id = tool_result
                                .content
                                .first()
                                .and_then(|c| {
                                    if let RawContent::Text(ref t) = c.raw {
                                        serde_json::from_str::<EntityResponse>(&t.text)
                                            .ok()
                                            .map(|r| r.id)
                                    } else {
                                        None
                                    }
                                });
                            succeeded += 1;
                            BatchOperationResult {
                                index,
                                success: true,
                                id,
                                error: None,
                            }
                        }
                        Err(e) => {
                            failed += 1;
                            BatchOperationResult {
                                index,
                                success: false,
                                id: None,
                                error: Some(BatchError {
                                    code: "CREATE_FAILED".to_string(),
                                    message: e.message.to_string(),
                                }),
                            }
                        }
                    }
                }
                BatchOperation::Update(update_params) => {
                    let id = update_params.id.clone();
                    match self.entity_update(Parameters(update_params)).await {
                        Ok(_) => {
                            succeeded += 1;
                            BatchOperationResult {
                                index,
                                success: true,
                                id: Some(id),
                                error: None,
                            }
                        }
                        Err(e) => {
                            failed += 1;
                            BatchOperationResult {
                                index,
                                success: false,
                                id: None,
                                error: Some(BatchError {
                                    code: "UPDATE_FAILED".to_string(),
                                    message: e.message.to_string(),
                                }),
                            }
                        }
                    }
                }
                BatchOperation::Delete(delete_params) => {
                    let id = delete_params.id.clone();
                    match self.entity_delete(Parameters(delete_params)).await {
                        Ok(_) => {
                            succeeded += 1;
                            BatchOperationResult {
                                index,
                                success: true,
                                id: Some(id),
                                error: None,
                            }
                        }
                        Err(e) => {
                            failed += 1;
                            BatchOperationResult {
                                index,
                                success: false,
                                id: None,
                                error: Some(BatchError {
                                    code: "DELETE_FAILED".to_string(),
                                    message: e.message.to_string(),
                                }),
                            }
                        }
                    }
                }
            };
            results.push(result);
        }

        let batch_result = BatchResult {
            results,
            succeeded,
            failed,
        };

        let json =
            serde_json::to_string_pretty(&batch_result).map_err(|e| McpError::InternalError {
                message: format!("Failed to serialize batch result: {}", e),
            })?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    // ========================================================================
    // search_fulltext
    // ========================================================================

    /// Full-text search across entities.
    #[tool(description = "Full-text search across entities via SQLite FTS5")]
    pub async fn search_fulltext(
        &self,
        Parameters(params): Parameters<SearchFulltextParams>,
    ) -> Result<CallToolResult, McpErrorData> {
        if params.query.trim().is_empty() {
            return Err(McpError::ValidationFailed {
                field: "query".to_string(),
                message: "Search query cannot be empty".to_string(),
            }
            .into());
        }

        if let Some(ref entity_type) = params.entity_type {
            validate_entity_type(entity_type)?;
        }

        let cache = self.cache.lock().await;
        let limit = params
            .limit
            .unwrap_or(validation::DEFAULT_LIMIT as u32)
            .min(validation::MAX_LIMIT as u32) as i64;

        let mut results: Vec<serde_json::Value> = Vec::new();

        // Determine which types to search
        let types_to_search: Vec<&str> = if let Some(ref t) = params.entity_type {
            vec![t.as_str()]
        } else {
            VALID_ENTITY_TYPES.to_vec()
        };

        for entity_type in types_to_search {
            match entity_type {
                "decision" => {
                    if let Ok(search_results) = cache.search_decisions(&params.query, limit) {
                        for r in search_results {
                            results.push(serde_json::json!({
                                "type": "decision",
                                "id": r.id,
                                "sequence_number": r.sequence_number,
                                "title": r.title,
                                "status": r.status,
                                "title_highlight": r.title_highlight,
                                "content_snippet": r.content_snippet,
                            }));
                        }
                    }
                }
                "task" => {
                    if let Ok(search_results) = cache.search_tasks(&params.query, limit) {
                        for r in search_results {
                            results.push(serde_json::json!({
                                "type": "task",
                                "id": r.id,
                                "sequence_number": r.sequence_number,
                                "title": r.title,
                                "status": r.status,
                                "priority": r.priority,
                                "title_highlight": r.title_highlight,
                                "content_snippet": r.content_snippet,
                            }));
                        }
                    }
                }
                "note" => {
                    if let Ok(search_results) = cache.search_notes(&params.query, limit) {
                        for r in search_results {
                            results.push(serde_json::json!({
                                "type": "note",
                                "id": r.id,
                                "sequence_number": r.sequence_number,
                                "title": r.title,
                                "note_type": r.note_type,
                                "title_highlight": r.title_highlight,
                                "content_snippet": r.content_snippet,
                            }));
                        }
                    }
                }
                "prompt" => {
                    if let Ok(search_results) = cache.search_prompts(&params.query, limit) {
                        for r in search_results {
                            results.push(serde_json::json!({
                                "type": "prompt",
                                "id": r.id,
                                "sequence_number": r.sequence_number,
                                "title": r.title,
                                "variables": r.variables,
                                "title_highlight": r.title_highlight,
                                "content_snippet": r.content_snippet,
                            }));
                        }
                    }
                }
                "component" => {
                    if let Ok(search_results) = cache.search_components(&params.query, limit) {
                        for r in search_results {
                            results.push(serde_json::json!({
                                "type": "component",
                                "id": r.id,
                                "sequence_number": r.sequence_number,
                                "title": r.title,
                                "status": r.status,
                                "component_type": r.component_type,
                                "title_highlight": r.title_highlight,
                                "content_snippet": r.content_snippet,
                            }));
                        }
                    }
                }
                "link" => {
                    if let Ok(search_results) = cache.search_links(&params.query, limit) {
                        for r in search_results {
                            results.push(serde_json::json!({
                                "type": "link",
                                "id": r.id,
                                "sequence_number": r.sequence_number,
                                "title": r.title,
                                "url": r.url,
                                "link_type": r.link_type,
                                "title_highlight": r.title_highlight,
                                "content_snippet": r.content_snippet,
                            }));
                        }
                    }
                }
                _ => {}
            }
        }

        // Truncate to limit
        results.truncate(limit as usize);

        let response = serde_json::json!({
            "results": results,
            "total": results.len(),
            "query": params.query,
        });

        let json =
            serde_json::to_string_pretty(&response).map_err(|e| McpError::InternalError {
                message: format!("Failed to serialize search results: {}", e),
            })?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    // ========================================================================
    // graph_relations
    // ========================================================================

    /// Get relations for an entity.
    #[tool(description = "Get relations for an entity (outgoing, incoming, or both)")]
    pub async fn graph_relations(
        &self,
        Parameters(params): Parameters<GraphRelationsParams>,
    ) -> Result<CallToolResult, McpErrorData> {
        let direction = params.direction.as_deref().unwrap_or("both");
        if !["from", "to", "both"].contains(&direction) {
            return Err(McpError::ValidationFailed {
                field: "direction".to_string(),
                message: "Direction must be 'from', 'to', or 'both'".to_string(),
            }
            .into());
        }

        let store = self.store.lock().await;

        // Resolve the ID to a UUID
        let uuid = self.resolve_entity_id(&store, &params.id)?;
        let uuid_str = uuid.to_string();

        let mut outgoing: Vec<RelationResponse> = Vec::new();
        let mut incoming: Vec<RelationResponse> = Vec::new();

        if direction == "from" || direction == "both" {
            let relations = store.get_relations_from(&uuid_str).map_err(McpError::from)?;
            outgoing = relations.iter().map(relation_to_response).collect();
        }

        if direction == "to" || direction == "both" {
            let relations = store.get_relations_to(&uuid_str).map_err(McpError::from)?;
            incoming = relations.iter().map(relation_to_response).collect();
        }

        let response = serde_json::json!({
            "entity_id": uuid_str,
            "outgoing": outgoing,
            "incoming": incoming,
        });

        let json =
            serde_json::to_string_pretty(&response).map_err(|e| McpError::InternalError {
                message: format!("Failed to serialize relations: {}", e),
            })?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    // ========================================================================
    // graph_path
    // ========================================================================

    /// Find shortest path between two entities.
    #[tool(description = "Find the shortest path between two entities using BFS traversal")]
    pub async fn graph_path(
        &self,
        Parameters(params): Parameters<GraphPathParams>,
    ) -> Result<CallToolResult, McpErrorData> {
        let max_depth = params
            .max_depth
            .unwrap_or(validation::DEFAULT_MAX_DEPTH as u32) as usize;

        let store = self.store.lock().await;

        // Resolve both IDs
        let from_uuid = self.resolve_entity_id(&store, &params.from_id)?;
        let to_uuid = self.resolve_entity_id(&store, &params.to_id)?;

        // Same entity
        if from_uuid == to_uuid {
            let response = serde_json::json!({
                "path": [from_uuid.to_string()],
                "length": 0,
            });
            let json = serde_json::to_string_pretty(&response).map_err(|e| {
                McpError::InternalError {
                    message: format!("Failed to serialize path: {}", e),
                }
            })?;
            return Ok(CallToolResult::success(vec![Content::text(json)]));
        }

        // BFS traversal
        let relations = store.list_relations().map_err(McpError::from)?;

        // Build adjacency list (bidirectional for path finding)
        let mut adjacency: HashMap<String, Vec<String>> = HashMap::new();
        for r in &relations {
            let source = r.source_id.to_string();
            let target = r.target_id.to_string();
            adjacency.entry(source.clone()).or_default().push(target.clone());
            adjacency.entry(target).or_default().push(source);
        }

        let from_str = from_uuid.to_string();
        let to_str = to_uuid.to_string();

        // BFS
        let mut visited: HashSet<String> = HashSet::new();
        let mut queue: VecDeque<(String, Vec<String>)> = VecDeque::new();

        visited.insert(from_str.clone());
        queue.push_back((from_str.clone(), vec![from_str.clone()]));

        while let Some((current, path)) = queue.pop_front() {
            if path.len() > max_depth + 1 {
                break;
            }

            if current == to_str {
                let response = serde_json::json!({
                    "path": path,
                    "length": path.len() - 1,
                });
                let json = serde_json::to_string_pretty(&response).map_err(|e| {
                    McpError::InternalError {
                        message: format!("Failed to serialize path: {}", e),
                    }
                })?;
                return Ok(CallToolResult::success(vec![Content::text(json)]));
            }

            if let Some(neighbors) = adjacency.get(&current) {
                for neighbor in neighbors {
                    if !visited.contains(neighbor) {
                        visited.insert(neighbor.clone());
                        let mut new_path = path.clone();
                        new_path.push(neighbor.clone());
                        queue.push_back((neighbor.clone(), new_path));
                    }
                }
            }
        }

        // No path found
        let response = serde_json::json!({
            "path": [],
            "length": null,
            "message": "No path found between entities",
        });
        let json =
            serde_json::to_string_pretty(&response).map_err(|e| McpError::InternalError {
                message: format!("Failed to serialize path: {}", e),
            })?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    // ========================================================================
    // graph_orphans
    // ========================================================================

    /// Find entities with no relations.
    #[tool(description = "Find entities with no incoming or outgoing relations")]
    pub async fn graph_orphans(
        &self,
        Parameters(params): Parameters<GraphOrphansParams>,
    ) -> Result<CallToolResult, McpErrorData> {
        if let Some(ref entity_type) = params.entity_type {
            validate_entity_type(entity_type)?;
        }

        let store = self.store.lock().await;
        let limit = params
            .limit
            .unwrap_or(validation::DEFAULT_LIMIT as u32)
            .min(validation::MAX_LIMIT as u32) as usize;

        // Get all entity IDs that have relations
        let relations = store.list_relations().map_err(McpError::from)?;
        let mut connected_ids: HashSet<String> = HashSet::new();
        for r in &relations {
            connected_ids.insert(r.source_id.to_string());
            connected_ids.insert(r.target_id.to_string());
        }

        let mut orphans: Vec<EntityResponse> = Vec::new();

        // Determine which types to check
        let types_to_check: Vec<&str> = if let Some(ref t) = params.entity_type {
            vec![t.as_str()]
        } else {
            VALID_ENTITY_TYPES.to_vec()
        };

        for entity_type in types_to_check {
            if orphans.len() >= limit {
                break;
            }

            match entity_type {
                "decision" => {
                    let decisions = store.list_decisions().map_err(McpError::from)?;
                    for d in decisions {
                        if !connected_ids.contains(&d.base.id.to_string()) {
                            orphans.push(decision_to_response(&d));
                            if orphans.len() >= limit {
                                break;
                            }
                        }
                    }
                }
                "task" => {
                    let tasks = store.list_tasks().map_err(McpError::from)?;
                    for t in tasks {
                        if !connected_ids.contains(&t.base.id.to_string()) {
                            orphans.push(task_to_response(&t));
                            if orphans.len() >= limit {
                                break;
                            }
                        }
                    }
                }
                "note" => {
                    let notes = store.list_notes().map_err(McpError::from)?;
                    for n in notes {
                        if !connected_ids.contains(&n.base.id.to_string()) {
                            orphans.push(note_to_response(&n));
                            if orphans.len() >= limit {
                                break;
                            }
                        }
                    }
                }
                "prompt" => {
                    let prompts = store.list_prompts().map_err(McpError::from)?;
                    for p in prompts {
                        if !connected_ids.contains(&p.base.id.to_string()) {
                            orphans.push(prompt_to_response(&p));
                            if orphans.len() >= limit {
                                break;
                            }
                        }
                    }
                }
                "component" => {
                    let components = store.list_components().map_err(McpError::from)?;
                    for c in components {
                        if !connected_ids.contains(&c.base.id.to_string()) {
                            orphans.push(component_to_response(&c));
                            if orphans.len() >= limit {
                                break;
                            }
                        }
                    }
                }
                "link" => {
                    let links = store.list_links().map_err(McpError::from)?;
                    for l in links {
                        if !connected_ids.contains(&l.base.id.to_string()) {
                            orphans.push(link_to_response(&l));
                            if orphans.len() >= limit {
                                break;
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        let response = serde_json::json!({
            "orphans": orphans,
            "total": orphans.len(),
        });

        let json =
            serde_json::to_string_pretty(&response).map_err(|e| McpError::InternalError {
                message: format!("Failed to serialize orphans: {}", e),
            })?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    // ========================================================================
    // task_ready (Beads Parity)
    // ========================================================================

    /// List tasks with no unresolved blockers, sorted by priority and due date.
    #[tool(
        description = "List tasks that are ready to work on (no unresolved blockers). Returns tasks sorted by priority (urgent > high > normal > low), then by due date."
    )]
    pub async fn task_ready(
        &self,
        Parameters(params): Parameters<TaskReadyParams>,
    ) -> Result<CallToolResult, McpErrorData> {
        let cache = self.cache.lock().await;
        let ready_tasks = cache.get_ready_tasks(params.limit).map_err(McpError::from)?;

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

        let json =
            serde_json::to_string_pretty(&response).map_err(|e| McpError::InternalError {
                message: format!("Failed to serialize ready tasks: {}", e),
            })?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    // ========================================================================
    // task_blocked (Beads Parity)
    // ========================================================================

    /// List blocked tasks and what blocks them.
    #[tool(
        description = "List blocked tasks with their blockers. Optionally get blockers for a specific task by ID."
    )]
    pub async fn task_blocked(
        &self,
        Parameters(params): Parameters<TaskBlockedParams>,
    ) -> Result<CallToolResult, McpErrorData> {
        let cache = self.cache.lock().await;

        // If a specific task ID is provided, get its blockers
        if let Some(ref task_id) = params.id {
            let store = self.store.lock().await;
            let uuid = self.resolve_entity_id(&store, task_id)?;
            drop(store);

            let blockers = cache
                .get_task_blockers(&uuid.to_string())
                .map_err(McpError::from)?;

            let blocker_list: Vec<serde_json::Value> = blockers
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

            let response = serde_json::json!({
                "task_id": uuid.to_string(),
                "blockers": blocker_list,
                "total": blocker_list.len(),
            });

            let json =
                serde_json::to_string_pretty(&response).map_err(|e| McpError::InternalError {
                    message: format!("Failed to serialize task blockers: {}", e),
                })?;

            return Ok(CallToolResult::success(vec![Content::text(json)]));
        }

        // Otherwise, get all blocked tasks
        let blocked_tasks = cache.get_blocked_tasks(params.limit).map_err(McpError::from)?;

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

        let json =
            serde_json::to_string_pretty(&response).map_err(|e| McpError::InternalError {
                message: format!("Failed to serialize blocked tasks: {}", e),
            })?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    // ========================================================================
    // task_next (Beads Parity)
    // ========================================================================

    /// Get the highest-priority ready task.
    #[tool(description = "Get the single highest-priority task that is ready to work on")]
    pub async fn task_next(&self) -> Result<CallToolResult, McpErrorData> {
        let cache = self.cache.lock().await;
        let next_task = cache.get_next_task().map_err(McpError::from)?;

        match next_task {
            Some(t) => {
                let response = serde_json::json!({
                    "id": t.id,
                    "sequence_number": t.sequence_number,
                    "title": t.title,
                    "status": t.status,
                    "priority": t.priority,
                    "due_date": t.due_date,
                    "assignee": t.assignee,
                });

                let json = serde_json::to_string_pretty(&response).map_err(|e| {
                    McpError::InternalError {
                        message: format!("Failed to serialize next task: {}", e),
                    }
                })?;

                Ok(CallToolResult::success(vec![Content::text(json)]))
            }
            None => {
                let response = serde_json::json!({
                    "message": "No ready tasks available",
                    "task": null,
                });

                let json = serde_json::to_string_pretty(&response).map_err(|e| {
                    McpError::InternalError {
                        message: format!("Failed to serialize response: {}", e),
                    }
                })?;

                Ok(CallToolResult::success(vec![Content::text(json)]))
            }
        }
    }

    // ========================================================================
    // task_complete (Convenience)
    // ========================================================================

    /// Mark a task as done.
    #[tool(description = "Mark a task as done (convenience wrapper for entity_update with status=done)")]
    pub async fn task_complete(
        &self,
        Parameters(params): Parameters<TaskCompleteParams>,
    ) -> Result<CallToolResult, McpErrorData> {
        let store = self.store.lock().await;
        let cache = self.cache.lock().await;

        let is_sequence = params.id.chars().all(|c| c.is_ascii_digit());

        // Find the task
        let tasks = store.list_tasks().map_err(McpError::from)?;
        for t in tasks {
            if self.matches_id(&t.base, &params.id, is_sequence) {
                let mut update = TaskUpdate::default();
                update.status = Some(crate::entity::TaskStatus::Done);

                store
                    .update_task(&t.base.id, update)
                    .map_err(McpError::from)?;
                store.save().map_err(McpError::from)?;

                let updated = store
                    .get_task(&t.base.id)
                    .map_err(McpError::from)?
                    .ok_or_else(|| McpError::EntityNotFound {
                        id: params.id.clone(),
                    })?;
                cache.index_task(&updated).map_err(McpError::from)?;

                let response = task_to_response(&updated);
                let json = serde_json::to_string_pretty(&response).map_err(|e| {
                    McpError::InternalError {
                        message: format!("Failed to serialize task: {}", e),
                    }
                })?;

                return Ok(CallToolResult::success(vec![Content::text(json)]));
            }
        }

        Err(McpError::EntityNotFound {
            id: params.id.clone(),
        }
        .into())
    }

    // ========================================================================
    // task_reschedule (Convenience)
    // ========================================================================

    /// Change a task's due date.
    #[tool(description = "Change a task's due date (convenience wrapper for entity_update)")]
    pub async fn task_reschedule(
        &self,
        Parameters(params): Parameters<TaskRescheduleParams>,
    ) -> Result<CallToolResult, McpErrorData> {
        // Validate the due date format
        let due_date = parse_date("due_date", &params.due_date)?;

        let store = self.store.lock().await;
        let cache = self.cache.lock().await;

        let is_sequence = params.id.chars().all(|c| c.is_ascii_digit());

        // Find the task
        let tasks = store.list_tasks().map_err(McpError::from)?;
        for t in tasks {
            if self.matches_id(&t.base, &params.id, is_sequence) {
                let mut update = TaskUpdate::default();
                update.due_date = Some(Some(due_date));

                store
                    .update_task(&t.base.id, update)
                    .map_err(McpError::from)?;
                store.save().map_err(McpError::from)?;

                let updated = store
                    .get_task(&t.base.id)
                    .map_err(McpError::from)?
                    .ok_or_else(|| McpError::EntityNotFound {
                        id: params.id.clone(),
                    })?;
                cache.index_task(&updated).map_err(McpError::from)?;

                let response = task_to_response(&updated);
                let json = serde_json::to_string_pretty(&response).map_err(|e| {
                    McpError::InternalError {
                        message: format!("Failed to serialize task: {}", e),
                    }
                })?;

                return Ok(CallToolResult::success(vec![Content::text(json)]));
            }
        }

        Err(McpError::EntityNotFound {
            id: params.id.clone(),
        }
        .into())
    }

    // ========================================================================
    // decision_supersede (Convenience)
    // ========================================================================

    /// Replace a decision with a new one.
    #[tool(
        description = "Replace a decision with a new one. Updates the old decision's status to 'superseded' and creates a 'supersedes' relation."
    )]
    pub async fn decision_supersede(
        &self,
        Parameters(params): Parameters<DecisionSupersedeParams>,
    ) -> Result<CallToolResult, McpErrorData> {
        let store = self.store.lock().await;
        let cache = self.cache.lock().await;

        // Resolve both IDs to UUIDs
        let old_uuid = self.resolve_entity_id(&store, &params.old_id)?;
        let new_uuid = self.resolve_entity_id(&store, &params.new_id)?;

        // Verify both are decisions
        let old_decision = store
            .get_decision(&old_uuid)
            .map_err(McpError::from)?
            .ok_or_else(|| McpError::EntityNotFound {
                id: params.old_id.clone(),
            })?;

        let _new_decision = store
            .get_decision(&new_uuid)
            .map_err(McpError::from)?
            .ok_or_else(|| McpError::EntityNotFound {
                id: params.new_id.clone(),
            })?;

        // Update old decision to superseded status
        let mut update = DecisionUpdate::default();
        update.status = Some(crate::entity::DecisionStatus::Superseded);
        update.superseded_by = Some(Some(new_uuid.to_string()));

        store
            .update_decision(&old_uuid, update)
            .map_err(McpError::from)?;

        // Create the supersedes relation (new -> old)
        let relation = crate::entity::Relation::new(
            new_uuid,
            "decision".to_string(),
            old_uuid,
            "decision".to_string(),
            crate::entity::RelationType::Supersedes,
        );

        store.add_relation(&relation).map_err(McpError::from)?;
        store.save().map_err(McpError::from)?;

        // Reindex both decisions
        let updated_old = store
            .get_decision(&old_uuid)
            .map_err(McpError::from)?
            .ok_or_else(|| McpError::EntityNotFound {
                id: params.old_id.clone(),
            })?;
        cache.index_decision(&updated_old).map_err(McpError::from)?;
        cache.index_relation(&relation).map_err(McpError::from)?;

        let response = serde_json::json!({
            "old_decision": decision_to_response(&updated_old),
            "new_decision_id": new_uuid.to_string(),
            "relation": {
                "type": "supersedes",
                "from": new_uuid.to_string(),
                "to": old_uuid.to_string(),
            },
            "message": format!(
                "Decision '{}' has been superseded by decision '{}'",
                old_decision.base.title,
                params.new_id
            ),
        });

        let json =
            serde_json::to_string_pretty(&response).map_err(|e| McpError::InternalError {
                message: format!("Failed to serialize response: {}", e),
            })?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    // ========================================================================
    // relation_create
    // ========================================================================

    /// Create a relation between two entities.
    #[tool(
        description = "Create a relation between two entities. Valid relation types: implements, blocks, supersedes, references, belongs_to, documents"
    )]
    pub async fn relation_create(
        &self,
        Parameters(params): Parameters<RelationCreateParams>,
    ) -> Result<CallToolResult, McpErrorData> {
        let store = self.store.lock().await;
        let cache = self.cache.lock().await;

        // Resolve source and target IDs to UUIDs with types
        let (source_uuid, source_type) =
            self.resolve_entity_id_with_type(&store, &params.source_id)?;
        let (target_uuid, target_type) =
            self.resolve_entity_id_with_type(&store, &params.target_id)?;

        // Parse and validate relation type
        let relation_type: crate::entity::RelationType = params
            .relation_type
            .parse()
            .map_err(|e: String| McpError::ValidationFailed {
                field: "relation_type".to_string(),
                message: e,
            })?;

        // Create the relation
        let relation = crate::entity::Relation::new(
            source_uuid,
            source_type.clone(),
            target_uuid,
            target_type.clone(),
            relation_type.clone(),
        );

        // Store and index the relation
        store.add_relation(&relation).map_err(McpError::from)?;
        store.save().map_err(McpError::from)?;
        cache.index_relation(&relation).map_err(McpError::from)?;

        let response = serde_json::json!({
            "source_id": source_uuid.to_string(),
            "source_type": source_type,
            "target_id": target_uuid.to_string(),
            "target_type": target_type,
            "relation_type": relation_type.to_string(),
            "created_at": relation.created_at.to_rfc3339(),
            "message": format!(
                "Created '{}' relation from {} to {}",
                relation_type, params.source_id, params.target_id
            ),
        });

        let json =
            serde_json::to_string_pretty(&response).map_err(|e| McpError::InternalError {
                message: format!("Failed to serialize response: {}", e),
            })?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    // ========================================================================
    // relation_delete
    // ========================================================================

    /// Delete a relation between two entities.
    #[tool(
        description = "Delete a relation between two entities. Specify source, target, and relation type."
    )]
    pub async fn relation_delete(
        &self,
        Parameters(params): Parameters<RelationDeleteParams>,
    ) -> Result<CallToolResult, McpErrorData> {
        let store = self.store.lock().await;
        let cache = self.cache.lock().await;

        // Resolve source and target IDs to UUIDs
        let (source_uuid, _) = self.resolve_entity_id_with_type(&store, &params.source_id)?;
        let (target_uuid, _) = self.resolve_entity_id_with_type(&store, &params.target_id)?;

        // Parse and validate relation type
        let relation_type: crate::entity::RelationType = params
            .relation_type
            .parse()
            .map_err(|e: String| McpError::ValidationFailed {
                field: "relation_type".to_string(),
                message: e,
            })?;

        // Build the composite key for cache deletion
        let composite_key = format!(
            "{}:{}:{}",
            source_uuid,
            relation_type,
            target_uuid
        );

        // Delete from store (takes individual parameters)
        store
            .delete_relation(
                &source_uuid.to_string(),
                &relation_type.to_string(),
                &target_uuid.to_string(),
            )
            .map_err(McpError::from)?;
        store.save().map_err(McpError::from)?;

        // Delete from cache (takes composite key)
        cache.remove_relation(&composite_key).map_err(McpError::from)?;

        let response = serde_json::json!({
            "deleted": true,
            "source_id": source_uuid.to_string(),
            "target_id": target_uuid.to_string(),
            "relation_type": relation_type.to_string(),
            "message": format!(
                "Deleted '{}' relation from {} to {}",
                relation_type, params.source_id, params.target_id
            ),
        });

        let json =
            serde_json::to_string_pretty(&response).map_err(|e| McpError::InternalError {
                message: format!("Failed to serialize response: {}", e),
            })?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }
}

// Helper methods that don't need #[tool] attribute - separate impl block
impl MedullaServer {
    fn find_entity_by_id(
        &self,
        store: &LoroStore,
        entity_type: &str,
        id: &str,
        is_sequence: bool,
    ) -> Result<Option<EntityResponse>, McpError> {
        match entity_type {
            "decision" => {
                let decisions = store.list_decisions().map_err(McpError::from)?;
                for d in decisions {
                    if self.matches_id(&d.base, id, is_sequence) {
                        return Ok(Some(decision_to_response(&d)));
                    }
                }
            }
            "task" => {
                let tasks = store.list_tasks().map_err(McpError::from)?;
                for t in tasks {
                    if self.matches_id(&t.base, id, is_sequence) {
                        return Ok(Some(task_to_response(&t)));
                    }
                }
            }
            "note" => {
                let notes = store.list_notes().map_err(McpError::from)?;
                for n in notes {
                    if self.matches_id(&n.base, id, is_sequence) {
                        return Ok(Some(note_to_response(&n)));
                    }
                }
            }
            "prompt" => {
                let prompts = store.list_prompts().map_err(McpError::from)?;
                for p in prompts {
                    if self.matches_id(&p.base, id, is_sequence) {
                        return Ok(Some(prompt_to_response(&p)));
                    }
                }
            }
            "component" => {
                let components = store.list_components().map_err(McpError::from)?;
                for c in components {
                    if self.matches_id(&c.base, id, is_sequence) {
                        return Ok(Some(component_to_response(&c)));
                    }
                }
            }
            "link" => {
                let links = store.list_links().map_err(McpError::from)?;
                for l in links {
                    if self.matches_id(&l.base, id, is_sequence) {
                        return Ok(Some(link_to_response(&l)));
                    }
                }
            }
            _ => {}
        }
        Ok(None)
    }

    fn matches_id(&self, base: &EntityBase, id: &str, is_sequence: bool) -> bool {
        if is_sequence {
            base.sequence_number.to_string() == id
        } else {
            // UUID prefix match (case-insensitive)
            let uuid_str = base.id.to_string().replace('-', "");
            let search_id = id.replace('-', "").to_lowercase();
            uuid_str.to_lowercase().starts_with(&search_id)
        }
    }

    fn matches_filters(
        &self,
        base: &EntityBase,
        params: &EntityListParams,
        status: Option<&str>,
    ) -> bool {
        // Filter by status
        if let Some(ref filter_status) = params.status {
            if let Some(entity_status) = status {
                if entity_status != filter_status {
                    return false;
                }
            } else {
                return false;
            }
        }

        // Filter by tag
        if let Some(ref filter_tag) = params.tag {
            if !base.tags.iter().any(|t| t == filter_tag) {
                return false;
            }
        }

        true
    }

    fn try_update_entity(
        &self,
        store: &LoroStore,
        cache: &SqliteCache,
        entity_type: &str,
        params: &EntityUpdateParams,
        is_sequence: bool,
    ) -> Result<Option<EntityResponse>, McpError> {
        match entity_type {
            "decision" => {
                let decisions = store.list_decisions().map_err(McpError::from)?;
                for d in decisions {
                    if self.matches_id(&d.base, &params.id, is_sequence) {
                        let mut update = DecisionUpdate::default();
                        update.title = params.title.clone();
                        update.content = params.content.clone();
                        update.add_tags = params.add_tags.clone().unwrap_or_default();
                        update.remove_tags = params.remove_tags.clone().unwrap_or_default();

                        if let Some(ref props) = params.properties {
                            if let Some(status) = props.get("status").and_then(|v| v.as_str()) {
                                update.status = Some(parse_decision_status(status)?);
                            }
                            if let Some(context) = props.get("context").and_then(|v| v.as_str()) {
                                update.context = Some(context.to_string());
                            }
                        }

                        store
                            .update_decision(&d.base.id, update)
                            .map_err(McpError::from)?;
                        store.save().map_err(McpError::from)?;

                        // Refetch and reindex
                        let updated = store
                            .get_decision(&d.base.id)
                            .map_err(McpError::from)?
                            .ok_or_else(|| McpError::EntityNotFound {
                                id: params.id.clone(),
                            })?;
                        cache.index_decision(&updated).map_err(McpError::from)?;

                        return Ok(Some(decision_to_response(&updated)));
                    }
                }
            }
            "task" => {
                let tasks = store.list_tasks().map_err(McpError::from)?;
                for t in tasks {
                    if self.matches_id(&t.base, &params.id, is_sequence) {
                        let mut update = TaskUpdate::default();
                        update.title = params.title.clone();
                        update.content = params.content.clone();
                        update.add_tags = params.add_tags.clone().unwrap_or_default();
                        update.remove_tags = params.remove_tags.clone().unwrap_or_default();

                        if let Some(ref props) = params.properties {
                            if let Some(status) = props.get("status").and_then(|v| v.as_str()) {
                                update.status = Some(parse_task_status(status)?);
                            }
                            if let Some(priority) = props.get("priority").and_then(|v| v.as_str()) {
                                update.priority = Some(parse_task_priority(priority)?);
                            }
                            if let Some(due_date) = props.get("due_date").and_then(|v| v.as_str()) {
                                update.due_date = Some(Some(parse_date("due_date", due_date)?));
                            }
                            if let Some(assignee) = props.get("assignee").and_then(|v| v.as_str()) {
                                update.assignee = Some(Some(assignee.to_string()));
                            }
                        }

                        store
                            .update_task(&t.base.id, update)
                            .map_err(McpError::from)?;
                        store.save().map_err(McpError::from)?;

                        let updated = store
                            .get_task(&t.base.id)
                            .map_err(McpError::from)?
                            .ok_or_else(|| McpError::EntityNotFound {
                                id: params.id.clone(),
                            })?;
                        cache.index_task(&updated).map_err(McpError::from)?;

                        return Ok(Some(task_to_response(&updated)));
                    }
                }
            }
            "note" => {
                let notes = store.list_notes().map_err(McpError::from)?;
                for n in notes {
                    if self.matches_id(&n.base, &params.id, is_sequence) {
                        let mut update = NoteUpdate::default();
                        update.title = params.title.clone();
                        update.content = params.content.clone();
                        update.add_tags = params.add_tags.clone().unwrap_or_default();
                        update.remove_tags = params.remove_tags.clone().unwrap_or_default();

                        if let Some(ref props) = params.properties {
                            if let Some(note_type) = props.get("note_type").and_then(|v| v.as_str())
                            {
                                update.note_type = Some(Some(note_type.to_string()));
                            }
                        }

                        store
                            .update_note(&n.base.id, update)
                            .map_err(McpError::from)?;
                        store.save().map_err(McpError::from)?;

                        let updated = store
                            .get_note(&n.base.id)
                            .map_err(McpError::from)?
                            .ok_or_else(|| McpError::EntityNotFound {
                                id: params.id.clone(),
                            })?;
                        cache.index_note(&updated).map_err(McpError::from)?;

                        return Ok(Some(note_to_response(&updated)));
                    }
                }
            }
            "prompt" => {
                let prompts = store.list_prompts().map_err(McpError::from)?;
                for p in prompts {
                    if self.matches_id(&p.base, &params.id, is_sequence) {
                        let mut update = PromptUpdate::default();
                        update.title = params.title.clone();
                        update.content = params.content.clone();
                        update.add_tags = params.add_tags.clone().unwrap_or_default();
                        update.remove_tags = params.remove_tags.clone().unwrap_or_default();

                        if let Some(ref props) = params.properties {
                            if let Some(template) = props.get("template").and_then(|v| v.as_str()) {
                                update.template = Some(Some(template.to_string()));
                            }
                            if let Some(output_schema) =
                                props.get("output_schema").and_then(|v| v.as_str())
                            {
                                update.output_schema = Some(Some(output_schema.to_string()));
                            }
                            if let Some(add_vars) =
                                props.get("add_variables").and_then(|v| v.as_array())
                            {
                                update.add_variables = add_vars
                                    .iter()
                                    .filter_map(|v| v.as_str().map(String::from))
                                    .collect();
                            }
                            if let Some(remove_vars) =
                                props.get("remove_variables").and_then(|v| v.as_array())
                            {
                                update.remove_variables = remove_vars
                                    .iter()
                                    .filter_map(|v| v.as_str().map(String::from))
                                    .collect();
                            }
                        }

                        store
                            .update_prompt(&p.base.id, update)
                            .map_err(McpError::from)?;
                        store.save().map_err(McpError::from)?;

                        let updated = store
                            .get_prompt(&p.base.id)
                            .map_err(McpError::from)?
                            .ok_or_else(|| McpError::EntityNotFound {
                                id: params.id.clone(),
                            })?;
                        cache.index_prompt(&updated).map_err(McpError::from)?;

                        return Ok(Some(prompt_to_response(&updated)));
                    }
                }
            }
            "component" => {
                let components = store.list_components().map_err(McpError::from)?;
                for c in components {
                    if self.matches_id(&c.base, &params.id, is_sequence) {
                        let mut update = ComponentUpdate::default();
                        update.title = params.title.clone();
                        update.content = params.content.clone();
                        update.add_tags = params.add_tags.clone().unwrap_or_default();
                        update.remove_tags = params.remove_tags.clone().unwrap_or_default();

                        if let Some(ref props) = params.properties {
                            if let Some(status) = props.get("status").and_then(|v| v.as_str()) {
                                update.status = Some(parse_component_status(status)?);
                            }
                            if let Some(component_type) =
                                props.get("component_type").and_then(|v| v.as_str())
                            {
                                update.component_type = Some(Some(component_type.to_string()));
                            }
                            if let Some(owner) = props.get("owner").and_then(|v| v.as_str()) {
                                update.owner = Some(Some(owner.to_string()));
                            }
                        }

                        store
                            .update_component(&c.base.id, update)
                            .map_err(McpError::from)?;
                        store.save().map_err(McpError::from)?;

                        let updated = store
                            .get_component(&c.base.id)
                            .map_err(McpError::from)?
                            .ok_or_else(|| McpError::EntityNotFound {
                                id: params.id.clone(),
                            })?;
                        cache.index_component(&updated).map_err(McpError::from)?;

                        return Ok(Some(component_to_response(&updated)));
                    }
                }
            }
            "link" => {
                let links = store.list_links().map_err(McpError::from)?;
                for l in links {
                    if self.matches_id(&l.base, &params.id, is_sequence) {
                        let mut update = LinkUpdate::default();
                        update.title = params.title.clone();
                        update.content = params.content.clone();
                        update.add_tags = params.add_tags.clone().unwrap_or_default();
                        update.remove_tags = params.remove_tags.clone().unwrap_or_default();

                        if let Some(ref props) = params.properties {
                            if let Some(url) = props.get("url").and_then(|v| v.as_str()) {
                                validate_url(url)?;
                                update.url = Some(url.to_string());
                            }
                            if let Some(link_type) = props.get("link_type").and_then(|v| v.as_str())
                            {
                                update.link_type = Some(Some(link_type.to_string()));
                            }
                        }

                        store
                            .update_link(&l.base.id, update)
                            .map_err(McpError::from)?;
                        store.save().map_err(McpError::from)?;

                        let updated = store
                            .get_link(&l.base.id)
                            .map_err(McpError::from)?
                            .ok_or_else(|| McpError::EntityNotFound {
                                id: params.id.clone(),
                            })?;
                        cache.index_link(&updated).map_err(McpError::from)?;

                        return Ok(Some(link_to_response(&updated)));
                    }
                }
            }
            _ => {}
        }
        Ok(None)
    }

    fn try_delete_entity(
        &self,
        store: &LoroStore,
        cache: &SqliteCache,
        entity_type: &str,
        id: &str,
        is_sequence: bool,
    ) -> Result<bool, McpError> {
        match entity_type {
            "decision" => {
                let decisions = store.list_decisions().map_err(McpError::from)?;
                for d in decisions {
                    if self.matches_id(&d.base, id, is_sequence) {
                        store.delete_decision(&d.base.id).map_err(McpError::from)?;
                        store.save().map_err(McpError::from)?;
                        cache.remove_decision(&d.base.id.to_string()).map_err(McpError::from)?;
                        return Ok(true);
                    }
                }
            }
            "task" => {
                let tasks = store.list_tasks().map_err(McpError::from)?;
                for t in tasks {
                    if self.matches_id(&t.base, id, is_sequence) {
                        store.delete_task(&t.base.id).map_err(McpError::from)?;
                        store.save().map_err(McpError::from)?;
                        cache.remove_task(&t.base.id.to_string()).map_err(McpError::from)?;
                        return Ok(true);
                    }
                }
            }
            "note" => {
                let notes = store.list_notes().map_err(McpError::from)?;
                for n in notes {
                    if self.matches_id(&n.base, id, is_sequence) {
                        store.delete_note(&n.base.id).map_err(McpError::from)?;
                        store.save().map_err(McpError::from)?;
                        cache.remove_note(&n.base.id.to_string()).map_err(McpError::from)?;
                        return Ok(true);
                    }
                }
            }
            "prompt" => {
                let prompts = store.list_prompts().map_err(McpError::from)?;
                for p in prompts {
                    if self.matches_id(&p.base, id, is_sequence) {
                        store.delete_prompt(&p.base.id).map_err(McpError::from)?;
                        store.save().map_err(McpError::from)?;
                        cache.remove_prompt(&p.base.id.to_string()).map_err(McpError::from)?;
                        return Ok(true);
                    }
                }
            }
            "component" => {
                let components = store.list_components().map_err(McpError::from)?;
                for c in components {
                    if self.matches_id(&c.base, id, is_sequence) {
                        store.delete_component(&c.base.id).map_err(McpError::from)?;
                        store.save().map_err(McpError::from)?;
                        cache.remove_component(&c.base.id.to_string()).map_err(McpError::from)?;
                        return Ok(true);
                    }
                }
            }
            "link" => {
                let links = store.list_links().map_err(McpError::from)?;
                for l in links {
                    if self.matches_id(&l.base, id, is_sequence) {
                        store.delete_link(&l.base.id).map_err(McpError::from)?;
                        store.save().map_err(McpError::from)?;
                        cache.remove_link(&l.base.id.to_string()).map_err(McpError::from)?;
                        return Ok(true);
                    }
                }
            }
            _ => {}
        }
        Ok(false)
    }

    fn resolve_entity_id(
        &self,
        store: &LoroStore,
        id: &str,
    ) -> Result<uuid::Uuid, McpError> {
        let is_sequence = id.chars().all(|c| c.is_ascii_digit());

        for entity_type in VALID_ENTITY_TYPES {
            if let Some(uuid) = self.find_uuid_by_id(store, entity_type, id, is_sequence)? {
                return Ok(uuid);
            }
        }

        Err(McpError::EntityNotFound { id: id.to_string() })
    }

    fn find_uuid_by_id(
        &self,
        store: &LoroStore,
        entity_type: &str,
        id: &str,
        is_sequence: bool,
    ) -> Result<Option<uuid::Uuid>, McpError> {
        match entity_type {
            "decision" => {
                let decisions = store.list_decisions().map_err(McpError::from)?;
                for d in decisions {
                    if self.matches_id(&d.base, id, is_sequence) {
                        return Ok(Some(d.base.id));
                    }
                }
            }
            "task" => {
                let tasks = store.list_tasks().map_err(McpError::from)?;
                for t in tasks {
                    if self.matches_id(&t.base, id, is_sequence) {
                        return Ok(Some(t.base.id));
                    }
                }
            }
            "note" => {
                let notes = store.list_notes().map_err(McpError::from)?;
                for n in notes {
                    if self.matches_id(&n.base, id, is_sequence) {
                        return Ok(Some(n.base.id));
                    }
                }
            }
            "prompt" => {
                let prompts = store.list_prompts().map_err(McpError::from)?;
                for p in prompts {
                    if self.matches_id(&p.base, id, is_sequence) {
                        return Ok(Some(p.base.id));
                    }
                }
            }
            "component" => {
                let components = store.list_components().map_err(McpError::from)?;
                for c in components {
                    if self.matches_id(&c.base, id, is_sequence) {
                        return Ok(Some(c.base.id));
                    }
                }
            }
            "link" => {
                let links = store.list_links().map_err(McpError::from)?;
                for l in links {
                    if self.matches_id(&l.base, id, is_sequence) {
                        return Ok(Some(l.base.id));
                    }
                }
            }
            _ => {}
        }
        Ok(None)
    }

    /// Resolve an entity ID to UUID and entity type.
    fn resolve_entity_id_with_type(
        &self,
        store: &LoroStore,
        id: &str,
    ) -> Result<(uuid::Uuid, String), McpError> {
        let is_sequence = id.chars().all(|c| c.is_ascii_digit());

        for entity_type in VALID_ENTITY_TYPES {
            if let Some(uuid) = self.find_uuid_by_id(store, entity_type, id, is_sequence)? {
                return Ok((uuid, entity_type.to_string()));
            }
        }

        Err(McpError::EntityNotFound { id: id.to_string() })
    }
}

#[tool_handler]
impl ServerHandler for MedullaServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::LATEST,
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(
                "Medulla is a knowledge engine for software projects. \
                 Use entity tools to manage decisions, tasks, notes, prompts, \
                 components, and links. Use search tools for full-text search. \
                 Use graph tools to explore relationships."
                    .to_string(),
            ),
        }
    }

    fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<ListResourcesResult, rmcp::ErrorData>> + Send + '_
    {
        use rmcp::model::AnnotateAble;
        async move {
            let static_resources = resources::build_static_resources();
            Ok(ListResourcesResult {
                resources: static_resources
                    .into_iter()
                    .map(|r| r.no_annotation())
                    .collect(),
                next_cursor: None,
                meta: None,
            })
        }
    }

    fn list_resource_templates(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<ListResourceTemplatesResult, rmcp::ErrorData>>
           + Send
           + '_ {
        use rmcp::model::AnnotateAble;
        async move {
            let templates = resources::build_resource_templates();
            Ok(ListResourceTemplatesResult {
                resource_templates: templates.into_iter().map(|t| t.no_annotation()).collect(),
                next_cursor: None,
                meta: None,
            })
        }
    }

    fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<ReadResourceResult, rmcp::ErrorData>> + Send + '_
    {
        async move {
            resources::read_resource(&request.uri, &self.store, &self.cache)
                .await
                .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))
        }
    }

    fn subscribe(
        &self,
        request: SubscribeRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<(), rmcp::ErrorData>> + Send + '_ {
        async move {
            // Validate the URI starts with medulla://
            if !request.uri.starts_with(resources::MEDULLA_SCHEME) {
                return Err(rmcp::ErrorData::invalid_params(
                    format!("Invalid resource URI: {}", request.uri),
                    None,
                ));
            }

            let mut subscriptions = self.subscriptions.lock().await;
            subscriptions.subscribe(&request.uri);
            Ok(())
        }
    }

    fn unsubscribe(
        &self,
        request: UnsubscribeRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<(), rmcp::ErrorData>> + Send + '_ {
        async move {
            let mut subscriptions = self.subscriptions.lock().await;
            // Note: The unsubscribe method takes an ID, but we're receiving a URI.
            // For now, we'll remove all subscriptions for this URI.
            // In a full implementation, we'd track subscription IDs per client.
            subscriptions.by_resource.remove(&request.uri);
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::SqliteCache;
    use crate::storage::LoroStore;
    use tempfile::TempDir;

    fn setup_test_server() -> (MedullaServer, TempDir) {
        let tmp = TempDir::new().unwrap();
        let store = LoroStore::init(tmp.path()).unwrap();
        let medulla_dir = tmp.path().join(".medulla");
        let cache = SqliteCache::open(&medulla_dir).unwrap();
        let server = MedullaServer::new(store, cache);
        (server, tmp)
    }

    #[test]
    fn test_subscription_state() {
        let mut state = SubscriptionState::new();

        // Subscribe
        let id1 = state.subscribe("medulla://entities/task");
        let _id2 = state.subscribe("medulla://entities/task");
        let _id3 = state.subscribe("medulla://entities/decision");

        assert_eq!(state.get_subscribers("medulla://entities/task").len(), 2);
        assert_eq!(
            state.get_subscribers("medulla://entities/decision").len(),
            1
        );
        assert_eq!(state.get_subscribers("medulla://entities/note").len(), 0);

        // Unsubscribe
        assert!(state.unsubscribe(&id1));
        assert_eq!(state.get_subscribers("medulla://entities/task").len(), 1);

        // Clear
        state.clear();
        assert_eq!(state.get_subscribers("medulla://entities/task").len(), 0);
        assert_eq!(
            state.get_subscribers("medulla://entities/decision").len(),
            0
        );
    }

    #[test]
    fn test_server_info() {
        let (server, _tmp) = setup_test_server();
        let info = server.get_info();

        assert!(info.instructions.is_some());
        assert!(info.instructions.unwrap().contains("Medulla"));
    }

    #[test]
    fn test_build_static_resources() {
        let resources = resources::build_static_resources();
        assert_eq!(resources.len(), 9);
        assert!(resources.iter().any(|r| r.uri == "medulla://schema"));
        assert!(resources.iter().any(|r| r.uri == "medulla://stats"));
        assert!(resources.iter().any(|r| r.uri == "medulla://entities"));
        assert!(resources.iter().any(|r| r.uri == "medulla://tasks/ready"));
        assert!(resources.iter().any(|r| r.uri == "medulla://graph"));
    }

    #[test]
    fn test_build_resource_templates() {
        let templates = resources::build_resource_templates();
        assert_eq!(templates.len(), 5);
        assert!(templates
            .iter()
            .any(|t| t.uri_template == "medulla://entities/{type}"));
        assert!(templates
            .iter()
            .any(|t| t.uri_template == "medulla://entity/{id}"));
        assert!(templates
            .iter()
            .any(|t| t.uri_template == "medulla://tasks/due/{date}"));
    }

    #[tokio::test]
    async fn test_read_resource_schema() {
        let (server, _tmp) = setup_test_server();
        let result =
            resources::read_resource("medulla://schema", &server.store, &server.cache).await;

        assert!(result.is_ok());
        let read_result = result.unwrap();
        assert_eq!(read_result.contents.len(), 1);
    }

    #[tokio::test]
    async fn test_read_resource_stats() {
        let (server, _tmp) = setup_test_server();
        let result =
            resources::read_resource("medulla://stats", &server.store, &server.cache).await;

        assert!(result.is_ok());
        let read_result = result.unwrap();
        assert_eq!(read_result.contents.len(), 1);

        if let ResourceContents::TextResourceContents { text, .. } = &read_result.contents[0] {
            let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
            assert!(parsed["entity_counts"].is_object());
            assert!(parsed["medulla_version"].is_string());
        } else {
            panic!("Expected TextResourceContents");
        }
    }

    #[tokio::test]
    async fn test_read_resource_invalid_uri() {
        let (server, _tmp) = setup_test_server();
        let result =
            resources::read_resource("invalid://schema", &server.store, &server.cache).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_subscription_via_server() {
        let (server, _tmp) = setup_test_server();

        // Subscribe directly via subscription state
        {
            let mut subs = server.subscriptions.lock().await;
            let _id = subs.subscribe("medulla://entities/task");
            assert!(!subs.get_subscribers("medulla://entities/task").is_empty());
        }

        // Unsubscribe via removing by URI
        {
            let mut subs = server.subscriptions.lock().await;
            subs.by_resource.remove("medulla://entities/task");
            assert!(subs.get_subscribers("medulla://entities/task").is_empty());
        }
    }

    // ========================================================================
    // MCP Tool Unit Tests
    // ========================================================================

    #[tokio::test]
    async fn test_entity_create_decision() {
        let (server, _tmp) = setup_test_server();

        let params = EntityCreateParams {
            entity_type: "decision".to_string(),
            title: "Use Rust for CLI".to_string(),
            content: Some("We decided to use Rust for the CLI implementation.".to_string()),
            tags: Some(vec!["lang".to_string(), "tooling".to_string()]),
            properties: Some(serde_json::json!({
                "status": "accepted",
                "context": "Need a fast, reliable CLI"
            })),
        };

        let result = server
            .entity_create(rmcp::handler::server::wrapper::Parameters(params))
            .await;

        assert!(result.is_ok());
        let tool_result = result.unwrap();
        assert!(!tool_result.is_error.unwrap_or(false));

        // Verify the entity was created
        let store = server.store.lock().await;
        let decisions = store.list_decisions().unwrap();
        assert_eq!(decisions.len(), 1);
        assert_eq!(decisions[0].base.title, "Use Rust for CLI");
        assert_eq!(decisions[0].status, crate::entity::DecisionStatus::Accepted);
    }

    #[tokio::test]
    async fn test_entity_create_task() {
        let (server, _tmp) = setup_test_server();

        let params = EntityCreateParams {
            entity_type: "task".to_string(),
            title: "Implement auth".to_string(),
            content: None,
            tags: Some(vec!["backend".to_string()]),
            properties: Some(serde_json::json!({
                "status": "todo",
                "priority": "high",
                "due_date": "2025-03-01",
                "assignee": "alice"
            })),
        };

        let result = server
            .entity_create(rmcp::handler::server::wrapper::Parameters(params))
            .await;

        assert!(result.is_ok());

        let store = server.store.lock().await;
        let tasks = store.list_tasks().unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].base.title, "Implement auth");
        assert_eq!(tasks[0].priority, crate::entity::TaskPriority::High);
        assert_eq!(tasks[0].assignee, Some("alice".to_string()));
    }

    #[tokio::test]
    async fn test_entity_create_validation_error_empty_title() {
        let (server, _tmp) = setup_test_server();

        let params = EntityCreateParams {
            entity_type: "decision".to_string(),
            title: "   ".to_string(), // whitespace only
            content: None,
            tags: None,
            properties: None,
        };

        let result = server
            .entity_create(rmcp::handler::server::wrapper::Parameters(params))
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("Title"));
    }

    #[tokio::test]
    async fn test_entity_create_validation_error_invalid_type() {
        let (server, _tmp) = setup_test_server();

        let params = EntityCreateParams {
            entity_type: "invalid_type".to_string(),
            title: "Test".to_string(),
            content: None,
            tags: None,
            properties: None,
        };

        let result = server
            .entity_create(rmcp::handler::server::wrapper::Parameters(params))
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_entity_get_by_sequence_number() {
        let (server, _tmp) = setup_test_server();

        // Create an entity first
        let create_params = EntityCreateParams {
            entity_type: "decision".to_string(),
            title: "Test Decision".to_string(),
            content: None,
            tags: None,
            properties: None,
        };
        server
            .entity_create(rmcp::handler::server::wrapper::Parameters(create_params))
            .await
            .unwrap();

        // Get by sequence number
        let get_params = EntityGetParams {
            id: "1".to_string(),
            entity_type: None,
        };

        let result = server
            .entity_get(rmcp::handler::server::wrapper::Parameters(get_params))
            .await;

        assert!(result.is_ok());
        let tool_result = result.unwrap();
        if let rmcp::model::RawContent::Text(t) = &tool_result.content[0].raw {
            assert!(t.text.contains("Test Decision"));
        }
    }

    #[tokio::test]
    async fn test_entity_get_not_found() {
        let (server, _tmp) = setup_test_server();

        let get_params = EntityGetParams {
            id: "999".to_string(),
            entity_type: None,
        };

        let result = server
            .entity_get(rmcp::handler::server::wrapper::Parameters(get_params))
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("not found") || err.code.0 == -32001);
    }

    #[tokio::test]
    async fn test_entity_list_all() {
        let (server, _tmp) = setup_test_server();

        // Create multiple entities
        for title in ["Decision A", "Decision B"] {
            let params = EntityCreateParams {
                entity_type: "decision".to_string(),
                title: title.to_string(),
                content: None,
                tags: None,
                properties: None,
            };
            server
                .entity_create(rmcp::handler::server::wrapper::Parameters(params))
                .await
                .unwrap();
        }

        let list_params = EntityListParams {
            entity_type: Some("decision".to_string()),
            status: None,
            tag: None,
            limit: None,
            offset: None,
        };

        let result = server
            .entity_list(rmcp::handler::server::wrapper::Parameters(list_params))
            .await;

        assert!(result.is_ok());
        let tool_result = result.unwrap();
        if let rmcp::model::RawContent::Text(t) = &tool_result.content[0].raw {
            let parsed: serde_json::Value = serde_json::from_str(&t.text).unwrap();
            assert_eq!(parsed["total"], 2);
        }
    }

    #[tokio::test]
    async fn test_entity_list_with_status_filter() {
        let (server, _tmp) = setup_test_server();

        // Create decisions with different statuses
        for (title, status) in [("Accepted", "accepted"), ("Proposed", "proposed")] {
            let params = EntityCreateParams {
                entity_type: "decision".to_string(),
                title: title.to_string(),
                content: None,
                tags: None,
                properties: Some(serde_json::json!({ "status": status })),
            };
            server
                .entity_create(rmcp::handler::server::wrapper::Parameters(params))
                .await
                .unwrap();
        }

        let list_params = EntityListParams {
            entity_type: Some("decision".to_string()),
            status: Some("accepted".to_string()),
            tag: None,
            limit: None,
            offset: None,
        };

        let result = server
            .entity_list(rmcp::handler::server::wrapper::Parameters(list_params))
            .await;

        assert!(result.is_ok());
        let tool_result = result.unwrap();
        if let rmcp::model::RawContent::Text(t) = &tool_result.content[0].raw {
            let parsed: serde_json::Value = serde_json::from_str(&t.text).unwrap();
            assert_eq!(parsed["total"], 1);
        }
    }

    #[tokio::test]
    async fn test_entity_update() {
        let (server, _tmp) = setup_test_server();

        // Create an entity
        let create_params = EntityCreateParams {
            entity_type: "decision".to_string(),
            title: "Original Title".to_string(),
            content: None,
            tags: Some(vec!["old-tag".to_string()]),
            properties: Some(serde_json::json!({ "status": "proposed" })),
        };
        server
            .entity_create(rmcp::handler::server::wrapper::Parameters(create_params))
            .await
            .unwrap();

        // Update it
        let update_params = EntityUpdateParams {
            id: "1".to_string(),
            title: Some("Updated Title".to_string()),
            content: Some("New content".to_string()),
            add_tags: Some(vec!["new-tag".to_string()]),
            remove_tags: Some(vec!["old-tag".to_string()]),
            properties: Some(serde_json::json!({ "status": "accepted" })),
        };

        let result = server
            .entity_update(rmcp::handler::server::wrapper::Parameters(update_params))
            .await;

        assert!(result.is_ok());

        // Verify
        let store = server.store.lock().await;
        let decisions = store.list_decisions().unwrap();
        assert_eq!(decisions[0].base.title, "Updated Title");
        assert_eq!(decisions[0].status, crate::entity::DecisionStatus::Accepted);
        assert!(decisions[0].base.tags.contains(&"new-tag".to_string()));
        assert!(!decisions[0].base.tags.contains(&"old-tag".to_string()));
    }

    #[tokio::test]
    async fn test_entity_delete() {
        let (server, _tmp) = setup_test_server();

        // Create an entity
        let create_params = EntityCreateParams {
            entity_type: "decision".to_string(),
            title: "To Be Deleted".to_string(),
            content: None,
            tags: None,
            properties: None,
        };
        server
            .entity_create(rmcp::handler::server::wrapper::Parameters(create_params))
            .await
            .unwrap();

        // Delete it
        let delete_params = EntityDeleteParams {
            id: "1".to_string(),
        };

        let result = server
            .entity_delete(rmcp::handler::server::wrapper::Parameters(delete_params))
            .await;

        assert!(result.is_ok());

        // Verify it's gone
        let store = server.store.lock().await;
        let decisions = store.list_decisions().unwrap();
        assert!(decisions.is_empty());
    }

    #[tokio::test]
    async fn test_search_fulltext() {
        let (server, _tmp) = setup_test_server();

        // Create entities with searchable content
        for (title, content) in [
            ("PostgreSQL Decision", "Use PostgreSQL for database storage"),
            ("Redis Decision", "Use Redis for caching"),
        ] {
            let params = EntityCreateParams {
                entity_type: "decision".to_string(),
                title: title.to_string(),
                content: Some(content.to_string()),
                tags: None,
                properties: None,
            };
            server
                .entity_create(rmcp::handler::server::wrapper::Parameters(params))
                .await
                .unwrap();
        }

        // Search for PostgreSQL
        let search_params = SearchFulltextParams {
            query: "PostgreSQL".to_string(),
            entity_type: None,
            limit: None,
        };

        let result = server
            .search_fulltext(rmcp::handler::server::wrapper::Parameters(search_params))
            .await;

        assert!(result.is_ok());
        let tool_result = result.unwrap();
        if let rmcp::model::RawContent::Text(t) = &tool_result.content[0].raw {
            let parsed: serde_json::Value = serde_json::from_str(&t.text).unwrap();
            assert_eq!(parsed["total"], 1);
            assert!(parsed["results"][0]["title"]
                .as_str()
                .unwrap()
                .contains("PostgreSQL"));
        }
    }

    #[tokio::test]
    async fn test_task_ready() {
        let (server, _tmp) = setup_test_server();

        // Create tasks
        for (title, priority) in [("Task High", "high"), ("Task Low", "low")] {
            let params = EntityCreateParams {
                entity_type: "task".to_string(),
                title: title.to_string(),
                content: None,
                tags: None,
                properties: Some(serde_json::json!({
                    "status": "todo",
                    "priority": priority
                })),
            };
            server
                .entity_create(rmcp::handler::server::wrapper::Parameters(params))
                .await
                .unwrap();
        }

        let ready_params = TaskReadyParams { limit: None };

        let result = server
            .task_ready(rmcp::handler::server::wrapper::Parameters(ready_params))
            .await;

        assert!(result.is_ok());
        let tool_result = result.unwrap();
        if let rmcp::model::RawContent::Text(t) = &tool_result.content[0].raw {
            let parsed: serde_json::Value = serde_json::from_str(&t.text).unwrap();
            assert_eq!(parsed["total"], 2);
            // High priority should come first
            assert!(parsed["tasks"][0]["title"].as_str().unwrap().contains("High"));
        }
    }

    #[tokio::test]
    async fn test_task_next() {
        let (server, _tmp) = setup_test_server();

        // Create tasks with different priorities
        for (title, priority) in [("Normal Task", "normal"), ("Urgent Task", "urgent")] {
            let params = EntityCreateParams {
                entity_type: "task".to_string(),
                title: title.to_string(),
                content: None,
                tags: None,
                properties: Some(serde_json::json!({
                    "status": "todo",
                    "priority": priority
                })),
            };
            server
                .entity_create(rmcp::handler::server::wrapper::Parameters(params))
                .await
                .unwrap();
        }

        let result = server.task_next().await;

        assert!(result.is_ok());
        let tool_result = result.unwrap();
        if let rmcp::model::RawContent::Text(t) = &tool_result.content[0].raw {
            let parsed: serde_json::Value = serde_json::from_str(&t.text).unwrap();
            // Urgent should be first
            assert!(parsed["title"].as_str().unwrap().contains("Urgent"));
        }
    }

    #[tokio::test]
    async fn test_task_complete() {
        let (server, _tmp) = setup_test_server();

        // Create a task
        let params = EntityCreateParams {
            entity_type: "task".to_string(),
            title: "Task to Complete".to_string(),
            content: None,
            tags: None,
            properties: Some(serde_json::json!({ "status": "todo" })),
        };
        server
            .entity_create(rmcp::handler::server::wrapper::Parameters(params))
            .await
            .unwrap();

        // Complete it
        let complete_params = TaskCompleteParams {
            id: "1".to_string(),
        };

        let result = server
            .task_complete(rmcp::handler::server::wrapper::Parameters(complete_params))
            .await;

        assert!(result.is_ok());

        // Verify it's done
        let store = server.store.lock().await;
        let tasks = store.list_tasks().unwrap();
        assert_eq!(tasks[0].status, crate::entity::TaskStatus::Done);
    }

    #[tokio::test]
    async fn test_graph_relations() {
        let (server, _tmp) = setup_test_server();

        // Create two decisions
        for title in ["Decision A", "Decision B"] {
            let params = EntityCreateParams {
                entity_type: "decision".to_string(),
                title: title.to_string(),
                content: None,
                tags: None,
                properties: None,
            };
            server
                .entity_create(rmcp::handler::server::wrapper::Parameters(params))
                .await
                .unwrap();
        }

        // Add a relation
        {
            let store = server.store.lock().await;
            let decisions = store.list_decisions().unwrap();
            let relation = crate::entity::Relation::new(
                decisions[0].base.id,
                "decision".to_string(),
                decisions[1].base.id,
                "decision".to_string(),
                crate::entity::RelationType::Supersedes,
            );
            store.add_relation(&relation).unwrap();
            store.save().unwrap();
        }

        // Query relations
        let params = GraphRelationsParams {
            id: "1".to_string(),
            direction: Some("both".to_string()),
        };

        let result = server
            .graph_relations(rmcp::handler::server::wrapper::Parameters(params))
            .await;

        assert!(result.is_ok());
        let tool_result = result.unwrap();
        if let rmcp::model::RawContent::Text(t) = &tool_result.content[0].raw {
            let parsed: serde_json::Value = serde_json::from_str(&t.text).unwrap();
            assert!(!parsed["outgoing"].as_array().unwrap().is_empty());
        }
    }

    #[tokio::test]
    async fn test_graph_orphans() {
        let (server, _tmp) = setup_test_server();

        // Create an orphan entity (no relations)
        let params = EntityCreateParams {
            entity_type: "decision".to_string(),
            title: "Orphan Decision".to_string(),
            content: None,
            tags: None,
            properties: None,
        };
        server
            .entity_create(rmcp::handler::server::wrapper::Parameters(params))
            .await
            .unwrap();

        let orphan_params = GraphOrphansParams {
            entity_type: None,
            limit: None,
        };

        let result = server
            .graph_orphans(rmcp::handler::server::wrapper::Parameters(orphan_params))
            .await;

        assert!(result.is_ok());
        let tool_result = result.unwrap();
        if let rmcp::model::RawContent::Text(t) = &tool_result.content[0].raw {
            let parsed: serde_json::Value = serde_json::from_str(&t.text).unwrap();
            assert_eq!(parsed["total"], 1);
        }
    }

    #[tokio::test]
    async fn test_entity_batch_mixed_operations() {
        let (server, _tmp) = setup_test_server();

        // First, create an entity to update/delete later
        let create_params = EntityCreateParams {
            entity_type: "decision".to_string(),
            title: "Existing Decision".to_string(),
            content: None,
            tags: None,
            properties: None,
        };
        server
            .entity_create(rmcp::handler::server::wrapper::Parameters(create_params))
            .await
            .unwrap();

        // Batch with create, update, delete (and one invalid delete)
        let batch_params = EntityBatchParams {
            operations: vec![
                BatchOperation::Create(EntityCreateParams {
                    entity_type: "decision".to_string(),
                    title: "New via Batch".to_string(),
                    content: None,
                    tags: None,
                    properties: None,
                }),
                BatchOperation::Update(EntityUpdateParams {
                    id: "1".to_string(),
                    title: Some("Updated via Batch".to_string()),
                    content: None,
                    add_tags: None,
                    remove_tags: None,
                    properties: None,
                }),
                BatchOperation::Delete(EntityDeleteParams {
                    id: "999".to_string(), // doesn't exist
                }),
            ],
        };

        let result = server
            .entity_batch(rmcp::handler::server::wrapper::Parameters(batch_params))
            .await;

        assert!(result.is_ok());
        let tool_result = result.unwrap();
        if let rmcp::model::RawContent::Text(t) = &tool_result.content[0].raw {
            let parsed: serde_json::Value = serde_json::from_str(&t.text).unwrap();
            assert_eq!(parsed["succeeded"], 2); // create and update succeed
            assert_eq!(parsed["failed"], 1);    // delete fails
        }
    }
}
