# Medulla

A git-native, AI-accessible knowledge engine for software projects.

## What is Medulla?

Medulla is a **project-scoped context engine** that lives in your git repository. It gives AI tools (Claude Code, Cursor, Copilot, etc.) structured access to project knowledge—decisions, tasks, notes, prompts—via the Model Context Protocol (MCP).

Unlike static files like `CLAUDE.md` or `.cursorrules`, Medulla provides:

- **Queryable data**: "What did we decide about authentication?"
- **Dynamic updates**: Context evolves as the project evolves
- **Structured types**: Decisions, tasks, prompts with schemas
- **Conflict-free sync**: CRDT-based, merges cleanly across branches

## Current Status

**Phase 1 (Vertical Slice)**: Core engine complete. CLI working with Loro CRDT storage.

```text
medulla init                    ✅
medulla add decision "..."      ✅
medulla list                    ✅
medulla get <id>                ✅
```

See the full [Product Requirements Document](docs/prd.md) for detailed roadmap.

## Installation

### From Source (Rust)

```bash
git clone https://github.com/jordanstella/medulla.git
cd medulla
cargo build --release
# Binary at ./target/release/medulla (~3.1MB)
```

### Coming Soon

- Homebrew: `brew install medulla`
- Cargo: `cargo install medulla`

## Quick Start

```bash
# Initialize Medulla in your project
medulla init

# Add a decision
medulla add decision "Use PostgreSQL for primary database" \
  --status accepted \
  --tag database

# List all decisions
medulla list

# Get a specific decision (by sequence number or UUID prefix)
medulla get 1
medulla get a1b2c3
```

## How It Works

Medulla stores project knowledge in a **Loro CRDT** (conflict-free replicated data type) that merges cleanly across git branches. It exposes this data via **MCP** for AI tools and auto-generates a **human-readable markdown snapshot** for GitHub browsing.

```text
.medulla/
  loro.db              # CRDT source of truth (binary, git-tracked)
  schema.json          # Type definitions
  config.json          # Project configuration
  snapshot/            # Auto-generated markdown (Phase 4)
```

### Data Model

| Type | Purpose |
|------|---------|
| **decision** | Architectural decisions (ADRs) |
| **task** | Work items |
| **note** | Freeform project notes |
| **prompt** | AI prompt templates |
| **component** | System components |
| **link** | External resources |

## Why Medulla?

### The Problem

AI assistants forget project decisions between sessions. Static markdown files can't be queried or filtered. Traditional markdown files conflict when edited on multiple branches.

### The Solution

Medulla captures decisions as queryable data:

```bash
# Capture a decision
medulla add decision "Use Rust over TypeScript" \
  --status accepted \
  --tag architecture \
  --context "Single binary distribution, first-class Loro support"

# Future sessions can query it
medulla search "why Rust"
```

### Comparison

| Solution | Queryable? | Dynamic? | Merge-safe? |
|----------|------------|----------|-------------|
| CLAUDE.md | As raw text | No | No |
| ADRs (Log4brains) | No MCP | No | No |
| GitHub Issues | Via MCP | Yes | N/A |
| **Medulla** | **Yes** | **Yes** | **Yes** |

## Tech Stack

| Component | Technology |
|-----------|------------|
| Language | Rust |
| CRDT | [Loro](https://loro.dev/) |
| CLI | clap |
| Serialization | serde |

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

- **Phase 1** ✅ Core Engine (Loro storage, CLI basics)
- **Phase 2** MCP Server (stdio transport, entity tools)
- **Phase 3** Search & Graph (FTS5, semantic search via fastembed)
- **Phase 4** Snapshot & Git Integration (markdown generation, hooks)
- **Phase 5** Polish & Distribution (Homebrew, cargo install)

## License

MIT

## Links

- [Product Requirements Document](docs/prd.md)
- [Website](https://medulla.cc) (coming soon)
- [Loro CRDT](https://loro.dev/)
- [Model Context Protocol](https://modelcontextprotocol.io/)
