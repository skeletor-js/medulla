# Medulla: Project Context Engine

> A git-native, AI-accessible knowledge engine for software projects.

**Status**: PRD Complete — Ready for Development
**Date**: 2025-01-30
**Authors**: Jordan Stella, Claude
**Website**: [medulla.cc](https://medulla.cc)

---

## Executive Summary

Medulla is a **project-scoped context engine** that lives in your git repository. It gives AI tools (Claude Code, Cursor, Copilot, etc.) structured access to project knowledge—decisions, tasks, notes, prompts—via the Model Context Protocol (MCP).

Unlike static files like `CLAUDE.md` or `.cursorrules`, Medulla provides:

- **Queryable data**: "What did we decide about authentication?"
- **Dynamic updates**: Context evolves as the project evolves
- **Structured types**: Decisions, tasks, prompts with schemas
- **Conflict-free sync**: CRDT-based, merges cleanly across branches

### Target Users

- **Solo developers**: Persistent project memory across AI sessions
- **Small teams**: Shared context via git, everyone's AI sees the same knowledge
- **OSS maintainers**: Onboard contributors faster with rich, queryable context

### Success Criteria

- AI tools can read/write project decisions via MCP
- Data survives git merges without conflicts
- Human-readable snapshot is browsable on GitHub
- Works offline with local embeddings
- Zero required API keys for core functionality

---

## Problem Statement

### What Exists Today

| Solution | Format | Queryable? | Dynamic? | Git-Native? |
|----------|--------|------------|----------|-------------|
| CLAUDE.md / AGENTS.md | Static markdown | As raw text | No | Yes |
| ADRs (Log4brains) | Markdown files | No MCP | No | Yes |
| GitHub Issues | API-only | Via MCP | Yes | No |
| Notion/Obsidian | Proprietary | Varies | Yes | No |

### The Gap

Nobody has built a **queryable, git-native, project-scoped knowledge engine** with first-class MCP support.

### Pain Points

1. **Context loss**: AI assistants forget project decisions between sessions
2. **Static files**: `CLAUDE.md` can't be queried or filtered
3. **Merge conflicts**: Markdown files conflict when edited on multiple branches
4. **External dependencies**: GitHub Issues requires API access, Notion requires accounts
5. **No structure**: Freeform text lacks schema for reliable AI parsing

### Motivating Example: Building Medulla

During the design of Medulla itself, the following decisions were made in conversation with an AI assistant:

- **Rust over TypeScript**: Debated tradeoffs (iteration speed vs distribution, Loro support)
- **Loro over Automerge/Yjs**: Compared maturity, data model fit, relation handling
- **CRDT is non-negotiable**: Established that relations must merge cleanly across branches

These decisions were manually transcribed into `prd.md`. Without that manual effort:

- The next AI session would have no idea these discussions happened
- Rationale and alternatives considered would be lost
- Future contributors couldn't query "why Rust?" and get context

With Medulla, these would be captured as queryable decisions:

```bash
medulla add decision "Use Rust over TypeScript" \
  --status=accepted \
  --tag=architecture \
  --context="Single binary distribution, first-class Loro support, compile-time safety"

medulla add decision "Use Loro for CRDT" \
  --status=accepted \
  --tag=architecture \
  --context="Compared Automerge (mature but less purpose-built) and Yjs (text-editing focused)"
```

Future sessions could then query: `medulla search "why Rust"` → full context with rationale.

---

## Solution Overview

Medulla stores project knowledge in a **Loro CRDT** (conflict-free replicated data type) that merges cleanly across git branches. It exposes this data via **MCP** for AI tools and auto-generates a **human-readable markdown snapshot** for GitHub browsing.

### Unique Value Proposition

**"Your project's brain, accessible to any AI tool, synced via git."**

---

## Architecture

### Storage Model: CRDT + Markdown Snapshot

```
.medulla/
  loro.db              # CRDT source of truth (binary, git-tracked)
  schema.json          # Type definitions (human-readable)
  config.json          # Project configuration
  cache.db             # SQLite for FTS + embeddings (gitignored)
  snapshot/            # Auto-generated on commit (derived, read-only)
    README.md          # Auto-generated index
    decisions/
      001-use-postgres.md
      002-auth-with-jwt.md
    tasks/
      active.md        # All non-completed tasks
      completed.md     # Archived completed tasks
    notes/
      <slug>.md
    prompts/
      <slug>.md
```

### Design Rationale

| Approach | Conflict-free | Human-readable | GitHub browsable |
|----------|---------------|----------------|------------------|
| Markdown files only | No | Yes | Yes |
| SQLite only | No | No | No |
| CRDT only | Yes | No | No |
| **CRDT + Snapshot** | **Yes** | **Derived** | **Yes** |

### Key Principles

1. **Loro CRDT** is the source of truth for conflict-free merging
2. **Markdown snapshot** is auto-generated on `git commit` via hook
3. **Snapshot is read-only** — manual edits are overwritten on next commit
4. **cache.db** is gitignored (derived data for search/embeddings)
5. **Headless-first** — all operations work non-interactively

---

## Data Model

### Base Entity

All entities share this structure:

```typescript
interface Entity {
  id: string;                    // UUID
  type: string;                  // "decision", "task", "note", etc.
  title: string;                 // Required
  content?: string;              // Markdown body
  tags: string[];                // Freeform tags
  relations: Relation[];         // Links to other entities
  created_at: DateTime;
  updated_at: DateTime;
  created_by?: string;           // Git author (with fallback)
  properties: Record<string, PropertyValue>;
}

interface Relation {
  target_id: string;
  relation_type: string;         // "blocks", "implements", "supersedes"
  properties?: Record<string, PropertyValue>;
}

type PropertyValue = string | number | boolean | DateTime | string[];
```

### Built-in Entity Types

| Type | Purpose | Key Properties |
|------|---------|----------------|
| **decision** | Architectural decisions (ADRs) | `status`, `superseded_by?`, `context`, `consequences[]` |
| **task** | Work items | `status`, `priority`, `due_date?`, `assignee?` |
| **note** | Freeform project notes | `note_type?` |
| **prompt** | AI prompt templates | `template`, `variables[]`, `output_schema?` |
| **component** | System components | `component_type`, `status`, `owner?` |
| **link** | External resources | `url`, `link_type` |

### Built-in Relation Types

| Relation | From → To | Meaning |
|----------|-----------|---------|
| `implements` | task → decision | Task implements this decision |
| `blocks` | task → task | Blocking dependency |
| `supersedes` | decision → decision | New decision replaces old |
| `references` | any → any | General reference |
| `belongs_to` | task → component | Task is for this component |
| `documents` | note → component | Note documents this component |

### Custom Types

Users can extend the schema in `schema.json`:

```json
{
  "types": {
    "experiment": {
      "properties": {
        "hypothesis": { "type": "text", "required": true },
        "status": { "type": "select", "options": ["proposed", "running", "concluded"] },
        "result": { "type": "select", "options": ["validated", "invalidated", "inconclusive"] }
      }
    }
  }
}
```

---

## Loro Schema Design

This section defines how the data model maps to Loro's CRDT primitives. These decisions shape merge behavior, query patterns, and the overall storage architecture.

### Root Structure

The Loro document uses a **nested-by-type** structure. Each entity type gets its own top-level `LoroMap`, making type-based queries a direct read and keeping merge scope narrow.

```text
root (LoroMap)
├── decisions (LoroMap<uuid, Entity>)
├── tasks (LoroMap<uuid, Entity>)
├── notes (LoroMap<uuid, Entity>)
├── prompts (LoroMap<uuid, Entity>)
├── components (LoroMap<uuid, Entity>)
├── links (LoroMap<uuid, Entity>)
├── <custom_types> (LoroMap<uuid, Entity>)  # e.g., experiments
├── relations (LoroMap<composite_key, Relation>)
└── _meta (LoroMap)
    ├── schema_version: "1.0"
    ├── created_at: "<iso>"
    ├── medulla_version: "0.1.0"
    └── type_sequences (LoroMap<type, number>)
```

**Design rationale:**

| Decision | Rationale |
|----------|-----------|
| Nested by type | Type-based listing is a direct map iteration; snapshot generation mirrors structure |
| Custom types at top level | First-class treatment; uniform code paths for built-in and custom types |
| Separate relations collection | Single source of truth; avoids CRDT sync issues with bidirectional storage |
| `_meta` for bookkeeping | Schema version for migrations; sequence counters for human-readable IDs |

### Entity Structure

Each entity is a `LoroMap` within its type's collection:

```text
<uuid> (LoroMap)
├── id: "<uuid>"
├── type: "decision"
├── sequence_number: 3              # For human display (003-...)
├── title: "Use PostgreSQL"
├── content: "## Context\n..."      # Plain string
├── tags (LoroList<string>)
├── created_at: "<iso>"
├── updated_at: "<iso>"
├── created_by: "jordan"
└── properties (LoroMap)
    ├── <scalar_prop>: value
    └── <array_prop> (LoroList<string>)
```

**Field type decisions:**

| Field | Loro Type | Rationale |
|-------|-----------|-----------|
| `tags` | `LoroList<string>` | Concurrent tag additions interleave cleanly on merge |
| `content` | Plain string | Not `LoroText`—changes are branch-level, not character-level; last-writer-wins is acceptable |
| `properties` | `LoroMap` | Type-specific fields; extensible per entity type |
| Array properties | `LoroList<string>` | e.g., `consequences[]` merges concurrent additions |
| Timestamps | ISO 8601 strings | Human-readable, no timezone ambiguity, debuggable |

### Entity IDs: Hybrid Approach

Entities use both a UUID (stable identifier) and a sequence number (human display):

- **`id`**: UUID, never changes, used in relations and internally
- **`sequence_number`**: Type-specific counter for human-friendly references (e.g., `003-use-postgres.md`)

**Merge-time reconciliation:**

1. Scan all entities of each type
2. Sort by `created_at` timestamp
3. Detect sequence number gaps or duplicates
4. Reassign sequence numbers to maintain clean 1, 2, 3... progression
5. UUIDs remain stable; only display numbers shift

The CLI accepts both: `medulla get 003` or `medulla get a1b2c3d` (UUID prefix).

### Relation Structure

Relations live in a dedicated top-level collection with composite keys:

```text
relations (LoroMap)
└── "<source_id>:<relation_type>:<target_id>" (LoroMap)
    ├── source_id: "<uuid>"
    ├── source_type: "task"         # Denormalized for query filtering
    ├── target_id: "<uuid>"
    ├── target_type: "task"         # Denormalized
    ├── relation_type: "blocks"
    ├── created_at: "<iso>"
    ├── created_by: "jordan"
    └── properties (LoroMap)        # Optional metadata
```

**Key design decisions:**

| Decision | Rationale |
|----------|-----------|
| Composite key `source:type:target` | Enforces one relation per type between entities; direct existence lookup |
| Separate collection (not on entities) | Single source of truth; no bidirectional sync issues |
| Denormalized `source_type`/`target_type` | Enables cache indexing without entity lookups |
| Properties map | Relations can carry metadata (e.g., "blocked since", "implementation notes") |

**Why not bidirectional storage?**

CRDTs don't support cross-container transactions. Storing relations on both source and target entities would risk inconsistent state after merge (one side updated, other not). A single collection eliminates this class of bugs. The SQLite cache builds bidirectional indexes for fast queries in both directions.

### Query Patterns

| Query | Implementation |
|-------|----------------|
| Get entity by ID | `root.get(type).get(id)` — direct lookup |
| List entities by type | `root.get(type).entries()` — iterate map |
| Get relations from entity | Cache index `relations_by_source` |
| Get relations to entity | Cache index `relations_by_target` |
| Full-text search | SQLite FTS5 on `cache.db` |
| Semantic search | fastembed vectors in `cache.db` |

The Loro layer is the **source of truth**; the SQLite cache is a **derived query accelerator** that can always be rebuilt.

---

## CLI Interface

### Design Principles

- **Headless-first**: No interactive editors, everything works non-interactively
- **Git-style**: Familiar subcommand pattern (`medulla add`, `medulla list`)
- **Scriptable**: Stdin + flags for all operations, JSON output option

### Commands

```bash
# Initialization
medulla init                              # Create .medulla/ (prompts for options)
medulla init --yes                        # Accept all optional features
medulla init --no                         # Decline all optional features
medulla init --hook=yes                   # Explicit per-feature

# Entity operations (id accepts sequence number "3" or UUID prefix "a1b2c")
medulla add <type> "<title>" [--flags]    # Create entity
medulla list [type] [--filters]           # List entities
medulla get <id>                          # Get single entity
medulla update <id> [--flags]             # Update entity
medulla delete <id>                       # Delete entity

# Search
medulla search "<query>"                  # Full-text search
medulla search --semantic "<query>"       # Vector similarity

# Git integration
medulla snapshot                          # Generate markdown snapshot
medulla hook install                      # Install pre-commit hook
medulla hook uninstall                    # Remove pre-commit hook

# MCP Server
medulla serve                             # MCP server (stdio, default)
medulla serve --http 3000                 # MCP server (HTTP)
```

### Input Handling

```bash
# Inline flags
medulla add decision "Use Postgres" --status=accepted --tag=database

# Relations (generic syntax for all types)
medulla add task "Implement auth" --relation="implements:abc123" --relation="blocks:def456"

# Piped content (for longer bodies)
echo "## Context\n..." | medulla add decision "Use Postgres" --stdin

# Interactive editing (opens $EDITOR with template)
medulla add decision "Use Postgres" --edit

# JSON for complex entities
medulla add decision --json='{"title":"...","context":"..."}'
```

### Output Modes

- **Human-readable** (default): Formatted for terminal
- **JSON** (`--json`): Machine-parseable output
- **Quiet** (`--quiet`): Exit codes only, for scripts

### Interactive Prompts Pattern

For optional features during init (and future commands):

- If stdin is a tty → prompt with y/n
- If `--yes` flag → enable all optional features
- If `--no` flag → disable all optional features
- Per-feature flags override (e.g., `--hook=yes`)

---

## Snapshot Format

### Decision File Example (`001-use-postgres.md`)

```markdown
---
id: abc123
title: Use PostgreSQL for primary database
status: accepted
created: 2025-01-30
updated: 2025-01-30
tags: [database, infrastructure]
supersedes: null
---

## Context

We need a relational database for user data...

## Decision

We will use PostgreSQL 16...

## Consequences

- Team needs PostgreSQL expertise
- Enables advanced querying features
```

### Tasks File Example (`active.md`)

```markdown
# Active Tasks

## High Priority

- [ ] **Implement auth flow** (abc123)
  Due: 2025-02-01 | Tags: auth, security

## Normal Priority

- [ ] **Write API docs** (def456)
  Tags: documentation
```

### Format Rules

- **Decisions**: Numbered individual files (ADR style), YAML frontmatter + markdown body
- **Tasks**: Grouped by status/priority in consolidated files
- **Notes/Prompts**: Individual files with slug-based names
- **README.md**: Auto-generated index with links to all entities

---

## MCP Interface

Compliant with [MCP Specification 2025-11-25](https://modelcontextprotocol.io/specification/2025-11-25).

### Server Capabilities

```json
{
  "protocolVersion": "2025-11-25",
  "capabilities": {
    "tools": { "listChanged": true },
    "resources": { "subscribe": true, "listChanged": true },
    "prompts": { "listChanged": true },
    "logging": {},
    "completions": {}
  }
}
```

### Transport Modes

| Mode | Command | Use Case |
|------|---------|----------|
| **stdio** (default) | `medulla serve` | Claude Desktop, Cursor, local AI tools |
| **HTTP** | `medulla serve --http 3000` | Web UIs, remote clients |

### Tools

| Tool | Description |
|------|-------------|
| `entity_create` | Create any entity type |
| `entity_update` | Update entity |
| `entity_delete` | Delete entity |
| `entity_get` | Get single entity |
| `entity_list` | List with filters |
| `entity_batch` | Batch operations (best-effort) |
| `search_fulltext` | Keyword search |
| `search_semantic` | Vector similarity search |
| `search_query` | Structured query |
| `graph_relations` | Get entity relations |
| `graph_path` | Find path between entities |
| `graph_orphans` | Find unconnected entities |
| `task_complete` | Mark task done |
| `task_reschedule` | Change due date |
| `decision_supersede` | Replace a decision |
| `sync_snapshot` | Generate markdown snapshot |

### Resources (URI Templates)

| Resource | Description |
|----------|-------------|
| `medulla://schema` | Type definitions |
| `medulla://entities` | All entities |
| `medulla://entities/{type}` | Entities by type |
| `medulla://entity/{id}` | Single entity |
| `medulla://decisions` | All decisions |
| `medulla://decisions/active` | Non-superseded decisions |
| `medulla://tasks` | All tasks |
| `medulla://tasks/active` | Incomplete tasks |
| `medulla://tasks/due/{date}` | Tasks due on date |
| `medulla://prompts` | Available prompts |
| `medulla://context/{topic}` | Semantic search for topic |
| `medulla://graph` | Full knowledge graph |
| `medulla://stats` | Project statistics |

### Prompts (Future)

| Prompt | Description |
|--------|-------------|
| `project_summary` | Current project state overview |
| `decision_review` | Evaluate proposal against existing decisions |
| `task_breakdown` | Break goal into actionable tasks |
| `onboard_contributor` | Generate onboarding context |
| `code_review` | Project-aware code review |
| `find_related` | Find all related project knowledge |
| `daily_standup` | Generate standup from recent activity |

---

## Search

### Full-Text Search

- **Implementation**: SQLite FTS5 on title + content
- **Filters**: `type:decision`, `status:active`, `tag:auth`, `created:>2025-01-01`

### Semantic Search

- **Default provider**: fastembed (local, no API key required)
- **Optional provider**: OpenAI embeddings (via `OPENAI_API_KEY` env var)
- **Embedding timing**: Computed eagerly on add/update, stored in `cache.db`
- **Default model**: `all-MiniLM-L6-v2` (configurable in `config.json`)

### Configuration

```json
{
  "version": "1.0",
  "embeddings": {
    "provider": "local",
    "model": "all-MiniLM-L6-v2"
  }
}
```

---

## Technical Decisions

### TD1: GitHub Issues Sync

**Decision**: Not included in initial release. GitHub Issues sync is a future roadmap item.

**Rationale**:

- Medulla should own the data for AI-native workflows
- Reduces initial complexity
- GitHub Issues is a migration path, not a core dependency

### TD2: Git Authorship

**Decision**: Use git author with fallback chain.

**Implementation**:

1. Check `git config user.name`
2. Fall back to `.medulla/config.json`
3. If neither, leave as `null`

### TD3: Snapshot Editing

**Decision**: Ignore manual edits to snapshot — next commit overwrites.

**Rationale**: Keep it simple. Snapshot is a view, not a source.

### TD4: CLI Style

**Decision**: Git-style subcommands with stdin + flags for headless operation.

**Rationale**:

- Familiar to developers (`medulla add` like `git add`)
- Fully scriptable for CI/CD and AI assistants
- No interactive editors (headless-first)

### TD5: Initialization Flow

**Decision**: Minimal bootstrap with y/n prompts for optional features.

**Implementation**:

- Creates `.medulla/` with empty `loro.db`, default `config.json`, built-in `schema.json`
- Prompts "Install git hook? (y/n)" if tty detected
- `--yes`/`--no` flags for non-interactive mode
- `medulla hook install/uninstall` available for later changes

### TD6: Snapshot Format

**Decision**: Flat by type with YAML frontmatter + markdown body.

**Structure**:

- Decisions: Numbered individual files (ADR style)
- Tasks: Grouped by status in consolidated files
- Notes/Prompts: Individual files with slug names

### TD7: Semantic Search

**Decision**: Local-first (fastembed) with optional OpenAI.

**Implementation**:

- Default: fastembed runs locally, no API key needed
- Optional: `OPENAI_API_KEY` enables OpenAI embeddings
- Embeddings computed eagerly on add/update (not lazy)

### TD8: Transport Modes

**Decision**: Both stdio (default) and HTTP (`--http` flag).

**Rationale**:

- stdio for Claude Desktop, Cursor, and local AI tools
- HTTP for web UIs and remote clients
- Default to stdio for simplest local setup

### TD9: Encryption

**Decision**: No encryption in initial release. Rely on repo-level access control.

**Rationale**:

- Project context typically isn't more sensitive than code
- Private repos already gate access
- Encryption adds friction; add later if demand emerges

### TD10: Multi-repo Support

**Decision**: Single repo only. Monorepo root works fine.

**Rationale**:

- Each project should own its context
- `.medulla/` at monorepo root covers multi-package scenarios
- Cross-repo federation adds sync/staleness complexity

### TD11: Language Choice

**Decision**: Rust over TypeScript.

**Rationale**:

- Single binary distribution (~5MB vs ~50-80MB with Bun compile)
- Loro has first-class Rust support (JS bindings are second-class)
- Compile-time safety prevents runtime surprises
- Performance matters for embeddings and large knowledge graphs
- Building for longevity over iteration speed

**Alternatives Considered**:

- TypeScript/Bun: Faster iteration, larger MCP ecosystem to copy from, but worse distribution story and second-class Loro support

### TD12: CRDT Library Choice

**Decision**: Loro over Automerge or Yjs.

**Rationale**:

- Best-in-class for rich document structures with relations
- Native Tree type fits entity graph model
- First-class Rust implementation
- Designed for structured data, not just collaborative text editing

**Alternatives Considered**:

- Automerge: Mature, good Rust/JS parity, but less purpose-built for document-with-relations model
- Yjs: Most battle-tested, but designed for collaborative text editing first—graph/relation modeling is clunky

### TD13: CRDT Necessity

**Decision**: CRDT is required, not optional.

**Rationale**:

- Relations between entities must merge cleanly across branches
- Example: `task A blocks B` + `task A blocks C` on different branches → both preserved
- Conflict-free updates (not just creates) are essential for living documentation
- One-file-per-entity (ADR style) would give conflict-free creates but not conflict-free updates or atomic relation management

### TD14: Git Hook Behavior

**Decision**: Pre-commit hook with fast-path, abort on failure with `--no-verify` escape hatch.

**Implementation**:

- Hook checks `git diff --cached --name-only | grep -q '.medulla/loro.db'`
- If `loro.db` not staged, skip snapshot generation entirely
- If staged, run `medulla snapshot`; non-zero exit aborts commit
- Error message reminds users about `--no-verify` for emergencies

**Rationale**:

- Pre-commit guarantees snapshot is always in sync with CRDT data
- Fast-path avoids penalizing commits that don't touch Medulla
- `--no-verify` is standard git convention for emergency bypass

### TD15: CLI Relation Input

**Decision**: Generic `--relation="type:target"` flag for all relation types.

**Implementation**:

```bash
medulla add task "Implement auth" --relation="implements:abc123" --relation="blocks:def456"
```

**Rationale**:

- One consistent pattern for built-in and custom relation types
- Typed flags (`--blocks`, `--implements`) don't scale to custom relations
- Multiple `--relation` flags allow multiple relations in one command

### TD16: CLI Content Input

**Decision**: Stdin by default, `--edit` flag for interactive editing via `$EDITOR`.

**Implementation**:

```bash
# Piped content (headless)
echo "## Context..." | medulla add decision "Use Postgres" --stdin

# Interactive editing
medulla add decision "Use Postgres" --edit  # Opens $EDITOR with template
```

**Rationale**:

- Maintains headless-first principle (stdin works in scripts/CI)
- `--edit` is opt-in for humans who want a better editing experience
- Follows `git commit -e` convention

### TD17: CLI ID Input and Display

**Decision**: Accept both sequence numbers and UUID prefixes; display as `003 (a1b2c3d)`.

**Implementation**:

- Input: `medulla get 3`, `medulla get 003`, `medulla get a1b2c` all work
- Resolution order: try sequence number first (within type context), then UUID prefix
- Display: `003 (a1b2c3d)` — sequence prominent, UUID fragment for disambiguation

**Rationale**:

- Sequence numbers are ergonomic for quick interactive use
- UUID prefixes are stable across merges for scripts and cross-branch references
- Hybrid display makes both visible without clutter

### TD18: MCP Tool Naming

**Decision**: Snake case with logical grouping, no global prefix.

**Implementation**:

- `entity_create`, `entity_update`, `entity_delete`, `entity_get`, `entity_list`
- `search_fulltext`, `search_semantic`, `search_query`
- `graph_relations`, `graph_path`, `graph_orphans`
- `task_complete`, `task_reschedule`, `decision_supersede`

**Rationale**:

- MCP already namespaces by server, so `medulla_` prefix is redundant
- Snake case is readable and follows common conventions
- Logical grouping (`entity_*`, `search_*`) aids discoverability

### TD19: MCP Batch Operations

**Decision**: Best-effort execution with per-operation success/failure reporting.

**Implementation**:

```json
{
  "results": [
    { "id": "abc123", "success": true },
    { "id": "def456", "success": false, "error": "Validation failed: title required" }
  ]
}
```

**Rationale**:

- For a knowledge base, partial success is usually acceptable
- One bad item shouldn't block the entire batch
- Clear reporting lets caller handle failures appropriately

### TD20: MCP Subscription Granularity

**Decision**: By-type subscriptions (e.g., `medulla://decisions`, `medulla://tasks`).

**Rationale**:

- Hits the sweet spot for typical use cases ("watch for new decisions")
- More useful than all-or-nothing (too noisy) or per-entity (too granular)
- Entity-level subscriptions can be added later if demand emerges

### TD21: Performance Warnings

**Decision**: Soft warnings at thresholds, no hard limits.

**Implementation**:

- Warn at ~1,000 entities or ~10MB `loro.db`
- Warning message explains potential performance impact
- No blocking—users can continue if they choose

**Rationale**:

- Most projects won't hit thresholds
- Informed users can make their own tradeoffs
- Thresholds can be tuned based on real-world performance data

### TD22: Embedding Computation

**Decision**: Immediate computation on each add/update.

**Implementation**:

- Compute embedding synchronously when entity is created or content updated
- CLI shows brief "indexing..." indicator during computation
- ~50-200ms latency per write (fastembed local)

**Rationale**:

- Keeps system simple—embeddings always fresh
- Acceptable latency for local knowledge base use case
- No background job complexity or stale embedding risk

### TD23: Cache Invalidation

**Decision**: Incremental sync with version tracking.

**Implementation**:

- Store "last synced" Loro version/hash in `cache.db`
- On startup, compare to current Loro state
- Reindex only entities that changed since last sync
- `medulla cache rebuild` available for manual full rebuild

**Rationale**:

- Standard pattern for derived indexes
- Scales well as knowledge base grows
- Full rebuild available as escape hatch if something goes wrong

---

## Technology Stack

| Component | Technology | Rationale |
|-----------|------------|-----------|
| Language | Rust | Performance, safety, single binary distribution |
| CRDT | Loro | Best-in-class CRDT with rich data types |
| Cache/FTS | SQLite + FTS5 | Proven, embedded, fast full-text search |
| Embeddings | fastembed | Local, no API key, good quality |
| CLI | clap | Standard Rust CLI framework |
| MCP Server | axum | Async Rust web framework |
| Serialization | serde | Standard Rust serialization |

---

## Implementation Roadmap

### Phase 1: Core Engine

- [x] Scaffold Rust project (cargo workspace, dependencies, module structure)
- [x] Loro CRDT storage layer with nested-by-type structure
- [x] Entity model with built-in types (decision only — task, note, prompt, component, link coming later)
- [ ] Relation storage in separate collection with composite keys
- [x] Hybrid ID system (UUID + sequence numbers, prefix matching)
- [ ] SQLite cache with version tracking for incremental sync
- [x] Basic CLI commands:
  - [x] `init` with `--yes`/`--no` flags (y/n prompts deferred to Phase 4)
  - [x] `add` with `--stdin` flag (`--relation` and `--edit` show warnings, deferred)
  - [x] `list`, `get` (accept sequence number or UUID prefix)
  - [ ] `update`, `delete`
- [x] JSON output mode (`--json`)
- [x] Human-readable output with `003 (a1b2c3d)` ID format

**Phase 1 Progress:** Vertical slice complete (init, add decision, list, get). 9 commits, 4 unit tests + 5 integration tests passing. Release binary: 3.1MB.

### Phase 2: MCP Server

- [ ] MCP 2025-11-25 protocol compliance
- [ ] stdio transport (default for Claude Desktop, Cursor)
- [ ] Entity tools: `entity_create`, `entity_update`, `entity_delete`, `entity_get`, `entity_list`
- [ ] Batch operations: `entity_batch` with best-effort semantics, per-operation results
- [ ] Resources with by-type subscriptions (`medulla://decisions`, `medulla://tasks`)
- [ ] Graph tools: `graph_relations`, `graph_path`, `graph_orphans`
- [ ] Convenience tools: `task_complete`, `task_reschedule`, `decision_supersede`

### Phase 3: Search & Graph

- [ ] Full-text search via SQLite FTS5 (`search_fulltext`)
- [ ] Semantic search via fastembed (`search_semantic`)
- [ ] Immediate embedding computation on add/update (~50-200ms)
- [ ] Search filters (`type:`, `status:`, `tag:`, `created:`)
- [ ] Structured queries (`search_query`)
- [ ] Soft warnings at performance thresholds (~1,000 entities, ~10MB loro.db)

### Phase 4: Snapshot & Git Integration

- [ ] Markdown snapshot generation (`medulla snapshot`)
- [ ] Pre-commit hook with fast-path (only runs if `loro.db` staged via `git diff --cached`)
- [ ] Hook failure handling (abort commit, message suggests `--no-verify` escape)
- [ ] `medulla hook install` / `medulla hook uninstall` commands
- [ ] YAML frontmatter format for decisions
- [ ] Grouped task files (active.md, completed.md)
- [ ] Auto-generated README.md index

### Phase 5: Polish & Distribution

- [ ] HTTP transport for web UIs (`medulla serve --http 3000`)
- [ ] OpenAPI documentation for HTTP mode
- [ ] `medulla cache rebuild` command
- [ ] Homebrew formula
- [ ] Cargo install (`cargo install medulla`)
- [ ] Documentation site (medulla.cc)

### Future Enhancements

- [ ] GitHub Issues sync (import/export)
- [ ] Team features (permissions, roles)
- [ ] Encryption at rest
- [ ] Plugin system for custom entity types
- [ ] MCP Prompts (project_summary, decision_review, task_breakdown, etc.)
- [ ] OpenAI embeddings option (via `OPENAI_API_KEY`)

---

## Out of Scope (Initial Release)

The following features are explicitly excluded from the initial release:

- Encryption at rest
- GitHub Issues sync
- Multi-repo federation
- Team permissions/roles
- Plugin system
- MCP Prompts (templates)

---

## Competitive Analysis

| Tool | Medulla Advantage |
|------|-------------------|
| CLAUDE.md | Queryable, structured, dynamic |
| ADRs (Log4brains) | MCP-accessible, linked to tasks |
| GitHub Issues | Git-native, AI-first, no external API |
| Notion/Obsidian | Lives in repo, no separate app |
| MCP Memory Server | Project-scoped, not personal |

---

## References

- [MCP Specification 2025-11-25](https://modelcontextprotocol.io/specification/2025-11-25)
- [Loro CRDT](https://loro.dev/)
- [ADR GitHub](https://adr.github.io/)
- [CLAUDE.md Best Practices](https://www.humanlayer.dev/blog/writing-a-good-claude-md)
- [fastembed](https://github.com/Anush008/fastembed-rs)

---

## Next Steps & Open Questions

Before implementation begins, the following areas need design decisions or further discussion.

### Immediate Next Steps

1. ~~**Loro schema design**: How do entities and relations map to Loro's data structures (Map, List, Tree)? This shapes everything else.~~ ✓ Complete — see [Loro Schema Design](#loro-schema-design) section.
2. ~~**Resolve open design questions**: Git integration, CLI design, MCP implementation, performance limits.~~ ✓ Complete — see TD14-TD23.
3. ~~**Scaffold Rust project**: Set up cargo workspace, dependencies, basic module structure.~~ ✓ Complete — see [docs/plans/2025-01-30-scaffold-rust-project.md](plans/2025-01-30-scaffold-rust-project.md).
4. ~~**Vertical slice**: Implement `init` + `add decision` + `list` end-to-end to prove the CRDT layer works.~~ ✓ Complete — CLI working with Loro CRDT storage.
5. **Complete Phase 1**: Add remaining entity types, relations, update/delete commands, SQLite cache.
6. **Validation**: Test git merge behavior with decisions on different branches.

### Open Design Questions ✓ All Resolved

All design questions have been resolved. See TD14-TD23 for detailed decisions.

#### Data Model Details

- ~~**Entity IDs**: UUIDs (portable, no conflicts) vs sequential numbers (human-readable for decisions)? Hybrid approach?~~ ✓ Hybrid — see [Entity IDs](#entity-ids-hybrid-approach)
- ~~**Relation storage**: Store on source entity, target entity, or separate collection? Affects query patterns.~~ ✓ Separate collection — see [Relation Structure](#relation-structure)
- ~~**Timestamps**: Store as ISO strings or Unix timestamps in Loro?~~ ✓ ISO 8601 strings
- ~~**Content format**: Raw markdown string, or structured (blocks, AST)?~~ ✓ Raw markdown string

#### Git Integration

- ~~**Hook type**: Pre-commit (blocks commit until snapshot generated) vs post-commit (async, never blocks)?~~ ✓ Pre-commit with fast-path — only runs if `loro.db` is staged
- ~~**Hook failures**: What happens if snapshot generation fails? Abort commit? Warn and continue?~~ ✓ Abort with escape hatch — abort by default, respect `--no-verify` for emergencies
- ~~**Dirty detection**: How to know if snapshot is stale and needs regeneration?~~ ✓ Git staging check — use `git diff --cached` to detect if `loro.db` is staged

#### CLI Design

- ~~**Relation input**: How do users specify relations on the command line? `--blocks=<id>`? `--relation="blocks:abc123"`?~~ ✓ Generic relation flag — `--relation="type:target"` for all relation types, supports multiple
- ~~**Content input**: Stdin for body works, but what about interactive editing? Respect `$EDITOR`?~~ ✓ Stdin + $EDITOR — stdin by default, `--edit` flag opens `$EDITOR` with template (like `git commit -e`)
- ~~**ID display**: Show full UUID or truncated? Allow prefix matching like git?~~ ✓ Both — accept sequence number (`3`) or UUID prefix (`a1b2c`), display as `003 (a1b2c3d)`

#### MCP Implementation

- ~~**Tool naming**: `entity.create` (namespaced) vs `medulla_create_entity` (flat)? Check MCP conventions.~~ ✓ Snake case with grouping — `entity_create`, `search_fulltext`, `graph_relations` (MCP namespaces by server)
- ~~**Batch operations**: Transactional (all-or-nothing) or best-effort?~~ ✓ Best-effort with report — execute all operations, return per-operation success/failure
- ~~**Subscription granularity**: Subscribe to all entities, by type, or by ID?~~ ✓ By type — subscribe to `medulla://decisions`, `medulla://tasks`, etc.

#### Performance & Limits

- ~~**Max entity count**: At what scale does Loro performance degrade? Should we warn/limit?~~ ✓ Soft warnings — warn at thresholds (e.g., 1,000 entities) but allow continuation
- ~~**Embedding batch size**: Compute embeddings one-by-one or batch on snapshot?~~ ✓ Immediate — compute embedding on each add/update (~50-200ms acceptable for local use)
- ~~**Cache invalidation**: When does `cache.db` need to be rebuilt?~~ ✓ Incremental with version tracking — store last synced Loro version, diff and reindex only changes

### Decisions to Defer

These can be decided during implementation, not upfront:

- Exact YAML frontmatter fields for each snapshot type
- Specific filter syntax for `medulla list`
- Error message wording
- CLI help text formatting

### Validation Milestones

Before moving to each phase, validate:

1. **After Phase 1**: Can two branches with different decisions merge cleanly via git?
2. **After Phase 2**: Can Claude Desktop query and create decisions via MCP?
3. **After Phase 3**: Does semantic search return relevant results? Are embeddings computed within ~200ms?
4. **After Phase 4**: Is the snapshot readable on GitHub without any tooling?
5. **After Phase 5**: Can users install via Homebrew and `cargo install`?
