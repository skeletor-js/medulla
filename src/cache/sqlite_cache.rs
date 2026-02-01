use std::path::{Path, PathBuf};

use rusqlite::{params, Connection, OptionalExtension};

use crate::entity::{Component, Decision, Link, Note, Prompt, Relation, Task};
use crate::error::{MedullaError, Result};

const CACHE_DB: &str = "cache.db";

/// SQLite cache for full-text search and query acceleration
pub struct SqliteCache {
    conn: Connection,
    #[allow(dead_code)]
    path: PathBuf,
}

impl SqliteCache {
    /// Open or create the cache database
    pub fn open(medulla_dir: &Path) -> Result<Self> {
        let path = medulla_dir.join(CACHE_DB);
        let conn = Connection::open(&path)?;

        let cache = Self { conn, path };
        cache.init_schema()?;
        Ok(cache)
    }

    /// Initialize the database schema
    fn init_schema(&self) -> Result<()> {
        // Metadata table for version tracking
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS meta (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            )",
            [],
        )?;

        // Decisions table
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS decisions (
                id TEXT PRIMARY KEY,
                sequence_number INTEGER NOT NULL,
                title TEXT NOT NULL,
                content TEXT,
                status TEXT NOT NULL,
                context TEXT,
                tags TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                created_by TEXT
            )",
            [],
        )?;

        // FTS5 virtual table for full-text search on decisions
        self.conn.execute(
            "CREATE VIRTUAL TABLE IF NOT EXISTS decisions_fts USING fts5(
                id,
                title,
                content,
                context,
                tags,
                content='decisions',
                content_rowid='rowid'
            )",
            [],
        )?;

        // Triggers to keep FTS in sync with decisions table
        self.conn.execute_batch(
            "
            CREATE TRIGGER IF NOT EXISTS decisions_ai AFTER INSERT ON decisions BEGIN
                INSERT INTO decisions_fts(rowid, id, title, content, context, tags)
                VALUES (new.rowid, new.id, new.title, new.content, new.context, new.tags);
            END;

            CREATE TRIGGER IF NOT EXISTS decisions_ad AFTER DELETE ON decisions BEGIN
                INSERT INTO decisions_fts(decisions_fts, rowid, id, title, content, context, tags)
                VALUES ('delete', old.rowid, old.id, old.title, old.content, old.context, old.tags);
            END;

            CREATE TRIGGER IF NOT EXISTS decisions_au AFTER UPDATE ON decisions BEGIN
                INSERT INTO decisions_fts(decisions_fts, rowid, id, title, content, context, tags)
                VALUES ('delete', old.rowid, old.id, old.title, old.content, old.context, old.tags);
                INSERT INTO decisions_fts(rowid, id, title, content, context, tags)
                VALUES (new.rowid, new.id, new.title, new.content, new.context, new.tags);
            END;
            ",
        )?;

        // Tasks table
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS tasks (
                id TEXT PRIMARY KEY,
                sequence_number INTEGER NOT NULL,
                title TEXT NOT NULL,
                content TEXT,
                status TEXT NOT NULL,
                priority TEXT NOT NULL,
                due_date TEXT,
                assignee TEXT,
                tags TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                created_by TEXT
            )",
            [],
        )?;

        // FTS5 virtual table for full-text search on tasks
        self.conn.execute(
            "CREATE VIRTUAL TABLE IF NOT EXISTS tasks_fts USING fts5(
                id,
                title,
                content,
                status,
                priority,
                assignee,
                tags,
                content='tasks',
                content_rowid='rowid'
            )",
            [],
        )?;

        // Triggers to keep FTS in sync with tasks table
        self.conn.execute_batch(
            "
            CREATE TRIGGER IF NOT EXISTS tasks_ai AFTER INSERT ON tasks BEGIN
                INSERT INTO tasks_fts(rowid, id, title, content, status, priority, assignee, tags)
                VALUES (new.rowid, new.id, new.title, new.content, new.status, new.priority, new.assignee, new.tags);
            END;

            CREATE TRIGGER IF NOT EXISTS tasks_ad AFTER DELETE ON tasks BEGIN
                INSERT INTO tasks_fts(tasks_fts, rowid, id, title, content, status, priority, assignee, tags)
                VALUES ('delete', old.rowid, old.id, old.title, old.content, old.status, old.priority, old.assignee, old.tags);
            END;

            CREATE TRIGGER IF NOT EXISTS tasks_au AFTER UPDATE ON tasks BEGIN
                INSERT INTO tasks_fts(tasks_fts, rowid, id, title, content, status, priority, assignee, tags)
                VALUES ('delete', old.rowid, old.id, old.title, old.content, old.status, old.priority, old.assignee, old.tags);
                INSERT INTO tasks_fts(rowid, id, title, content, status, priority, assignee, tags)
                VALUES (new.rowid, new.id, new.title, new.content, new.status, new.priority, new.assignee, new.tags);
            END;
            ",
        )?;

        // Notes table
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS notes (
                id TEXT PRIMARY KEY,
                sequence_number INTEGER NOT NULL,
                title TEXT NOT NULL,
                content TEXT,
                note_type TEXT,
                tags TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                created_by TEXT
            )",
            [],
        )?;

        // FTS5 virtual table for full-text search on notes
        self.conn.execute(
            "CREATE VIRTUAL TABLE IF NOT EXISTS notes_fts USING fts5(
                id,
                title,
                content,
                note_type,
                tags,
                content='notes',
                content_rowid='rowid'
            )",
            [],
        )?;

        // Triggers to keep FTS in sync with notes table
        self.conn.execute_batch(
            "
            CREATE TRIGGER IF NOT EXISTS notes_ai AFTER INSERT ON notes BEGIN
                INSERT INTO notes_fts(rowid, id, title, content, note_type, tags)
                VALUES (new.rowid, new.id, new.title, new.content, new.note_type, new.tags);
            END;

            CREATE TRIGGER IF NOT EXISTS notes_ad AFTER DELETE ON notes BEGIN
                INSERT INTO notes_fts(notes_fts, rowid, id, title, content, note_type, tags)
                VALUES ('delete', old.rowid, old.id, old.title, old.content, old.note_type, old.tags);
            END;

            CREATE TRIGGER IF NOT EXISTS notes_au AFTER UPDATE ON notes BEGIN
                INSERT INTO notes_fts(notes_fts, rowid, id, title, content, note_type, tags)
                VALUES ('delete', old.rowid, old.id, old.title, old.content, old.note_type, old.tags);
                INSERT INTO notes_fts(rowid, id, title, content, note_type, tags)
                VALUES (new.rowid, new.id, new.title, new.content, new.note_type, new.tags);
            END;
            ",
        )?;

        // Prompts table
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS prompts (
                id TEXT PRIMARY KEY,
                sequence_number INTEGER NOT NULL,
                title TEXT NOT NULL,
                content TEXT,
                template TEXT,
                output_schema TEXT,
                variables TEXT,
                tags TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                created_by TEXT
            )",
            [],
        )?;

        // FTS5 virtual table for full-text search on prompts
        self.conn.execute(
            "CREATE VIRTUAL TABLE IF NOT EXISTS prompts_fts USING fts5(
                id,
                title,
                content,
                template,
                tags,
                content='prompts',
                content_rowid='rowid'
            )",
            [],
        )?;

        // Triggers to keep FTS in sync with prompts table
        self.conn.execute_batch(
            "
            CREATE TRIGGER IF NOT EXISTS prompts_ai AFTER INSERT ON prompts BEGIN
                INSERT INTO prompts_fts(rowid, id, title, content, template, tags)
                VALUES (new.rowid, new.id, new.title, new.content, new.template, new.tags);
            END;

            CREATE TRIGGER IF NOT EXISTS prompts_ad AFTER DELETE ON prompts BEGIN
                INSERT INTO prompts_fts(prompts_fts, rowid, id, title, content, template, tags)
                VALUES ('delete', old.rowid, old.id, old.title, old.content, old.template, old.tags);
            END;

            CREATE TRIGGER IF NOT EXISTS prompts_au AFTER UPDATE ON prompts BEGIN
                INSERT INTO prompts_fts(prompts_fts, rowid, id, title, content, template, tags)
                VALUES ('delete', old.rowid, old.id, old.title, old.content, old.template, old.tags);
                INSERT INTO prompts_fts(rowid, id, title, content, template, tags)
                VALUES (new.rowid, new.id, new.title, new.content, new.template, new.tags);
            END;
            ",
        )?;

        // Components table
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS components (
                id TEXT PRIMARY KEY,
                sequence_number INTEGER NOT NULL,
                title TEXT NOT NULL,
                content TEXT,
                status TEXT NOT NULL,
                component_type TEXT,
                owner TEXT,
                tags TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                created_by TEXT
            )",
            [],
        )?;

        // FTS5 virtual table for full-text search on components
        self.conn.execute(
            "CREATE VIRTUAL TABLE IF NOT EXISTS components_fts USING fts5(
                id,
                title,
                content,
                status,
                component_type,
                owner,
                tags,
                content='components',
                content_rowid='rowid'
            )",
            [],
        )?;

        // Triggers to keep FTS in sync with components table
        self.conn.execute_batch(
            "
            CREATE TRIGGER IF NOT EXISTS components_ai AFTER INSERT ON components BEGIN
                INSERT INTO components_fts(rowid, id, title, content, status, component_type, owner, tags)
                VALUES (new.rowid, new.id, new.title, new.content, new.status, new.component_type, new.owner, new.tags);
            END;

            CREATE TRIGGER IF NOT EXISTS components_ad AFTER DELETE ON components BEGIN
                INSERT INTO components_fts(components_fts, rowid, id, title, content, status, component_type, owner, tags)
                VALUES ('delete', old.rowid, old.id, old.title, old.content, old.status, old.component_type, old.owner, old.tags);
            END;

            CREATE TRIGGER IF NOT EXISTS components_au AFTER UPDATE ON components BEGIN
                INSERT INTO components_fts(components_fts, rowid, id, title, content, status, component_type, owner, tags)
                VALUES ('delete', old.rowid, old.id, old.title, old.content, old.status, old.component_type, old.owner, old.tags);
                INSERT INTO components_fts(rowid, id, title, content, status, component_type, owner, tags)
                VALUES (new.rowid, new.id, new.title, new.content, new.status, new.component_type, new.owner, new.tags);
            END;
            ",
        )?;

        // Links table
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS links (
                id TEXT PRIMARY KEY,
                sequence_number INTEGER NOT NULL,
                title TEXT NOT NULL,
                content TEXT,
                url TEXT NOT NULL,
                link_type TEXT,
                tags TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                created_by TEXT
            )",
            [],
        )?;

        // FTS5 virtual table for full-text search on links
        self.conn.execute(
            "CREATE VIRTUAL TABLE IF NOT EXISTS links_fts USING fts5(
                id,
                title,
                content,
                url,
                link_type,
                tags,
                content='links',
                content_rowid='rowid'
            )",
            [],
        )?;

        // Triggers to keep FTS in sync with links table
        self.conn.execute_batch(
            "
            CREATE TRIGGER IF NOT EXISTS links_ai AFTER INSERT ON links BEGIN
                INSERT INTO links_fts(rowid, id, title, content, url, link_type, tags)
                VALUES (new.rowid, new.id, new.title, new.content, new.url, new.link_type, new.tags);
            END;

            CREATE TRIGGER IF NOT EXISTS links_ad AFTER DELETE ON links BEGIN
                INSERT INTO links_fts(links_fts, rowid, id, title, content, url, link_type, tags)
                VALUES ('delete', old.rowid, old.id, old.title, old.content, old.url, old.link_type, old.tags);
            END;

            CREATE TRIGGER IF NOT EXISTS links_au AFTER UPDATE ON links BEGIN
                INSERT INTO links_fts(links_fts, rowid, id, title, content, url, link_type, tags)
                VALUES ('delete', old.rowid, old.id, old.title, old.content, old.url, old.link_type, old.tags);
                INSERT INTO links_fts(rowid, id, title, content, url, link_type, tags)
                VALUES (new.rowid, new.id, new.title, new.content, new.url, new.link_type, new.tags);
            END;
            ",
        )?;

        // Relations table with indexes for fast lookups
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS relations (
                composite_key TEXT PRIMARY KEY,
                source_id TEXT NOT NULL,
                source_type TEXT NOT NULL,
                target_id TEXT NOT NULL,
                target_type TEXT NOT NULL,
                relation_type TEXT NOT NULL,
                created_at TEXT NOT NULL,
                created_by TEXT
            )",
            [],
        )?;

        // Indexes for relation queries
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_relations_source ON relations(source_id)",
            [],
        )?;
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_relations_target ON relations(target_id)",
            [],
        )?;

        Ok(())
    }

    /// Get the stored Loro version hash
    pub fn get_loro_version(&self) -> Result<Option<String>> {
        let result: Option<String> = self
            .conn
            .query_row(
                "SELECT value FROM meta WHERE key = 'loro_version'",
                [],
                |row| row.get(0),
            )
            .optional()?;
        Ok(result)
    }

    /// Set the stored Loro version hash
    pub fn set_loro_version(&self, version: &str) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO meta (key, value) VALUES ('loro_version', ?1)",
            [version],
        )?;
        Ok(())
    }

    /// Index a decision in the cache
    pub fn index_decision(&self, decision: &Decision) -> Result<()> {
        let tags_str = decision.base.tags.join(", ");

        self.conn.execute(
            "INSERT OR REPLACE INTO decisions
             (id, sequence_number, title, content, status, context, tags, created_at, updated_at, created_by)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                decision.base.id.to_string(),
                decision.base.sequence_number,
                decision.base.title,
                decision.base.content,
                decision.status.to_string(),
                decision.context,
                tags_str,
                decision.base.created_at.to_rfc3339(),
                decision.base.updated_at.to_rfc3339(),
                decision.base.created_by,
            ],
        )?;

        Ok(())
    }

    /// Remove a decision from the cache
    pub fn remove_decision(&self, id: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM decisions WHERE id = ?1", [id])?;
        Ok(())
    }

    /// Index a task in the cache
    pub fn index_task(&self, task: &Task) -> Result<()> {
        let tags_str = task.base.tags.join(", ");

        self.conn.execute(
            "INSERT OR REPLACE INTO tasks
             (id, sequence_number, title, content, status, priority, due_date, assignee, tags, created_at, updated_at, created_by)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                task.base.id.to_string(),
                task.base.sequence_number,
                task.base.title,
                task.base.content,
                task.status.to_string(),
                task.priority.to_string(),
                task.due_date.map(|d| d.to_string()),
                task.assignee,
                tags_str,
                task.base.created_at.to_rfc3339(),
                task.base.updated_at.to_rfc3339(),
                task.base.created_by,
            ],
        )?;

        Ok(())
    }

    /// Remove a task from the cache
    pub fn remove_task(&self, id: &str) -> Result<()> {
        self.conn.execute("DELETE FROM tasks WHERE id = ?1", [id])?;
        Ok(())
    }

    /// Index a note in the cache
    pub fn index_note(&self, note: &Note) -> Result<()> {
        let tags_str = note.base.tags.join(", ");

        self.conn.execute(
            "INSERT OR REPLACE INTO notes
             (id, sequence_number, title, content, note_type, tags, created_at, updated_at, created_by)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                note.base.id.to_string(),
                note.base.sequence_number,
                note.base.title,
                note.base.content,
                note.note_type,
                tags_str,
                note.base.created_at.to_rfc3339(),
                note.base.updated_at.to_rfc3339(),
                note.base.created_by,
            ],
        )?;

        Ok(())
    }

    /// Remove a note from the cache
    pub fn remove_note(&self, id: &str) -> Result<()> {
        self.conn.execute("DELETE FROM notes WHERE id = ?1", [id])?;
        Ok(())
    }

    /// Index a prompt in the cache
    pub fn index_prompt(&self, prompt: &Prompt) -> Result<()> {
        let tags_str = prompt.base.tags.join(", ");
        let vars_str = prompt.variables.join(", ");

        self.conn.execute(
            "INSERT OR REPLACE INTO prompts
             (id, sequence_number, title, content, template, output_schema, variables, tags, created_at, updated_at, created_by)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                prompt.base.id.to_string(),
                prompt.base.sequence_number,
                prompt.base.title,
                prompt.base.content,
                prompt.template,
                prompt.output_schema,
                vars_str,
                tags_str,
                prompt.base.created_at.to_rfc3339(),
                prompt.base.updated_at.to_rfc3339(),
                prompt.base.created_by,
            ],
        )?;

        Ok(())
    }

    /// Remove a prompt from the cache
    pub fn remove_prompt(&self, id: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM prompts WHERE id = ?1", [id])?;
        Ok(())
    }

    /// Index a component in the cache
    pub fn index_component(&self, component: &Component) -> Result<()> {
        let tags_str = component.base.tags.join(", ");

        self.conn.execute(
            "INSERT OR REPLACE INTO components
             (id, sequence_number, title, content, status, component_type, owner, tags, created_at, updated_at, created_by)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                component.base.id.to_string(),
                component.base.sequence_number,
                component.base.title,
                component.base.content,
                component.status.to_string(),
                component.component_type,
                component.owner,
                tags_str,
                component.base.created_at.to_rfc3339(),
                component.base.updated_at.to_rfc3339(),
                component.base.created_by,
            ],
        )?;

        Ok(())
    }

    /// Remove a component from the cache
    pub fn remove_component(&self, id: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM components WHERE id = ?1", [id])?;
        Ok(())
    }

    /// Index a link in the cache
    pub fn index_link(&self, link: &Link) -> Result<()> {
        let tags_str = link.base.tags.join(", ");

        self.conn.execute(
            "INSERT OR REPLACE INTO links
             (id, sequence_number, title, content, url, link_type, tags, created_at, updated_at, created_by)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                link.base.id.to_string(),
                link.base.sequence_number,
                link.base.title,
                link.base.content,
                link.url,
                link.link_type,
                tags_str,
                link.base.created_at.to_rfc3339(),
                link.base.updated_at.to_rfc3339(),
                link.base.created_by,
            ],
        )?;

        Ok(())
    }

    /// Remove a link from the cache
    pub fn remove_link(&self, id: &str) -> Result<()> {
        self.conn.execute("DELETE FROM links WHERE id = ?1", [id])?;
        Ok(())
    }

    /// Index a relation in the cache
    pub fn index_relation(&self, relation: &Relation) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO relations
             (composite_key, source_id, source_type, target_id, target_type, relation_type, created_at, created_by)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                relation.composite_key(),
                relation.source_id.to_string(),
                relation.source_type,
                relation.target_id.to_string(),
                relation.target_type,
                relation.relation_type.to_string(),
                relation.created_at.to_rfc3339(),
                relation.created_by,
            ],
        )?;
        Ok(())
    }

    /// Remove a relation from the cache
    pub fn remove_relation(&self, composite_key: &str) -> Result<()> {
        self.conn.execute(
            "DELETE FROM relations WHERE composite_key = ?1",
            [composite_key],
        )?;
        Ok(())
    }

    /// Clear all cached data (for full rebuild)
    pub fn clear(&self) -> Result<()> {
        self.conn.execute("DELETE FROM decisions", [])?;
        self.conn.execute("DELETE FROM tasks", [])?;
        self.conn.execute("DELETE FROM notes", [])?;
        self.conn.execute("DELETE FROM prompts", [])?;
        self.conn.execute("DELETE FROM components", [])?;
        self.conn.execute("DELETE FROM links", [])?;
        self.conn.execute("DELETE FROM relations", [])?;
        self.conn.execute("DELETE FROM meta", [])?;
        Ok(())
    }

    /// Full-text search for decisions
    pub fn search_decisions(&self, query: &str, limit: i64) -> Result<Vec<DecisionSearchResult>> {
        let mut stmt = self.conn.prepare(
            "SELECT d.id, d.sequence_number, d.title, d.status,
                    highlight(decisions_fts, 1, '<mark>', '</mark>') as title_highlight,
                    snippet(decisions_fts, 2, '<mark>', '</mark>', '...', 32) as content_snippet
             FROM decisions_fts f
             JOIN decisions d ON d.id = f.id
             WHERE decisions_fts MATCH ?1
             ORDER BY rank
             LIMIT ?2",
        )?;

        let results = stmt
            .query_map(params![query, limit], |row| {
                Ok(DecisionSearchResult {
                    id: row.get(0)?,
                    sequence_number: row.get(1)?,
                    title: row.get(2)?,
                    status: row.get(3)?,
                    title_highlight: row.get(4)?,
                    content_snippet: row.get(5)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(results)
    }

    /// Full-text search for tasks
    pub fn search_tasks(&self, query: &str, limit: i64) -> Result<Vec<TaskSearchResult>> {
        let mut stmt = self.conn.prepare(
            "SELECT t.id, t.sequence_number, t.title, t.status, t.priority, t.assignee,
                    highlight(tasks_fts, 1, '<mark>', '</mark>') as title_highlight,
                    snippet(tasks_fts, 2, '<mark>', '</mark>', '...', 32) as content_snippet
             FROM tasks_fts f
             JOIN tasks t ON t.id = f.id
             WHERE tasks_fts MATCH ?1
             ORDER BY rank
             LIMIT ?2",
        )?;

        let results = stmt
            .query_map(params![query, limit], |row| {
                Ok(TaskSearchResult {
                    id: row.get(0)?,
                    sequence_number: row.get(1)?,
                    title: row.get(2)?,
                    status: row.get(3)?,
                    priority: row.get(4)?,
                    assignee: row.get(5)?,
                    title_highlight: row.get(6)?,
                    content_snippet: row.get(7)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(results)
    }

    /// Full-text search for notes
    pub fn search_notes(&self, query: &str, limit: i64) -> Result<Vec<NoteSearchResult>> {
        let mut stmt = self.conn.prepare(
            "SELECT n.id, n.sequence_number, n.title, n.note_type,
                    highlight(notes_fts, 1, '<mark>', '</mark>') as title_highlight,
                    snippet(notes_fts, 2, '<mark>', '</mark>', '...', 32) as content_snippet
             FROM notes_fts f
             JOIN notes n ON n.id = f.id
             WHERE notes_fts MATCH ?1
             ORDER BY rank
             LIMIT ?2",
        )?;

        let results = stmt
            .query_map(params![query, limit], |row| {
                Ok(NoteSearchResult {
                    id: row.get(0)?,
                    sequence_number: row.get(1)?,
                    title: row.get(2)?,
                    note_type: row.get(3)?,
                    title_highlight: row.get(4)?,
                    content_snippet: row.get(5)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(results)
    }

    /// Full-text search for prompts
    pub fn search_prompts(&self, query: &str, limit: i64) -> Result<Vec<PromptSearchResult>> {
        let mut stmt = self.conn.prepare(
            "SELECT p.id, p.sequence_number, p.title, p.variables,
                    highlight(prompts_fts, 1, '<mark>', '</mark>') as title_highlight,
                    snippet(prompts_fts, 2, '<mark>', '</mark>', '...', 32) as content_snippet
             FROM prompts_fts f
             JOIN prompts p ON p.id = f.id
             WHERE prompts_fts MATCH ?1
             ORDER BY rank
             LIMIT ?2",
        )?;

        let results = stmt
            .query_map(params![query, limit], |row| {
                let vars_str: String = row.get(3)?;
                let variables = vars_str
                    .split(", ")
                    .filter(|s| !s.is_empty())
                    .map(String::from)
                    .collect();
                Ok(PromptSearchResult {
                    id: row.get(0)?,
                    sequence_number: row.get(1)?,
                    title: row.get(2)?,
                    variables,
                    title_highlight: row.get(4)?,
                    content_snippet: row.get(5)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(results)
    }

    /// Full-text search for components
    pub fn search_components(&self, query: &str, limit: i64) -> Result<Vec<ComponentSearchResult>> {
        let mut stmt = self.conn.prepare(
            "SELECT c.id, c.sequence_number, c.title, c.status, c.component_type, c.owner,
                    highlight(components_fts, 1, '<mark>', '</mark>') as title_highlight,
                    snippet(components_fts, 2, '<mark>', '</mark>', '...', 32) as content_snippet
             FROM components_fts f
             JOIN components c ON c.id = f.id
             WHERE components_fts MATCH ?1
             ORDER BY rank
             LIMIT ?2",
        )?;

        let results = stmt
            .query_map(params![query, limit], |row| {
                Ok(ComponentSearchResult {
                    id: row.get(0)?,
                    sequence_number: row.get(1)?,
                    title: row.get(2)?,
                    status: row.get(3)?,
                    component_type: row.get(4)?,
                    owner: row.get(5)?,
                    title_highlight: row.get(6)?,
                    content_snippet: row.get(7)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(results)
    }

    /// Full-text search for links
    pub fn search_links(&self, query: &str, limit: i64) -> Result<Vec<LinkSearchResult>> {
        let mut stmt = self.conn.prepare(
            "SELECT l.id, l.sequence_number, l.title, l.url, l.link_type,
                    highlight(links_fts, 1, '<mark>', '</mark>') as title_highlight,
                    snippet(links_fts, 2, '<mark>', '</mark>', '...', 32) as content_snippet
             FROM links_fts f
             JOIN links l ON l.id = f.id
             WHERE links_fts MATCH ?1
             ORDER BY rank
             LIMIT ?2",
        )?;

        let results = stmt
            .query_map(params![query, limit], |row| {
                Ok(LinkSearchResult {
                    id: row.get(0)?,
                    sequence_number: row.get(1)?,
                    title: row.get(2)?,
                    url: row.get(3)?,
                    link_type: row.get(4)?,
                    title_highlight: row.get(5)?,
                    content_snippet: row.get(6)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(results)
    }

    /// Search across all entity types and return combined results
    pub fn search_all(&self, query: &str, limit: i64) -> Result<Vec<SearchResult>> {
        let mut all_results = Vec::new();

        // Search each entity type
        if let Ok(decisions) = self.search_decisions(query, limit) {
            for r in decisions {
                all_results.push(SearchResult::Decision(r));
            }
        }

        if let Ok(tasks) = self.search_tasks(query, limit) {
            for r in tasks {
                all_results.push(SearchResult::Task(r));
            }
        }

        if let Ok(notes) = self.search_notes(query, limit) {
            for r in notes {
                all_results.push(SearchResult::Note(r));
            }
        }

        if let Ok(prompts) = self.search_prompts(query, limit) {
            for r in prompts {
                all_results.push(SearchResult::Prompt(r));
            }
        }

        if let Ok(components) = self.search_components(query, limit) {
            for r in components {
                all_results.push(SearchResult::Component(r));
            }
        }

        if let Ok(links) = self.search_links(query, limit) {
            for r in links {
                all_results.push(SearchResult::Link(r));
            }
        }

        // Limit total results
        all_results.truncate(limit as usize);

        Ok(all_results)
    }

    /// Get relations from a source entity
    pub fn get_relations_from(&self, source_id: &str) -> Result<Vec<CachedRelation>> {
        let mut stmt = self.conn.prepare(
            "SELECT composite_key, source_id, source_type, target_id, target_type,
                    relation_type, created_at, created_by
             FROM relations WHERE source_id = ?1",
        )?;

        let results = stmt
            .query_map([source_id], |row| {
                Ok(CachedRelation {
                    composite_key: row.get(0)?,
                    source_id: row.get(1)?,
                    source_type: row.get(2)?,
                    target_id: row.get(3)?,
                    target_type: row.get(4)?,
                    relation_type: row.get(5)?,
                    created_at: row.get(6)?,
                    created_by: row.get(7)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(results)
    }

    /// Get relations to a target entity
    pub fn get_relations_to(&self, target_id: &str) -> Result<Vec<CachedRelation>> {
        let mut stmt = self.conn.prepare(
            "SELECT composite_key, source_id, source_type, target_id, target_type,
                    relation_type, created_at, created_by
             FROM relations WHERE target_id = ?1",
        )?;

        let results = stmt
            .query_map([target_id], |row| {
                Ok(CachedRelation {
                    composite_key: row.get(0)?,
                    source_id: row.get(1)?,
                    source_type: row.get(2)?,
                    target_id: row.get(3)?,
                    target_type: row.get(4)?,
                    relation_type: row.get(5)?,
                    created_at: row.get(6)?,
                    created_by: row.get(7)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(results)
    }

    /// Sync the cache with the Loro store
    /// Returns true if a full reindex was performed
    pub fn sync_from_loro(
        &self,
        decisions: &[Decision],
        relations: &[Relation],
        loro_version: &str,
    ) -> Result<bool> {
        let stored_version = self.get_loro_version()?;

        // If versions match, cache is up to date
        if stored_version.as_deref() == Some(loro_version) {
            return Ok(false);
        }

        // For now, do a full reindex
        // TODO: Implement incremental sync by tracking individual entity versions
        self.clear()?;

        for decision in decisions {
            self.index_decision(decision)?;
        }

        for relation in relations {
            self.index_relation(relation)?;
        }

        self.set_loro_version(loro_version)?;

        Ok(true)
    }
}

/// Search result from full-text search for decisions
#[derive(Debug, Clone)]
pub struct DecisionSearchResult {
    pub id: String,
    pub sequence_number: u32,
    pub title: String,
    pub status: String,
    pub title_highlight: Option<String>,
    pub content_snippet: Option<String>,
}

/// Search result from full-text search for tasks
#[derive(Debug, Clone)]
pub struct TaskSearchResult {
    pub id: String,
    pub sequence_number: u32,
    pub title: String,
    pub status: String,
    pub priority: String,
    pub assignee: Option<String>,
    pub title_highlight: Option<String>,
    pub content_snippet: Option<String>,
}

/// Search result from full-text search for notes
#[derive(Debug, Clone)]
pub struct NoteSearchResult {
    pub id: String,
    pub sequence_number: u32,
    pub title: String,
    pub note_type: Option<String>,
    pub title_highlight: Option<String>,
    pub content_snippet: Option<String>,
}

/// Search result from full-text search for prompts
#[derive(Debug, Clone)]
pub struct PromptSearchResult {
    pub id: String,
    pub sequence_number: u32,
    pub title: String,
    pub variables: Vec<String>,
    pub title_highlight: Option<String>,
    pub content_snippet: Option<String>,
}

/// Search result from full-text search for components
#[derive(Debug, Clone)]
pub struct ComponentSearchResult {
    pub id: String,
    pub sequence_number: u32,
    pub title: String,
    pub status: String,
    pub component_type: Option<String>,
    pub owner: Option<String>,
    pub title_highlight: Option<String>,
    pub content_snippet: Option<String>,
}

/// Search result from full-text search for links
#[derive(Debug, Clone)]
pub struct LinkSearchResult {
    pub id: String,
    pub sequence_number: u32,
    pub title: String,
    pub url: String,
    pub link_type: Option<String>,
    pub title_highlight: Option<String>,
    pub content_snippet: Option<String>,
}

/// Combined search result for all entity types
#[derive(Debug, Clone)]
pub enum SearchResult {
    Decision(DecisionSearchResult),
    Task(TaskSearchResult),
    Note(NoteSearchResult),
    Prompt(PromptSearchResult),
    Component(ComponentSearchResult),
    Link(LinkSearchResult),
}

/// Cached relation for fast queries
#[derive(Debug, Clone)]
pub struct CachedRelation {
    pub composite_key: String,
    pub source_id: String,
    pub source_type: String,
    pub target_id: String,
    pub target_type: String,
    pub relation_type: String,
    pub created_at: String,
    pub created_by: Option<String>,
}

// Implement From for rusqlite::Error
impl From<rusqlite::Error> for MedullaError {
    fn from(e: rusqlite::Error) -> Self {
        MedullaError::Storage(format!("SQLite error: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_cache_open_creates_db() {
        let tmp = TempDir::new().unwrap();
        let _cache = SqliteCache::open(tmp.path()).unwrap();
        assert!(tmp.path().join("cache.db").exists());
    }

    #[test]
    fn test_version_tracking() {
        let tmp = TempDir::new().unwrap();
        let cache = SqliteCache::open(tmp.path()).unwrap();

        // Initially no version
        assert!(cache.get_loro_version().unwrap().is_none());

        // Set version
        cache.set_loro_version("abc123").unwrap();
        assert_eq!(
            cache.get_loro_version().unwrap(),
            Some("abc123".to_string())
        );

        // Update version
        cache.set_loro_version("def456").unwrap();
        assert_eq!(
            cache.get_loro_version().unwrap(),
            Some("def456".to_string())
        );
    }

    #[test]
    fn test_index_and_search_decision() {
        let tmp = TempDir::new().unwrap();
        let cache = SqliteCache::open(tmp.path()).unwrap();

        let decision = crate::entity::Decision::new("Use PostgreSQL for database".to_string(), 1);
        cache.index_decision(&decision).unwrap();

        // Search for it
        let results = cache.search_decisions("PostgreSQL", 50).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Use PostgreSQL for database");

        // Search for non-matching term
        let results = cache.search_decisions("MySQL", 50).unwrap();
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_index_and_query_relations() {
        let tmp = TempDir::new().unwrap();
        let cache = SqliteCache::open(tmp.path()).unwrap();

        let source_id = uuid::Uuid::new_v4();
        let target_id = uuid::Uuid::new_v4();

        let relation = crate::entity::Relation::new(
            source_id,
            "decision".to_string(),
            target_id,
            "decision".to_string(),
            crate::entity::RelationType::Supersedes,
        );

        cache.index_relation(&relation).unwrap();

        // Query from source
        let from_results = cache.get_relations_from(&source_id.to_string()).unwrap();
        assert_eq!(from_results.len(), 1);
        assert_eq!(from_results[0].relation_type, "supersedes");

        // Query to target
        let to_results = cache.get_relations_to(&target_id.to_string()).unwrap();
        assert_eq!(to_results.len(), 1);
        assert_eq!(to_results[0].relation_type, "supersedes");
    }

    #[test]
    fn test_sync_from_loro() {
        let tmp = TempDir::new().unwrap();
        let cache = SqliteCache::open(tmp.path()).unwrap();

        let decision1 = crate::entity::Decision::new("Decision One".to_string(), 1);
        let decision2 = crate::entity::Decision::new("Decision Two".to_string(), 2);

        let decisions = vec![decision1.clone(), decision2.clone()];

        let relation = crate::entity::Relation::new(
            decision2.base.id,
            "decision".to_string(),
            decision1.base.id,
            "decision".to_string(),
            crate::entity::RelationType::Supersedes,
        );

        let relations = vec![relation];

        // First sync should reindex
        let reindexed = cache.sync_from_loro(&decisions, &relations, "v1").unwrap();
        assert!(reindexed);

        // Search should work
        let results = cache.search_decisions("Decision", 50).unwrap();
        assert_eq!(results.len(), 2);

        // Same version should skip
        let reindexed = cache.sync_from_loro(&decisions, &relations, "v1").unwrap();
        assert!(!reindexed);

        // New version should reindex
        let reindexed = cache.sync_from_loro(&decisions, &relations, "v2").unwrap();
        assert!(reindexed);
    }
}
