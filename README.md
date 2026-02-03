# Medulla

[![License: Apache 2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![Crates.io](https://img.shields.io/crates/v/medulla)](https://crates.io/crates/medulla)
[![NPM](https://img.shields.io/npm/v/medulla-cc)](https://www.npmjs.com/package/medulla-cc)
[![Rust](https://img.shields.io/badge/rust-1.75+-orange.svg)](https://www.rust-lang.org)
[![MCP](https://img.shields.io/badge/MCP-2025--11--25-purple)](https://modelcontextprotocol.io/)
[![Powered by Loro](https://img.shields.io/badge/Powered%20by-Loro-green)](https://loro.dev/)
[![CLI Tool](https://img.shields.io/badge/CLI-Tool-black?logo=terminal)]()

> 🚧 **Beta**: Medulla is actively being developed. We'd love for you to try it out and [report any issues](https://github.com/skeletor-js/medulla/issues) you find!

A **free, open-source**, git-native knowledge engine for software projects.

No subscriptions. No cloud dependencies. Your data stays in your repo.

## What is Medulla?

Medulla is a **project-scoped context engine** that lives in your git repository. It gives AI tools (Claude Code, Cursor, Copilot, etc.) structured access to project knowledge—decisions, tasks, notes, prompts—via the Model Context Protocol (MCP).

Unlike static files like `CLAUDE.md` or `.cursorrules`, Medulla provides:

- **Queryable data**: "What did we decide about authentication?"
- **Full-text & semantic search**: Find related decisions instantly
- **Dynamic updates**: Context evolves as the project evolves
- **Structured types**: Decisions, tasks, prompts with schemas
- **Conflict-free sync**: CRDT-based, merges cleanly across branches
- **Human-readable snapshots**: Auto-generated markdown for GitHub browsing

## Who is this for?

Medulla is designed for teams who:

- **Use AI assistants heavily** (Claude Code, Cursor, Copilot) and feel the pain of context loss between sessions
- **Already maintain documentation** but want it to be queryable and AI-accessible
- **Work across multiple branches** and need decisions that merge cleanly
- **Are tired of subscription fees** for AI context/memory tools and want a free, self-hosted alternative

**This might not be for you if:**

- You're happy with GitHub Issues + markdown files and don't struggle with AI context management
- Your team doesn't already write any project documentation
- You rarely use AI coding assistants

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

## FAQ

**Q: How is this better than just writing ADRs in markdown?**

ADRs are great for humans but terrible for AI assistants—they can't query them, search semantically, or understand relationships. Medulla captures the same information in a structured format that both humans and AI can use.

**Q: Isn't this just another thing to maintain?**

Initially, yes. But once set up, the pre-commit hook auto-generates human-readable snapshots. The real question is: do you value AI-accessible project knowledge enough to invest 30 seconds per decision?

**Q: Why not use a paid AI context/memory service?**

Most AI context tools charge monthly subscriptions and store your data in their cloud. Medulla is free, open-source, and lives entirely in your git repo. Your data stays yours—forever accessible, even if the company behind it disappears.

**Q: What if MCP doesn't become the standard?**

Medulla's CLI and markdown snapshots work independently of MCP. If the protocol landscape changes, your data remains accessible via git and the command line.

## Why Medulla?

We built this because we were frustrated. Here's where it helps:

### The Problem

**Problem 1: Context loss between AI sessions**

AI assistants forget what you discussed yesterday. You end up re-explaining the same architecture decisions over and over.

**Problem 2: Finding old decisions is painful**

Without Medulla:

```bash
# Searching project context the hard way
git log --all --oneline --grep="authentication"
grep -r "auth" docs/adr/
# Hope you remember which PR discussed JWT vs sessions
```

With Medulla:

```bash
# Queryable knowledge at your fingertips
medulla search "why JWT over sessions"
medulla search --semantic "authentication decisions"
medulla get 1  # Shows full context, rationale, and consequences
```

**Problem 3: Merge conflicts in documentation**

Editing ADRs on multiple branches causes conflicts. CRDTs solve this automatically—your decisions merge cleanly regardless of when or where they were edited.

**When NOT to use Medulla:**

- If you rarely use AI coding assistants
- If your team doesn't already write documentation
- If GitHub Issues covers all your needs perfectly

We think it's worth trying if AI context loss frustrates you regularly.

### Comparison

| Solution | Queryable? | Semantic Search? | MCP? | Merge-safe? | Cost |
|----------|------------|------------------|------|-------------|------|
| CLAUDE.md | As raw text | No | No | No | Free |
| ADRs (Log4brains) | No | No | No | No | Free |
| GitHub Issues | Via API | No | Via MCP | N/A | Free* |
| Notion/Obsidian | Limited | No | No | No | Subscription |
| **Medulla** | **Yes** | **Yes** | **Native** | **Yes** | **Free** |

*GitHub Issues is free for public repos; paid features for private repos at scale

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
