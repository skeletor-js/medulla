# Phase 2: MCP Server Design

**Date**: 2025-01-31
**Status**: Approved

## Overview

Implement an MCP (Model Context Protocol) server that exposes Medulla's knowledge engine to AI tools like Claude Desktop, Cursor, and Copilot via the standard MCP protocol.

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
| `medulla://tasks/due/{date}` | Tasks due on date |
| `medulla://prompts` | Available prompts |
| `medulla://graph` | Full knowledge graph (entities + relations) |

### Subscriptions

Per TD20, we support by-type subscriptions. When entities change, we notify subscribers of `medulla://entities/{type}`.

**Deferred to Phase 3:** `medulla://context/{topic}` (semantic search)

## CLI Integration

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

### Pre-requisites (Remaining Phase 1 Work)

1. Implement `task` entity type with properties: `status`, `priority`, `due_date?`, `assignee?`
2. Implement `note` entity type with properties: `note_type?`
3. Implement `prompt` entity type with properties: `template`, `variables[]`, `output_schema?`
4. Implement `component` entity type with properties: `component_type`, `status`, `owner?`
5. Implement `link` entity type with properties: `url`, `link_type`
6. Update CLI to support all entity types in `add`, `list`, `get`, `update`, `delete`

### Phase 2 Implementation

1. Add dependencies (`rmcp`, `tokio`, `schemars`) to `Cargo.toml`
2. Create `src/mcp/mod.rs` with `MedullaServer` struct
3. Implement tools in `src/mcp/tools.rs`:
   - `entity_create`, `entity_get`, `entity_list`, `entity_update`, `entity_delete`
   - `entity_batch`
   - `search_fulltext`
   - `graph_relations`
   - `task_complete`, `task_reschedule`, `decision_supersede`
4. Implement resources in `src/mcp/resources.rs`
5. Add `serve` command to CLI
6. Test with Claude Desktop / MCP Inspector

## Validation Milestone

From PRD:
> After Phase 2: Can Claude Desktop query and create decisions via MCP?

## References

- [MCP Specification 2025-11-25](https://modelcontextprotocol.io/specification/2025-11-25)
- [rmcp crate](https://crates.io/crates/rmcp)
- [Shuttle MCP Server Tutorial](https://www.shuttle.dev/blog/2025/07/18/how-to-build-a-stdio-mcp-server-in-rust)
