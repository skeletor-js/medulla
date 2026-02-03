# Medulla

[![Crates.io](https://img.shields.io/crates/v/medulla)](https://crates.io/crates/medulla)
[![NPM](https://img.shields.io/npm/v/medulla-cc)](https://www.npmjs.com/package/medulla-cc)
[![License: Apache 2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![Rust](https://img.shields.io/badge/rust-1.75+-orange.svg)](https://www.rust-lang.org)
[![MCP](https://img.shields.io/badge/MCP-2025--11--25-purple)](https://modelcontextprotocol.io/)
[![Powered by Loro](https://img.shields.io/badge/Powered%20by-Loro-green)](https://loro.dev/)
[![CLI Tool](https://img.shields.io/badge/CLI-Tool-black?logo=terminal)]()

> 🚧 **Beta**: Medulla is actively being developed. We'd love for you to try it out and [report any issues](https://github.com/skeletor-js/medulla/issues) you find!

A git-native, AI-accessible knowledge engine for software projects.

## What is Medulla?

Medulla is a **project-scoped context engine** that lives in your git repository. It gives AI tools (Claude Code, Cursor, Copilot, etc.) structured access to project knowledge—decisions, tasks, notes, prompts—via the Model Context Protocol (MCP).

Unlike static files like `CLAUDE.md` or `.cursorrules`, Medulla provides:

- **Queryable data**: "What did we decide about authentication?"
- **Full-text & semantic search**: Find related decisions instantly
- **Dynamic updates**: Context evolves as the project evolves
- **Structured types**: Decisions, tasks, prompts with schemas
- **Conflict-free sync**: CRDT-based, merges cleanly across branches
- **Human-readable snapshots**: Auto-generated markdown for GitHub browsing

## Installation

### Cargo (Rust)

```bash
cargo install medulla
```

### NPM (Node.js)

```bash
npm install -g medulla-cc
```

The NPM package automatically downloads the appropriate binary for your platform (macOS, Linux, Windows on x64 and ARM64).

### Homebrew (Coming Soon)

```bash
brew install medulla
```

### From Source

```bash
git clone https://github.com/skeletor-js/medulla.git
cd medulla
cargo build --release
# Binary at ./target/release/medulla (~3.1MB)
```

## Quick Start

```bash
# Initialize Medulla in your project
medulla init

# Add a decision
medulla add decision "Use PostgreSQL for primary database" \
  --status accepted \
  --tag database

# Search your knowledge base
medulla search "database"
medulla search --semantic "authentication strategy"

# List all decisions
medulla list

# Get a specific decision (by sequence number or UUID prefix)
medulla get 1
medulla get a1b2c3
```

## How It Works

Medulla stores project knowledge in a **Loro CRDT** (conflict-free replicated data type) that merges cleanly across git branches. It exposes this data via **MCP** for AI tools and auto-generates **human-readable markdown snapshots** for GitHub browsing.

```text
.medulla/
  loro.db              # CRDT source of truth (binary, git-tracked)
  schema.json          # Type definitions
  config.json          # Project configuration
  cache.db             # SQLite for search & embeddings (gitignored)
  snapshot/            # Auto-generated markdown
    README.md          # Index of all entities
    decisions/
    tasks/
    notes/
    prompts/
```

### Data Model

| Type | Purpose | Key Properties |
|------|---------|----------------|
| **decision** | Architectural decisions (ADRs) | `status`, `context`, `consequences` |
| **task** | Work items | `status`, `priority`, `due_date`, `assignee` |
| **note** | Freeform project notes | `note_type` |
| **prompt** | AI prompt templates | `template`, `variables` |
| **component** | System components | `component_type`, `status` |
| **link** | External resources | `url`, `link_type` |

### Built-in Relations

Link entities together to build a knowledge graph:

- `implements` — Task implements a decision
- `blocks` — Blocking dependency between tasks
- `supersedes` — New decision replaces old
- `references` — General reference between any entities
- `belongs_to` — Task belongs to a component
- `documents` — Note documents a component

## MCP Integration

Medulla exposes your project knowledge via the [Model Context Protocol](https://modelcontextprotocol.io/), making it accessible to AI assistants.

### Transport Modes

| Mode | Command | Use Case |
|------|---------|----------|
| **stdio** (default) | `medulla serve` | Claude Desktop, Cursor, local AI tools |
| **HTTP** | `medulla serve --http 3000` | Web UIs, remote clients, custom integrations |

### MCP Tools

- `entity_create`, `entity_update`, `entity_delete`, `entity_get`, `entity_list`
- `search_fulltext`, `search_semantic`, `search_query`
- `graph_relations`, `graph_path`, `graph_orphans`
- `task_complete`, `task_reschedule`, `decision_supersede`
- `sync_snapshot` — Generate markdown snapshot

### MCP Resources

Access your data via URI templates:

- `medulla://decisions` — All decisions
- `medulla://tasks/active` — Incomplete tasks
- `medulla://entity/{id}` — Single entity
- `medulla://context/{topic}` — Semantic search results

## Why Medulla?

### The Problem

AI assistants forget project decisions between sessions. Static markdown files can't be queried or filtered. Traditional markdown files conflict when edited on multiple branches.

### The Solution

Medulla captures decisions as queryable, searchable data:

```bash
# Capture a decision with context
medulla add decision "Use Rust over TypeScript" \
  --status accepted \
  --tag architecture \
  --context "Single binary distribution, first-class Loro support"

# Future sessions can find it
medulla search "why Rust"
medulla search --semantic "technology choice"
```

### Comparison

| Solution | Queryable? | Semantic Search? | MCP? | Merge-safe? |
|----------|------------|------------------|------|-------------|
| CLAUDE.md | As raw text | No | No | No |
| ADRs (Log4brains) | No | No | No | No |
| GitHub Issues | Via API | No | Via MCP | N/A |
| **Medulla** | **Yes** | **Yes** | **Native** | **Yes** |

## Git Integration

Medulla includes a pre-commit hook that automatically generates markdown snapshots when you commit changes:

```bash
# Install the git hook
medulla hook install

# Generate snapshot manually
medulla snapshot
```

The hook has a fast-path: it only runs if `.medulla/loro.db` is staged, so regular commits aren't slowed down.

## Development

```bash
# Run tests
cargo test

# Build release
cargo build --release

# Run directly
cargo run -- init
cargo run -- add decision "Test decision"
cargo run -- list
```

## Roadmap

> We're building this in the open! Many features are still being implemented and we welcome feedback and contributions.

- ✅ **Phase 1** — Core Engine (Loro storage, CLI basics, all entity types)
- ✅ **Phase 2** — MCP Server (stdio transport, entity tools, graph tools)
- ✅ **Phase 3** — Search & Graph (FTS5 full-text, fastembed semantic search)
- ✅ **Phase 4** — Snapshot & Git Integration (markdown generation, pre-commit hook)
- ✅ **Phase 5** — HTTP Transport & Distribution (HTTP mode, cargo/npm publish)
- 🔮 **Future** — Homebrew formula, GitHub Issues sync, MCP Prompts, encryption

## License

Apache 2.0

## Links

- [Crates.io](https://crates.io/crates/medulla)
- [NPM](https://www.npmjs.com/package/medulla-cc)
- [Product Requirements Document](docs/prd.md)
- [Loro CRDT](https://loro.dev/)
- [Model Context Protocol](https://modelcontextprotocol.io/)
