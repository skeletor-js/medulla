# Phase 2: MCP Server Design

**Date**: 2025-01-31
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

## Dependencies

```toml
# MCP Server
rmcp = { version = "0.14", features = ["server", "macros", "transport-io"] }
tokio = { version = "1", features = ["rt-multi-thread", "macros", "io-std"] }
schemars = "0.8"  # For JSON schema generation on tool params
```

## Module Structure

```
src/
├── mcp/
│   ├── mod.rs           # MCP module exports, MedullaServer struct
│   ├── tools.rs         # Tool implementations (entity_*, search_*)
│   └── resources.rs     # Resource implementations (medulla://*)
├── lib.rs               # Add `pub mod mcp;`
└── main.rs              # Add `serve` subcommand
```

**MedullaServer struct:**
```rust
#[derive(Clone)]
pub struct MedullaServer {
    store: Arc<Mutex<LoroStore>>,
    cache: Arc<SqliteCache>,
    tool_router: ToolRouter<Self>,
}
```

## Tools

### Core Entity Tools

| Tool | Description | Parameters |
|------|-------------|------------|
| `entity_create` | Create any entity type | `type`, `title`, `content?`, `tags?`, `properties` (type-specific) |
| `entity_get` | Get by ID | `id` (seq number or UUID prefix) |
| `entity_list` | List with filters | `type?`, `status?`, `tag?`, `limit?` |
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

## Resources

### Static Resources

| URI | Description | Returns |
|-----|-------------|---------|
| `medulla://schema` | Type definitions | JSON schema for all entity types |
| `medulla://stats` | Project statistics | Entity counts, last updated, etc. |

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

**Deferred to Phase 3:** `medulla://context/{topic}` (semantic search)

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
4. Call `server.serve(rmcp::transport::io::stdio()).await`
5. Wait for shutdown signal

**Error handling:**
- If `.medulla/` doesn't exist: "Not a medulla project. Run `medulla init` first."
- Server logs to stderr (stdout reserved for MCP protocol)

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

1. Add dependencies (`rmcp`, `tokio`, `schemars`) to `Cargo.toml`
2. Create `src/mcp/mod.rs` with `MedullaServer` struct
3. Implement core query functions in `src/storage/queries.rs`:
   - `get_ready_tasks()` - tasks with no unresolved blockers
   - `get_blocked_tasks()` - tasks with their blockers
   - `get_task_blockers(id)` - what blocks a specific task
4. Implement tools in `src/mcp/tools.rs`:
   - `entity_create`, `entity_get`, `entity_list`, `entity_update`, `entity_delete`
   - `entity_batch`
   - `search_fulltext`
   - `graph_relations`
   - `task_ready`, `task_blocked`, `task_next` (Beads parity)
   - `task_complete`, `task_reschedule`, `decision_supersede`
5. Implement resources in `src/mcp/resources.rs`
6. Add CLI commands:
   - `medulla serve` (MCP server)
   - `medulla tasks ready`, `medulla tasks next`, `medulla tasks blocked`
7. Test with Claude Desktop / MCP Inspector

## Validation Milestone

From PRD:
> After Phase 2: Can Claude Desktop query and create decisions via MCP?

## References

- [MCP Specification 2025-11-25](https://modelcontextprotocol.io/specification/2025-11-25)
- [rmcp crate](https://crates.io/crates/rmcp)
- [Shuttle MCP Server Tutorial](https://www.shuttle.dev/blog/2025/07/18/how-to-build-a-stdio-mcp-server-in-rust)
