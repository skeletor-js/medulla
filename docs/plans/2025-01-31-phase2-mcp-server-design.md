# Phase 2: MCP Server Design

**Date**: 2025-01-31
**Updated**: 2026-02-02
**Status**: Approved

## Overview

Implement an MCP (Model Context Protocol) server that exposes Medulla's knowledge engine to AI tools like Claude Desktop, Cursor, and Copilot via the standard MCP protocol.

### Design Goals

1. **MCP-first**: Full protocol compliance with tools, resources, and subscriptions
2. **CLI parity**: Every MCP capability has a corresponding CLI command
3. **Beads parity**: Task queue management features (ready tasks, blockers) matching [steveyegge/beads](https://github.com/steveyegge/beads), but with Medulla's richer context (decisions, semantic search, relations)

## Architecture

```
┌─────────────────────────────────────────┐
│  Claude Desktop / Cursor / AI Tools    │
└────────────────┬────────────────────────┘
                 │ stdio (JSON-RPC)
┌────────────────▼────────────────────────┐
│  MCP Server (rmcp)                      │
│  - Tools: entity_*, search_*, graph_*   │
│  - Resources: medulla://decisions, ...  │
└────────────────┬────────────────────────┘
                 │
┌────────────────▼────────────────────────┐
│  LoroStore + SqliteCache (existing)     │
└─────────────────────────────────────────┘
```

**Key decisions:**

- Use `rmcp` crate (v0.14.0) with features: `server`, `macros`, `transport-io`
- Server struct holds `Arc<Mutex<LoroStore>>` and `Arc<SqliteCache>` for thread-safe access
- CLI gains `medulla serve` command that starts the MCP server

## Server Capabilities

The MCP server advertises the following capabilities during initialization:

```json
{
  "protocolVersion": "2025-11-25",
  "serverInfo": {
    "name": "medulla",
    "version": "0.2.0"
  },
  "capabilities": {
    "tools": { "listChanged": false },
    "resources": { "subscribe": true, "listChanged": false },
    "prompts": { "listChanged": false },
    "logging": {}
  }
}
```

**Capability notes:**

- `tools.listChanged: false` — Tool list is static (no dynamic tool registration)
- `resources.subscribe: true` — By-type subscriptions supported (TD20)
- `resources.listChanged: false` — Resource list is static
- `prompts.listChanged: false` — Prompts deferred to future phase (returns empty list)
- `completions` — Not implemented in Phase 2

**Protocol version:**

- Server requires MCP 2025-11-25
- On version mismatch: Return error with supported versions

## Dependencies

```toml
# MCP Server
rmcp = { version = "0.14", features = ["server", "macros", "transport-io"] }
tokio = { version = "1", features = ["rt-multi-thread", "macros", "io-std", "signal"] }
schemars = "0.8"  # For JSON schema generation on tool params
tracing = "0.1"   # Structured logging
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

## Module Structure

```
src/
├── mcp/
│   ├── mod.rs           # MCP module exports, MedullaServer struct
│   ├── tools.rs         # Tool implementations (entity_*, search_*, graph_*)
│   ├── resources.rs     # Resource implementations (medulla://*)
│   └── error.rs         # MCP-specific error types and mapping
├── storage/
│   ├── mod.rs           # Existing module exports
│   ├── loro_store.rs    # Existing Loro storage
│   └── queries.rs       # NEW: Task queue queries (ready, blocked)
├── lib.rs               # Add `pub mod mcp;`
└── main.rs              # Add `serve` subcommand
```

**MedullaServer struct:**

```rust
#[derive(Clone)]
pub struct MedullaServer {
    store: Arc<Mutex<LoroStore>>,
    cache: Arc<SqliteCache>,
    subscriptions: Arc<Mutex<SubscriptionState>>,
}

struct SubscriptionState {
    by_resource: HashMap<String, Vec<SubscriptionId>>,
}
```

## Concurrency Model

### Thread Safety

The MCP server handles concurrent requests with the following model:

```
Request → Acquire Lock → Execute → Release Lock → Response
```

**Lock strategy:**

- `Arc<Mutex<LoroStore>>` — Single mutex for all Loro operations
- `Arc<SqliteCache>` — SQLite handles its own thread safety internally
- `Arc<Mutex<SubscriptionState>>` — Separate mutex for subscription management

**Lock ordering (to prevent deadlocks):**

1. Always acquire `store` lock before `subscriptions` lock if both needed
2. Never hold locks across async `.await` points
3. Release locks immediately after operation completes

**Read operations:**

```rust
async fn entity_get(&self, id: &str) -> Result<Entity> {
    let store = self.store.lock().await;
    let entity = store.get_entity(id)?;
    drop(store); // Explicit release
    Ok(entity)
}
```

**Write operations:**

```rust
async fn entity_create(&self, params: CreateParams) -> Result<Entity> {
    let mut store = self.store.lock().await;
    let entity = store.create_entity(params)?;
    store.save()?;
    self.cache.sync_entity(&entity)?;
    drop(store);

    // Notify subscribers (separate lock)
    self.notify_subscribers(&format!("medulla://entities/{}", entity.entity_type)).await;
    Ok(entity)
}
```

**Performance considerations:**

- Mutex contention is expected to be low (single-user typical use case)
- If read-heavy workloads measured in production, consider `RwLock` upgrade
- Batch operations hold lock for entire batch (acceptable for best-effort semantics)

## Tools

### Core Entity Tools

| Tool | Description | Parameters |
|------|-------------|------------|
| `entity_create` | Create any entity type | `type`, `title`, `content?`, `tags?`, `properties` (type-specific) |
| `entity_get` | Get by ID | `id` (seq number or UUID prefix) |
| `entity_list` | List with filters | `type?`, `status?`, `tag?`, `limit?`, `offset?` |
| `entity_update` | Update entity | `id`, fields to update |
| `entity_delete` | Delete entity | `id` |
| `entity_batch` | Batch operations | `operations[]` (best-effort semantics) |

### Search Tool

| Tool | Description | Parameters |
|------|-------------|------------|
| `search_fulltext` | FTS via SQLite | `query`, `type?`, `limit?` |

### Graph Tools

| Tool | Description | Parameters |
|------|-------------|------------|
| `graph_relations` | Get relations for entity | `id`, `direction?` (from/to/both) |
| `graph_path` | Find shortest path between entities | `from_id`, `to_id`, `max_depth?` |
| `graph_orphans` | Find entities with no relations | `type?`, `limit?` |

**Graph tool implementation notes:**

- `graph_path`: BFS traversal through relations, returns path as array of entity IDs
  - Default `max_depth`: 10
  - Returns empty array if no path exists
  - Returns `[from_id]` if `from_id == to_id`

- `graph_orphans`: Entities where neither incoming nor outgoing relations exist
  - Useful for finding disconnected knowledge
  - Can filter by entity type

### Task Queue Tools (Beads Parity)

These tools provide feature parity with [Beads](https://github.com/steveyegge/beads) for AI agent task management:

| Tool | Description | Parameters |
|------|-------------|------------|
| `task_ready` | List tasks with no unresolved blocking dependencies | `limit?`, `priority?` |
| `task_blocked` | List blocked tasks and what blocks them | `id?` (single task or all if omitted) |
| `task_next` | Get the highest-priority ready task | - |

**Implementation notes:**

- `task_ready` queries tasks where `status != done` AND no incoming `blocks` relations from non-done tasks
- Returns tasks sorted by priority (urgent > high > normal > low), then by due date
- `task_next` is a convenience wrapper that returns `task_ready(limit=1)`

### Convenience Tools

| Tool | Description | Parameters |
|------|-------------|------------|
| `task_complete` | Mark task done | `id` |
| `task_reschedule` | Change task due date | `id`, `due_date` |
| `decision_supersede` | Replace a decision | `old_id`, `new_id` |

### Entity-Specific Properties

For `entity_create`, type-specific properties are passed in a `properties` object:

```json
{
  "type": "task",
  "title": "Implement auth",
  "properties": {
    "status": "todo",
    "priority": "high",
    "due_date": "2025-02-01"
  }
}
```

**Entity type properties:**

| Type | Properties |
|------|------------|
| `decision` | `status`, `context`, `consequences[]`, `superseded_by?` |
| `task` | `status`, `priority`, `due_date?`, `assignee?` |
| `note` | `note_type?` |
| `prompt` | `template`, `variables[]`, `output_schema?` |
| `component` | `component_type`, `status`, `owner?` |
| `link` | `url`, `link_type` |

### Batch Operation Semantics

The `entity_batch` tool uses best-effort semantics:

```json
// Request
{
  "operations": [
    { "op": "create", "type": "task", "title": "Task 1", "properties": {...} },
    { "op": "update", "id": "abc123", "title": "Updated" },
    { "op": "delete", "id": "def456" }
  ]
}

// Response
{
  "results": [
    { "index": 0, "success": true, "id": "new-uuid-1" },
    { "index": 1, "success": true, "id": "abc123" },
    { "index": 2, "success": false, "error": { "code": "ENTITY_NOT_FOUND", "message": "Entity def456 not found" } }
  ],
  "succeeded": 2,
  "failed": 1
}
```

**Semantics:**

- Operations execute sequentially in array order
- Each operation sees results of previous operations (not isolated)
- On failure: Continue with remaining operations (no rollback)
- Response includes per-operation results with success/failure
- Partial success is valid — caller decides how to handle

## Input Validation

### Validation Rules

| Field | Type | Required | Constraints |
|-------|------|----------|-------------|
| `title` | string | Yes | 1-500 characters, non-empty after trim |
| `content` | string | No | Max 100KB (102,400 bytes) |
| `type` | string | Yes | One of: `decision`, `task`, `note`, `prompt`, `component`, `link` |
| `tags` | string[] | No | Each tag: 1-100 chars, max 50 tags |
| `id` | string | Varies | Sequence number (digits) or UUID prefix (min 4 hex chars) |

### Type-Specific Validation

**Decision:**

| Field | Required | Constraints |
|-------|----------|-------------|
| `status` | No (default: `proposed`) | One of: `proposed`, `accepted`, `deprecated`, `superseded` |
| `context` | No | Max 50KB |
| `consequences` | No | Array of strings, each max 1KB |
| `superseded_by` | No | Valid entity ID (if provided, status must be `superseded`) |

**Task:**

| Field | Required | Constraints |
|-------|----------|-------------|
| `status` | No (default: `todo`) | One of: `todo`, `in_progress`, `done`, `blocked` |
| `priority` | No (default: `normal`) | One of: `low`, `normal`, `high`, `urgent` |
| `due_date` | No | ISO 8601 date: `YYYY-MM-DD` |
| `assignee` | No | 1-100 characters |

**Note:**

| Field | Required | Constraints |
|-------|----------|-------------|
| `note_type` | No | 1-50 characters |

**Prompt:**

| Field | Required | Constraints |
|-------|----------|-------------|
| `template` | No | Max 50KB |
| `variables` | No | Array of strings, each 1-100 chars |
| `output_schema` | No | Valid JSON string, max 10KB |

**Component:**

| Field | Required | Constraints |
|-------|----------|-------------|
| `component_type` | No | 1-50 characters |
| `status` | No (default: `active`) | One of: `active`, `deprecated`, `planned` |
| `owner` | No | 1-100 characters |

**Link:**

| Field | Required | Constraints |
|-------|----------|-------------|
| `url` | Yes | Valid URL, max 2KB |
| `link_type` | No | 1-50 characters |

### Validation Behavior

- Validation errors return immediately (fail-fast)
- All validation errors include field name and human-readable message
- Empty strings after trim are treated as missing (for required fields)
- Unknown properties in `properties` object are ignored (forward compatibility)

## Error Handling

### Error Types

```rust
pub enum McpError {
    // Entity errors
    EntityNotFound { id: String },
    EntityTypeInvalid { provided: String, valid: Vec<String> },

    // Validation errors
    ValidationFailed { field: String, message: String },
    TitleRequired,
    TitleTooLong { max: usize, actual: usize },
    ContentTooLarge { max: usize, actual: usize },
    InvalidEnumValue { field: String, value: String, valid: Vec<String> },
    InvalidDateFormat { field: String, value: String },
    InvalidUrl { value: String },

    // Relation errors
    RelationTargetNotFound { target_id: String },
    SelfReferentialRelation { id: String },

    // Graph errors
    PathNotFound { from: String, to: String },
    MaxDepthExceeded { max: usize },

    // Resource errors
    ResourceNotFound { uri: String },
    InvalidResourceUri { uri: String },

    // Server errors
    StorageError { message: String },
    InternalError { message: String },
}
```

### MCP Error Code Mapping

| Error Type | MCP Error Code | HTTP Equivalent |
|------------|----------------|-----------------|
| `EntityNotFound` | `-32001` | 404 |
| `EntityTypeInvalid` | `-32002` | 400 |
| `ValidationFailed` | `-32003` | 400 |
| `RelationTargetNotFound` | `-32004` | 400 |
| `ResourceNotFound` | `-32005` | 404 |
| `InvalidResourceUri` | `-32006` | 400 |
| `StorageError` | `-32010` | 500 |
| `InternalError` | `-32011` | 500 |
| Parse error | `-32700` | 400 |
| Invalid request | `-32600` | 400 |
| Method not found | `-32601` | 404 |
| Invalid params | `-32602` | 400 |

### Error Response Format

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": -32001,
    "message": "Entity not found",
    "data": {
      "error_type": "EntityNotFound",
      "id": "nonexistent-id"
    }
  }
}
```

## Resources

### Response Format

All resources return JSON with MIME type `application/json`.

**Single entity response:**

```json
{
  "id": "uuid-here",
  "sequence_number": 3,
  "type": "decision",
  "title": "Use PostgreSQL",
  "content": "## Context...",
  "tags": ["database", "infrastructure"],
  "created_at": "2025-01-30T12:00:00Z",
  "updated_at": "2025-01-30T12:00:00Z",
  "properties": {
    "status": "accepted",
    "context": "We need a relational database..."
  }
}
```

**Entity list response:**

```json
{
  "entities": [...],
  "total": 42,
  "limit": 50,
  "offset": 0
}
```

**Pagination:**

- Default limit: 50
- Max limit: 100
- Use `?limit=N&offset=M` query params on URI templates

### Static Resources

| URI | Description | Returns |
|-----|-------------|---------|
| `medulla://schema` | Type definitions | JSON schema for all entity types |
| `medulla://stats` | Project statistics | Entity counts, last updated, etc. |

**Stats response:**

```json
{
  "entity_counts": {
    "decision": 15,
    "task": 42,
    "note": 8,
    "prompt": 3,
    "component": 5,
    "link": 12
  },
  "relation_count": 67,
  "last_modified": "2025-01-30T15:30:00Z",
  "medulla_version": "0.2.0"
}
```

### Dynamic Resources (URI Templates)

| URI Template | Description |
|--------------|-------------|
| `medulla://entities` | All entities |
| `medulla://entities/{type}` | Entities by type |
| `medulla://entity/{id}` | Single entity by ID |
| `medulla://decisions` | All decisions |
| `medulla://decisions/active` | Non-superseded decisions |
| `medulla://tasks` | All tasks |
| `medulla://tasks/active` | Incomplete tasks |
| `medulla://tasks/ready` | Tasks with no unresolved blockers (ready to work on) |
| `medulla://tasks/blocked` | Blocked tasks with their blockers |
| `medulla://tasks/due/{date}` | Tasks due on date |
| `medulla://prompts` | Available prompts |
| `medulla://graph` | Full knowledge graph (entities + relations) |

### Subscriptions

Per TD20, we support by-type subscriptions. When entities change, we notify subscribers of `medulla://entities/{type}`.

**Subscription state:**

- In-memory `HashMap<ResourceUri, Vec<SubscriptionId>>`
- Cleaned up on client disconnect

**Notification payload:**

```json
{
  "jsonrpc": "2.0",
  "method": "notifications/resources/updated",
  "params": {
    "uri": "medulla://entities/task"
  }
}
```

**Subscription lifecycle:**

1. Client subscribes to `medulla://entities/{type}`
2. On entity create/update/delete of that type: Send notification
3. On client disconnect: Remove all subscriptions for that client
4. Invalid URI: Return error, don't create subscription

**Deferred to Phase 3:** `medulla://context/{topic}` (semantic search)

## Logging

### Strategy

Use `tracing` crate for structured logging to stderr (stdout reserved for MCP protocol).

**Log levels:**

| Level | Use Case |
|-------|----------|
| ERROR | Operation failures, unrecoverable errors |
| WARN | Recoverable issues, validation failures |
| INFO | Request/response summary, startup/shutdown |
| DEBUG | Detailed operation flow, lock acquisition |
| TRACE | Full request/response bodies (development only) |

**Log format:**

```
2025-01-30T12:00:00.000Z INFO medulla::mcp request{id=1 method="tools/call" tool="entity_create"} started
2025-01-30T12:00:00.050Z INFO medulla::mcp request{id=1} completed duration_ms=50
```

**Configuration:**

- `MEDULLA_LOG_LEVEL` env var: `trace|debug|info|warn|error` (default: `info`)
- `RUST_LOG` also respected for fine-grained control

## Configuration

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `MEDULLA_LOG_LEVEL` | `info` | Log verbosity |
| `MEDULLA_REQUEST_TIMEOUT_MS` | `30000` | Request timeout in milliseconds |
| `MEDULLA_MAX_BATCH_SIZE` | `100` | Maximum operations per batch |

### Config File (Future)

Configuration via `.medulla/config.json` will be added in Phase 5 for non-environment settings.

## Graceful Shutdown

### Shutdown Flow

```
1. Receive SIGTERM or SIGINT
2. Log "Shutting down..."
3. Stop accepting new requests
4. Wait for in-flight requests (max 5 seconds)
5. Send close notification to subscribed clients
6. Flush any pending writes to LoroStore
7. Close SqliteCache connection
8. Exit with code 0
```

### Implementation

```rust
async fn run_server(server: MedullaServer) -> Result<()> {
    let shutdown = tokio::signal::ctrl_c();

    tokio::select! {
        _ = server.serve(stdio()) => {},
        _ = shutdown => {
            tracing::info!("Shutdown signal received");
            server.graceful_shutdown(Duration::from_secs(5)).await?;
        }
    }

    Ok(())
}
```

## CLI Integration

### Task Queue Commands (Beads Parity)

```bash
medulla tasks ready              # List tasks with no blockers, sorted by priority
medulla tasks ready --limit=5    # Limit results
medulla tasks next               # Show single highest-priority ready task
medulla tasks blocked            # List blocked tasks and what blocks them
medulla tasks blocked <id>       # Show blockers for specific task
```

**Implementation:** These commands are thin wrappers over the same query logic used by MCP tools, ensuring CLI and MCP have feature parity.

### MCP Server Commands

```
medulla serve              # stdio transport (default)
medulla serve --http 3000  # HTTP transport (Phase 5)
```

**Server startup flow:**

1. Open existing `LoroStore` (fail if not initialized)
2. Open/create `SqliteCache`, sync from Loro
3. Create `MedullaServer` with store + cache
4. Install signal handlers for graceful shutdown
5. Call `server.serve(rmcp::transport::io::stdio()).await`
6. Wait for shutdown signal

**Error handling:**

- If `.medulla/` doesn't exist: "Not a medulla project. Run `medulla init` first."
- Server logs to stderr (stdout reserved for MCP protocol)

## Testing Strategy

### Unit Tests

```
tests/unit/
├── mcp/
│   ├── tools_test.rs       # Each tool handler with mock store
│   ├── resources_test.rs   # Resource resolution and formatting
│   ├── validation_test.rs  # Input validation rules
│   └── error_test.rs       # Error code mapping
└── storage/
    └── queries_test.rs     # Task queue queries
```

**Coverage targets:**

- All tool handlers with valid inputs
- All validation error paths
- All error code mappings
- Edge cases (empty results, max limits, unicode)

### Integration Tests

```
tests/integration/
├── mcp_protocol_test.rs    # Full JSON-RPC round-trips
├── entity_crud_test.rs     # Create/read/update/delete via MCP
├── subscriptions_test.rs   # Subscribe/notify/unsubscribe
└── batch_test.rs           # Batch operations with mixed success
```

**Test scenarios:**

- Client connects, lists tools, calls tool, disconnects
- Create entity via MCP, verify via CLI
- Subscribe to type, create entity, verify notification received
- Batch with 3 creates, 1 invalid — verify 3 succeed, 1 fails

### Protocol Compliance Tests

```
tests/protocol/
├── jsonrpc_test.rs         # JSON-RPC 2.0 compliance
├── mcp_spec_test.rs        # MCP 2025-11-25 spec compliance
└── malformed_test.rs       # Malformed request handling
```

**Test scenarios:**

- Invalid JSON → parse error
- Missing `jsonrpc` field → invalid request
- Unknown method → method not found
- Missing required params → invalid params
- Batch requests (JSON-RPC arrays)

### Manual Testing

After automated tests pass:

1. Test with MCP Inspector (`npx @anthropics/mcp-inspector`)
2. Test with Claude Desktop (add to `claude_desktop_config.json`)
3. Test with Cursor (verify tool discovery and invocation)

## Backwards Compatibility

- Phase 2 works with existing Phase 1 `.medulla/` directories
- No migration needed — MCP server is additive
- Existing CLI commands unchanged
- `medulla serve` is the only new command

## Deferred Features

The following features are explicitly deferred to later phases:

| Feature | Deferred To | Rationale |
|---------|-------------|-----------|
| `search_semantic` tool | Phase 3 | Requires embedding infrastructure |
| `search_query` tool | Phase 3 | Structured query language TBD |
| `sync_snapshot` tool | Phase 4 | Depends on snapshot implementation |
| `medulla://context/{topic}` | Phase 3 | Semantic search resource |
| HTTP transport | Phase 5 | stdio sufficient for initial release |
| MCP Prompts | Future | Empty list returned; templates TBD |
| `listChanged` notifications | Future | Requires file watching |
| Rate limiting | Future | Not needed for single-user use case |

## Implementation Order

### Pre-requisites ✓ COMPLETE

All Phase 1 entity types are now implemented:

- [x] `task` entity type with properties: `status`, `priority`, `due_date?`, `assignee?`
- [x] `note` entity type with properties: `note_type?`
- [x] `prompt` entity type with properties: `template`, `variables[]`, `output_schema?`
- [x] `component` entity type with properties: `component_type`, `status`, `owner?`
- [x] `link` entity type with properties: `url`, `link_type`
- [x] CLI support for all entity types in `add`, `list`, `get`, `update`, `delete`

### Phase 2 Implementation

#### Batch 1: Foundation ✓ COMPLETE (2026-02-02)

- [x] Add dependencies (`rmcp`, `tokio`, `schemars`, `tracing`) to `Cargo.toml`
- [x] Create `src/mcp/error.rs` with `McpError` enum and MCP code mapping (-32001 to -32011)
- [x] Create `src/mcp/mod.rs` with `MedullaServer` struct, `SubscriptionState`, ping tool
- [x] Update `src/lib.rs` to export mcp module
- [x] All 36 tests pass (25 unit + 11 integration)

**Key implementation notes:**

- `SqliteCache` wrapped in `Arc<Mutex<>>` for thread safety (rusqlite Connection is not Sync)
- Using `rmcp::ErrorData` (not deprecated `rmcp::Error`)
- `ErrorCode` is a newtype wrapper: `ErrorCode(i32)`
- `Implementation::from_build_env()` used for server info

#### Batch 2: Task Queue Queries (Beads Parity) ✓ COMPLETE (2026-02-02)

- [x] Add task queue query methods to `SqliteCache`:
  - `get_ready_tasks()` - tasks with no unresolved blockers, sorted by priority/due date
  - `get_blocked_tasks()` - tasks with their blockers listed
  - `get_task_blockers(id)` - what blocks a specific task
  - `get_next_task()` - convenience method for highest-priority ready task
- [x] Export types: `ReadyTask`, `BlockedTask`, `TaskBlocker`
- [x] All 46 tests pass (35 unit + 11 integration)

**Key implementation notes:**

- Methods added directly to `SqliteCache` (not separate queries.rs) for access to private `conn` field
- SQL queries join `tasks` and `relations` tables to find blocking dependencies
- Ready tasks exclude those with non-done blockers via NOT IN subquery
- Priority ordering: urgent > high > normal > low, then due date (nulls last)

#### Batch 3: Core Entity Tools - NEXT

- [X] Implement tools in `src/mcp/tools.rs`:
  - `entity_create`, `entity_get`, `entity_list`, `entity_update`, `entity_delete`

#### Batch 4: Batch Operations + Search + Graph Tools

- [X] Implement in `src/mcp/tools.rs`:
  - `entity_batch`
  - `search_fulltext`
  - `graph_relations`, `graph_path`, `graph_orphans`

#### Batch 5: Task Queue Tools (Beads Parity) ✓ COMPLETE (2026-02-02)

- [x] Implement in `src/mcp/tools.rs`:
  - `task_ready`, `task_blocked`, `task_next` (Beads parity)
  - `task_complete`, `task_reschedule`, `decision_supersede`

**Key implementation notes:**

- Task queue tools wrap `SqliteCache` query methods (implemented in Batch 2)
- `task_blocked` supports both listing all blocked tasks and querying blockers for a specific task
- `decision_supersede` creates a `supersedes` relation and updates the old decision's status
- Added `superseded_by` field to `DecisionUpdate` struct for full supersede support
- All 52 tests pass (41 unit + 11 integration)

#### Batch 6: Resources ✓ COMPLETE (2026-02-02)

- [x] Implement resources in `src/mcp/resources.rs`:
  - Static resources: `medulla://schema`, `medulla://stats`
  - Entity resources: `medulla://entities`, `medulla://entities/{type}`, `medulla://entity/{id}`
  - Decision resources: `medulla://decisions`, `medulla://decisions/active`
  - Task resources: `medulla://tasks`, `medulla://tasks/active`, `medulla://tasks/ready`, `medulla://tasks/blocked`, `medulla://tasks/due/{date}`
  - Other resources: `medulla://prompts`, `medulla://graph`
- [x] Implement `ServerHandler` trait methods: `list_resources`, `list_resource_templates`, `read_resource`, `subscribe`, `unsubscribe`
- [x] All 72 tests pass (61 unit + 11 integration)

**Key implementation notes:**

- Resources use `medulla://` URI scheme for all resource URIs
- Static resources (schema, stats) provide metadata about the project
- Dynamic resources (entities, tasks, etc.) provide access to project data
- `ResourceContents::TextResourceContents` used with `application/json` MIME type
- `RawResource` converted to `Resource` (Annotated wrapper) using `.no_annotation()`
- Subscription state managed via `SubscriptionState` in `MedullaServer`

#### Batch 7: CLI Integration

- [ ] Add CLI commands:
  - `medulla serve` (MCP server with graceful shutdown)
  - `medulla tasks ready`, `medulla tasks next`, `medulla tasks blocked`

#### Batch 8: Testing

- [ ] Write unit tests for all tools and error paths
- [ ] Write integration tests for MCP protocol
- [ ] Test with Claude Code / MCP Inspector

## Validation Checklist

From PRD:
> After Phase 2: Can Claude Code query and create decisions via MCP?

**Specific test cases:**

- [ ] `medulla serve` starts without error on initialized project
- [ ] MCP Inspector connects and lists all tools
- [ ] Claude Desktop connects via stdio transport
- [ ] `entity_create` creates decision, visible in `medulla list decisions`
- [ ] `entity_list` with `type=decision` returns all decisions
- [ ] `entity_get` retrieves specific decision by sequence number
- [ ] `entity_get` retrieves specific decision by UUID prefix
- [ ] `entity_update` modifies decision title
- [ ] `entity_delete` removes decision
- [ ] `entity_batch` with mixed operations reports per-operation results
- [ ] `search_fulltext` finds decision by keyword in title
- [ ] `search_fulltext` finds decision by keyword in content
- [ ] `graph_relations` shows task→decision relation
- [ ] `graph_path` finds path between related entities
- [ ] `graph_orphans` returns entities with no relations
- [ ] `task_ready` returns tasks without blockers, sorted by priority
- [ ] `task_next` returns single highest-priority ready task
- [ ] `task_blocked` shows blocked tasks with their blockers
- [ ] Resource `medulla://decisions` returns decision list
- [ ] Resource `medulla://entity/{id}` returns single entity
- [ ] Subscription to `medulla://entities/task` notifies on task create
- [ ] Invalid tool params return validation error with field name
- [ ] Entity not found returns -32001 error code
- [ ] Graceful shutdown completes in-flight requests
- [ ] Server logs to stderr, not stdout

## References

- [MCP Specification 2025-11-25](https://modelcontextprotocol.io/specification/2025-11-25)
- [rmcp crate](https://crates.io/crates/rmcp)
- [Shuttle MCP Server Tutorial](https://www.shuttle.dev/blog/2025/07/18/how-to-build-a-stdio-mcp-server-in-rust)
- [JSON-RPC 2.0 Specification](https://www.jsonrpc.org/specification)
