# voice

> terse. technical. lowercase. no hype.

medulla speaks like a senior engineer who writes excellent documentation—clear, helpful, occasionally dry-humored, never condescending.

---

## tone characteristics

| dimension | position |
|-----------|----------|
| formal ↔ casual | slightly casual—professional but not stiff |
| serious ↔ playful | mostly serious, occasionally dry |
| respectful ↔ irreverent | respectful—no snark |
| enthusiastic ↔ matter-of-fact | matter-of-fact—confidence without hype |

---

## voice examples

| context | don't | do |
|---------|-------|-----|
| tagline | "Your Project's Brain, Accessible to Any AI!" | "your project's brain, accessible to any AI" |
| error | "Oops! We couldn't find that entity." | "entity not found: a1b2c3d" |
| success | "Great job! Decision created successfully! 🎉" | "✓ decision created: 003 (a1b2c3d)" |
| feature | "Revolutionary AI-Powered Context Engine" | "queryable project memory via MCP" |
| empty state | "Nothing here yet! Why not add something?" | "no decisions found. create one with `medulla add decision`" |
| loading | "Hang tight, we're working on it..." | "indexing 47 entities..." |
| completion | "All done! You're all set!" | "done. 47 entities indexed in 1.2s" |

---

## writing guidelines

### lead with what, then why

```
✓ medulla uses Loro CRDT for storage because it enables 
  conflict-free merging across git branches.

✗ we chose an amazing technology called CRDT which stands 
  for Conflict-free Replicated Data Type because we wanted 
  to make sure your data would never have merge conflicts!
```

### be specific

```
✓ full-text search via SQLite FTS5, semantic search via local embeddings

✗ fast, powerful search capabilities
```

### acknowledge tradeoffs

```
✓ this adds ~50-200ms per entity update, which is acceptable for local-first usage

✗ blazing fast performance!
```

### respect expertise

don't over-explain git, MCP, or CRDT to the target audience. assume they know what these are.

### show, don't tell

prefer code examples and concrete output over abstract descriptions.

```
✓ $ medulla search "database"
  › 001 Use Postgres for data storage [accepted]

✗ medulla's powerful search functionality allows you to 
  find any entity in your knowledge base quickly and easily.
```

---

## capitalization rules

### sentence case everywhere

all headlines, titles, and labels use sentence case:

```
✓ the problem
✗ The Problem

✓ how it works
✗ How It Works

✓ getting started with medulla
✗ Getting Started With Medulla
```

### "medulla" is always lowercase

even at the start of sentences:

```
✓ medulla stores project decisions in a CRDT.
✗ Medulla stores project decisions in a CRDT.
```

### technical terms follow their conventions

```
✓ CRDT, MCP, SQLite, Git, JSON, API
✗ Crdt, Mcp, Sqlite, git, Json, Api
```

---

## punctuation

- use serial (Oxford) commas
- no exclamation points
- no emoji in product UI or documentation
- inline code uses backticks: `medulla add decision`
- em dashes with spaces: "medulla stores decisions — not just text"

---

## CLI output patterns

### success

```
✓ decision created: 003 (a1b2c3d)
```

### error

```
✗ error: missing required argument <title>

  usage: medulla add decision <title> [options]

  run 'medulla add decision --help' for details
```

### warning

```
⚠ warning: 1,247 entities indexed. performance may degrade above 1,000.
```

### progress

```
  indexing entities...
  [████████████████████░░░░░░░░░░░░] 342/512
```

### completion

```
  done. 512 entities indexed in 2.3s
```

### empty state

```
  no tasks found. create one with 'medulla add task "Your task"'
```

### help

```
medulla v0.1.0
your project's brain, accessible to any AI

usage: medulla <command> [options]

commands:
  init          initialize medulla in current directory
  add           create a new entity
  list          list entities with optional filters
  get           retrieve a specific entity by id
  search        full-text and semantic search
  serve         start MCP server

options:
  -h, --help    show this help message
  -v, --version show version
  --json        output as JSON

run 'medulla <command> --help' for command-specific help
```

---

## messaging by context

### pain point messaging

| pain | message |
|------|---------|
| context loss | "stop re-explaining your project to AI assistants. medulla remembers." |
| merge conflicts | "your project memory should merge as cleanly as your code." |
| external dependencies | "no accounts. no API keys. your project knowledge lives in your repo." |
| static files | "`CLAUDE.md` is a start. medulla is the evolution." |

### feature descriptions

| feature | description |
|---------|-------------|
| CRDT storage | "conflict-free data that merges cleanly across git branches" |
| MCP interface | "structured access for Claude, Cursor, Copilot, and any MCP client" |
| local embeddings | "semantic search without API keys or external services" |
| markdown snapshot | "human-readable output that's browsable on GitHub" |

---

## don'ts

- no exclamation points
- no emoji
- no "we're excited to announce"
- no "revolutionary" or "game-changing"
- no title case headlines
- no capitalizing "medulla"
- no apologizing for limitations
- no filler words ("just", "simply", "easily")
- no passive voice when active is clearer
- no marketing speak in technical contexts
