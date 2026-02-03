# Phase 4: Snapshot & Git Integration Implementation Plan

**Status**: Complete
**Date**: 2026-02-03
**Completed**: 2026-02-03
**Dependencies**: Phase 3 Complete (Semantic Search & Filters)

---

## Overview

Phase 4 implements markdown snapshot generation and git hook integration, making Medulla data browsable on GitHub without any tooling.

### Goals

1. **Markdown snapshots** - Human-readable derived view of all entities
2. **Git hooks** - Automatic snapshot regeneration on commit
3. **GitHub browsable** - Snapshot files render nicely in GitHub UI

### Key Technical Decisions (from PRD)

| Decision | Reference |
|----------|-----------|
| Snapshot is read-only (manual edits overwritten) | TD3 |
| Flat by type with YAML frontmatter + markdown body | TD6 |
| Pre-commit hook with fast-path (only if `loro.db` staged) | TD14 |
| Abort on failure, `--no-verify` escape hatch | TD14 |

---

## Target Directory Structure

```
.medulla/
├── loro.db              # CRDT source of truth (git-tracked)
├── cache.db             # SQLite FTS + embeddings (gitignored)
└── snapshot/            # Auto-generated (git-tracked, read-only)
    ├── README.md        # Auto-generated index with links
    ├── decisions/
    │   ├── 001-use-postgres.md
    │   └── 002-auth-with-jwt.md
    ├── tasks/
    │   ├── active.md    # Non-completed tasks grouped by priority
    │   └── completed.md # Done tasks (archived)
    ├── notes/
    │   └── <slug>.md    # Individual note files
    ├── prompts/
    │   └── <slug>.md    # Individual prompt files
    ├── components/
    │   └── <slug>.md    # Individual component files
    └── links/
        └── <slug>.md    # Individual link files
```

---

## Implementation Batches

### Batch 1: Snapshot Module Foundation

**Goal**: Create the snapshot generation infrastructure and utilities.

**Files to Create**:
- `src/snapshot/mod.rs` - Module exports and main `generate_snapshot()` function
- `src/snapshot/frontmatter.rs` - YAML frontmatter serialization
- `src/snapshot/utils.rs` - Slug generation, file writing utilities

**Tasks**:

1. **Create `src/snapshot/mod.rs`**
   ```rust
   mod frontmatter;
   mod utils;
   mod decision;
   mod task;
   mod note;
   mod prompt;
   mod component;
   mod link;
   mod readme;

   pub use self::frontmatter::*;
   pub use self::utils::*;

   pub fn generate_snapshot(store: &LoroStore, root: &Path) -> Result<SnapshotStats>;
   ```

2. **Create `src/snapshot/frontmatter.rs`**
   - `to_yaml_frontmatter<T: Serialize>(data: &T) -> String`
   - Handle optional fields gracefully (skip nulls)
   - Format dates as YYYY-MM-DD
   - Format arrays as YAML lists

3. **Create `src/snapshot/utils.rs`**
   - `slugify(title: &str) -> String` - Convert title to file-safe slug
   - `write_snapshot_file(path: &Path, content: &str) -> Result<()>`
   - `ensure_snapshot_dir(root: &Path) -> Result<PathBuf>`
   - `clear_snapshot_dir(snapshot_dir: &Path) -> Result<()>` - Remove old files before regenerating

4. **Update `src/lib.rs`** - Export snapshot module

**Tests**:
- Unit tests for `slugify()` (handles special chars, unicode, etc.)
- Unit tests for YAML frontmatter generation
- Unit test for file writing utilities

---

### Batch 2: Decision Snapshots

**Goal**: Generate numbered decision files with full YAML frontmatter.

**Files to Create/Modify**:
- `src/snapshot/decision.rs` - Decision snapshot generation

**Target Format** (`001-use-postgres.md`):
```markdown
---
id: abc123de-f456-7890-abcd-ef1234567890
sequence: 1
title: Use PostgreSQL for primary database
status: accepted
created: 2025-01-30
updated: 2025-01-30
created_by: jordan
tags:
  - database
  - infrastructure
superseded_by: null
---

## Context

We need a relational database for user data...

## Decision

We will use PostgreSQL 16...

## Consequences

- Team needs PostgreSQL expertise
- Enables advanced querying features
```

**Tasks**:

1. **Create `src/snapshot/decision.rs`**
   ```rust
   pub fn generate_decision_snapshots(
       store: &LoroStore,
       snapshot_dir: &Path,
   ) -> Result<Vec<GeneratedFile>>;
   ```

2. **Implement decision frontmatter struct**
   ```rust
   #[derive(Serialize)]
   struct DecisionFrontmatter {
       id: String,
       sequence: u32,
       title: String,
       status: String,
       created: String,      // YYYY-MM-DD
       updated: String,      // YYYY-MM-DD
       created_by: Option<String>,
       tags: Vec<String>,
       superseded_by: Option<String>,
   }
   ```

3. **Generate markdown body**
   - `## Context` section from `decision.context`
   - Main content from `decision.content`
   - `## Consequences` section from `decision.consequences` (bulleted list)

4. **File naming**: `{sequence:03}-{slug}.md` (e.g., `001-use-postgres.md`)

5. **Sort decisions by sequence number** for consistent ordering

**Tests**:
- Unit test: Decision with all fields populated
- Unit test: Decision with minimal fields (no context, no consequences)
- Unit test: Superseded decision includes `superseded_by` reference
- Unit test: File naming with special characters in title

---

### Batch 3: Task Snapshots

**Goal**: Generate grouped task files (active.md, completed.md).

**Files to Create/Modify**:
- `src/snapshot/task.rs` - Task snapshot generation

**Target Format** (`active.md`):
```markdown
# Active Tasks

> Generated from Medulla. Do not edit directly.

## Urgent

- [ ] **Fix authentication bug** `#5` `(abc123d)`
  Due: 2025-02-01 | Assignee: jordan | Tags: auth, security

## High Priority

- [ ] **Implement OAuth flow** `#8` `(def456e)`
  Tags: auth

## Normal Priority

- [ ] **Write API docs** `#3` `(789abcd)`
  Tags: documentation

## Low Priority

- [ ] **Refactor utils module** `#12` `(fedcba9)`

---

*Last updated: 2025-02-03 12:34:56 UTC*
```

**Target Format** (`completed.md`):
```markdown
# Completed Tasks

> Generated from Medulla. Do not edit directly.

- [x] **Set up CI pipeline** `#1` `(111222a)` - Completed 2025-01-28
  Tags: infra, ci

- [x] **Add user model** `#2` `(333444b)` - Completed 2025-01-29
  Tags: database

---

*Last updated: 2025-02-03 12:34:56 UTC*
```

**Tasks**:

1. **Create `src/snapshot/task.rs`**
   ```rust
   pub fn generate_task_snapshots(
       store: &LoroStore,
       snapshot_dir: &Path,
   ) -> Result<Vec<GeneratedFile>>;
   ```

2. **Group active tasks by priority** (urgent, high, normal, low)

3. **Format task line**:
   - Checkbox: `- [ ]` for active, `- [x]` for completed
   - Bold title
   - Sequence number and short UUID
   - Optional: due date, assignee, tags on second line

4. **Handle blocked tasks** - Show in active.md with `[blocked]` indicator

5. **Sort within priority groups** by due date (soonest first), then by sequence number

**Tests**:
- Unit test: Tasks grouped correctly by priority
- Unit test: Blocked tasks marked appropriately
- Unit test: Completed tasks in separate file
- Unit test: Due dates formatted correctly

---

### Batch 4: Note & Prompt Snapshots

**Goal**: Generate individual files for notes and prompts.

**Files to Create/Modify**:
- `src/snapshot/note.rs` - Note snapshot generation
- `src/snapshot/prompt.rs` - Prompt snapshot generation

**Note Format** (`meeting-notes-2025-01-30.md`):
```markdown
---
id: abc123de-f456-7890-abcd-ef1234567890
sequence: 7
title: Meeting notes 2025-01-30
type: meeting
created: 2025-01-30
updated: 2025-01-30
created_by: jordan
tags:
  - planning
  - q1
---

Discussion about Q1 priorities...
```

**Prompt Format** (`code-review.md`):
```markdown
---
id: abc123de-f456-7890-abcd-ef1234567890
sequence: 2
title: Code Review
created: 2025-01-25
updated: 2025-01-25
created_by: jordan
tags:
  - review
variables:
  - file_path
  - focus_areas
---

## Template

Review the code in {{file_path}} with focus on:
{{focus_areas}}

Provide feedback on:
1. Code quality
2. Performance
3. Security

## Output Schema

```json
{
  "type": "object",
  "properties": {
    "issues": { "type": "array" },
    "suggestions": { "type": "array" }
  }
}
```
```

**Tasks**:

1. **Create `src/snapshot/note.rs`**
   ```rust
   pub fn generate_note_snapshots(
       store: &LoroStore,
       snapshot_dir: &Path,
   ) -> Result<Vec<GeneratedFile>>;
   ```

2. **Create `src/snapshot/prompt.rs`**
   ```rust
   pub fn generate_prompt_snapshots(
       store: &LoroStore,
       snapshot_dir: &Path,
   ) -> Result<Vec<GeneratedFile>>;
   ```

3. **File naming**: `{slug}.md` based on title

4. **Handle duplicate slugs**: Append sequence number if collision (e.g., `meeting-notes-7.md`)

5. **Prompt-specific sections**:
   - `## Template` with the template content
   - `## Output Schema` (if defined) with JSON code block

**Tests**:
- Unit test: Note with type field
- Unit test: Note without type field
- Unit test: Prompt with variables and output schema
- Unit test: Slug collision handling

---

### Batch 5: Component & Link Snapshots + README Index

**Goal**: Complete entity snapshots and generate the index README.

**Files to Create/Modify**:
- `src/snapshot/component.rs` - Component snapshot generation
- `src/snapshot/link.rs` - Link snapshot generation
- `src/snapshot/readme.rs` - README index generation

**Component Format** (`auth-service.md`):
```markdown
---
id: abc123de-f456-7890-abcd-ef1234567890
sequence: 3
title: Auth Service
type: service
status: active
owner: jordan
created: 2025-01-20
updated: 2025-01-25
tags:
  - backend
  - security
---

Handles user authentication and authorization...

## Related

- Implements: [Use JWT for authentication](../decisions/002-auth-with-jwt.md)
- Tasks: [Implement OAuth flow](../tasks/active.md#8)
```

**Link Format** (`rust-book.md`):
```markdown
---
id: abc123de-f456-7890-abcd-ef1234567890
sequence: 1
title: The Rust Book
url: https://doc.rust-lang.org/book/
type: documentation
created: 2025-01-15
tags:
  - rust
  - learning
---

Official Rust programming language book.
```

**README.md Format**:
```markdown
# Project Knowledge Base

> Auto-generated by [Medulla](https://medulla.cc). Do not edit directly.

## Summary

| Type | Count |
|------|-------|
| Decisions | 5 |
| Tasks | 12 (8 active) |
| Notes | 3 |
| Prompts | 2 |
| Components | 4 |
| Links | 6 |

## Recent Activity

- **Decision**: [Use PostgreSQL](decisions/001-use-postgres.md) - accepted
- **Task**: [Implement OAuth flow](tasks/active.md#8) - in progress
- **Note**: [Meeting notes 2025-01-30](notes/meeting-notes-2025-01-30.md)

## Quick Links

### Decisions

- [001 - Use PostgreSQL](decisions/001-use-postgres.md) `accepted`
- [002 - Use JWT](decisions/002-auth-with-jwt.md) `accepted`

### Active Tasks

See [tasks/active.md](tasks/active.md)

### Components

- [Auth Service](components/auth-service.md) `active`
- [API Gateway](components/api-gateway.md) `active`

---

*Generated: 2025-02-03 12:34:56 UTC*
```

**Tasks**:

1. **Create `src/snapshot/component.rs`**
2. **Create `src/snapshot/link.rs`**
3. **Create `src/snapshot/readme.rs`**
   ```rust
   pub fn generate_readme(
       store: &LoroStore,
       snapshot_dir: &Path,
       stats: &SnapshotStats,
   ) -> Result<()>;
   ```

4. **SnapshotStats struct**:
   ```rust
   pub struct SnapshotStats {
       pub decisions: usize,
       pub tasks_total: usize,
       pub tasks_active: usize,
       pub notes: usize,
       pub prompts: usize,
       pub components: usize,
       pub links: usize,
       pub generated_files: Vec<String>,
   }
   ```

5. **Recent activity**: Show 5 most recently updated entities across all types

6. **Cross-references in components**: Include `## Related` section with links to related decisions/tasks via relations

**Tests**:
- Unit test: Component with relations
- Unit test: Link snapshot format
- Unit test: README summary table
- Unit test: Recent activity ordering

---

### Batch 6: CLI Command & Integration

**Goal**: Implement `medulla snapshot` CLI command and integrate with main generation.

**Files to Modify**:
- `src/cli/commands.rs` - Add snapshot command
- `src/cli/handlers.rs` - Add snapshot handler
- `src/main.rs` - Wire up command

**Tasks**:

1. **Add CLI command**:
   ```rust
   #[derive(Subcommand)]
   pub enum Commands {
       // ... existing commands

       /// Generate markdown snapshot
       Snapshot {
           /// Output directory (default: .medulla/snapshot)
           #[arg(long)]
           output: Option<PathBuf>,

           /// Show verbose output
           #[arg(short, long)]
           verbose: bool,
       },
   }
   ```

2. **Implement handler**:
   ```rust
   pub fn handle_snapshot(output: Option<PathBuf>, verbose: bool) -> Result<()> {
       let root = find_project_root()?;
       let store = LoroStore::open(&root)?;

       let snapshot_dir = output.unwrap_or_else(|| root.join(".medulla/snapshot"));

       // Clear existing snapshot
       clear_snapshot_dir(&snapshot_dir)?;

       // Generate all snapshots
       let stats = generate_snapshot(&store, &snapshot_dir)?;

       if verbose {
           println!("Generated {} files:", stats.generated_files.len());
           for file in &stats.generated_files {
               println!("  {}", file);
           }
       }

       println!("Snapshot generated: {} decisions, {} tasks, {} notes, {} prompts, {} components, {} links",
           stats.decisions, stats.tasks_total, stats.notes,
           stats.prompts, stats.components, stats.links);

       Ok(())
   }
   ```

3. **Main `generate_snapshot()` function** in `src/snapshot/mod.rs`:
   - Create snapshot directory structure
   - Call each entity-type generator
   - Generate README last (needs stats from others)
   - Return SnapshotStats

4. **Handle empty project** - Generate README with "No entities yet" message

**Tests**:
- Integration test: Full snapshot generation with sample data
- Integration test: Empty project snapshot
- Integration test: Snapshot with `--output` flag

---

### Batch 7: Git Hook Implementation

**Goal**: Implement pre-commit hook installation and management.

**Files to Create/Modify**:
- `src/cli/commands.rs` - Add hook subcommands
- `src/cli/handlers.rs` - Add hook handlers
- `src/hook.rs` - Hook script generation and installation

**Hook Script Template**:
```bash
#!/bin/sh
# Medulla pre-commit hook
# Auto-generated - do not edit

# Fast-path: skip if loro.db not staged
if ! git diff --cached --name-only | grep -q '\.medulla/loro.db'; then
    exit 0
fi

# Generate snapshot
if ! medulla snapshot; then
    echo ""
    echo "Error: Medulla snapshot generation failed."
    echo "Fix the issue or use 'git commit --no-verify' to skip."
    exit 1
fi

# Stage generated snapshot files
git add .medulla/snapshot/

exit 0
```

**Tasks**:

1. **Add CLI commands**:
   ```rust
   #[derive(Subcommand)]
   pub enum HookCommand {
       /// Install git pre-commit hook
       Install {
           /// Force overwrite existing hook
           #[arg(long)]
           force: bool,
       },
       /// Uninstall git pre-commit hook
       Uninstall,
       /// Check if hook is installed
       Status,
   }
   ```

2. **Create `src/hook.rs`**:
   ```rust
   pub fn install_hook(git_dir: &Path, force: bool) -> Result<()>;
   pub fn uninstall_hook(git_dir: &Path) -> Result<()>;
   pub fn hook_status(git_dir: &Path) -> HookStatus;

   pub enum HookStatus {
       Installed,
       NotInstalled,
       CustomHookExists,  // Non-medulla hook exists
   }
   ```

3. **Hook installation logic**:
   - Find `.git/hooks/` directory
   - Check for existing `pre-commit` hook
   - If exists and not ours: require `--force` or abort
   - Write hook script with executable permissions
   - Add marker comment to identify as Medulla hook

4. **Hook uninstallation logic**:
   - Check if hook is Medulla's (by marker comment)
   - If yes, remove it
   - If not ours, warn and abort

5. **Handle hook chaining** (stretch goal):
   - If existing hook found, offer to chain (call original after ours)
   - Store original hook as `pre-commit.backup`

6. **Implement handlers**:
   ```rust
   pub fn handle_hook_install(force: bool) -> Result<()>;
   pub fn handle_hook_uninstall() -> Result<()>;
   pub fn handle_hook_status() -> Result<()>;
   ```

**Tests**:
- Integration test: Install hook in fresh repo
- Integration test: Install hook with existing non-Medulla hook (should fail without --force)
- Integration test: Uninstall hook
- Integration test: Hook status detection
- Integration test: Hook execution (verify snapshot generated on commit)

---

### Batch 8: Testing & Documentation

**Goal**: Comprehensive testing and update documentation.

**Tasks**:

1. **Unit Tests** (in respective modules):
   - Frontmatter serialization edge cases
   - Slug generation edge cases (unicode, very long titles, etc.)
   - Each entity type snapshot format

2. **Integration Tests** (`tests/snapshot_test.rs`):
   - Full snapshot generation with all entity types
   - Snapshot with relations (cross-references)
   - Incremental updates (verify old files removed)
   - Performance with 100+ entities

3. **Integration Tests** (`tests/hook_test.rs`):
   - Hook installation and uninstallation
   - Hook execution triggers snapshot
   - Hook fast-path (skips when loro.db not staged)
   - Hook failure aborts commit

4. **Manual Testing Guide** (`docs/testing/manual-snapshot-testing.md`):
   - Steps to test snapshot generation
   - Steps to test git hook behavior
   - Verification checklist for GitHub rendering

5. **Update PRD**:
   - Mark Phase 4 tasks as complete
   - Add any new technical decisions made during implementation

6. **Update README** (if exists):
   - Document `medulla snapshot` command
   - Document `medulla hook` commands

---

## File Summary

### New Files

| File | Description |
|------|-------------|
| `src/snapshot/mod.rs` | Module exports, main generate_snapshot() |
| `src/snapshot/frontmatter.rs` | YAML frontmatter utilities |
| `src/snapshot/utils.rs` | Slug generation, file writing |
| `src/snapshot/decision.rs` | Decision snapshot generation |
| `src/snapshot/task.rs` | Task snapshot generation |
| `src/snapshot/note.rs` | Note snapshot generation |
| `src/snapshot/prompt.rs` | Prompt snapshot generation |
| `src/snapshot/component.rs` | Component snapshot generation |
| `src/snapshot/link.rs` | Link snapshot generation |
| `src/snapshot/readme.rs` | README index generation |
| `src/hook.rs` | Git hook management |
| `tests/snapshot_test.rs` | Snapshot integration tests |
| `tests/hook_test.rs` | Hook integration tests |

### Modified Files

| File | Changes |
|------|---------|
| `src/lib.rs` | Export snapshot and hook modules |
| `src/cli/commands.rs` | Add Snapshot and Hook commands |
| `src/cli/handlers.rs` | Add snapshot and hook handlers |
| `src/main.rs` | Wire up new commands |
| `docs/prd.md` | Mark Phase 4 complete |

---

## Dependencies

No new crate dependencies required. Existing dependencies cover all needs:

- `serde` / `serde_yaml` - Frontmatter serialization (need to add `serde_yaml`)
- `chrono` - Date formatting
- `std::fs` - File operations

**Add to Cargo.toml**:
```toml
serde_yaml = "0.9"
```

---

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| Hook conflicts with user's existing hooks | Detect and require `--force`, offer backup |
| Large repos slow to snapshot | Fast-path in hook (skip if loro.db not staged) |
| Snapshot files conflict on merge | Snapshot is derived; regenerate on each branch |
| Cross-platform hook issues | Test on Linux/macOS/Windows; use portable shell |

---

## Success Criteria

1. `medulla snapshot` generates correct markdown for all entity types
2. Snapshot files render correctly on GitHub
3. Pre-commit hook only runs when `loro.db` is staged
4. Hook failure prevents commit with helpful error message
5. `git commit --no-verify` bypasses hook
6. All existing tests pass
7. New tests provide >80% coverage of snapshot module

---

## Estimated Scope

| Batch | New Lines (est.) | Test Lines (est.) |
|-------|------------------|-------------------|
| Batch 1 | ~200 | ~100 |
| Batch 2 | ~150 | ~80 |
| Batch 3 | ~200 | ~100 |
| Batch 4 | ~200 | ~80 |
| Batch 5 | ~300 | ~120 |
| Batch 6 | ~150 | ~100 |
| Batch 7 | ~250 | ~150 |
| Batch 8 | ~50 | ~200 |
| **Total** | **~1,500** | **~930** |

---

## Next Steps

1. Review and approve this plan
2. Begin Batch 1 implementation
3. Review after each batch before proceeding
