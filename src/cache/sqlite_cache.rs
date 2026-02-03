use std::path::{Path, PathBuf};

use chrono::Utc;
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

        // Embeddings table for semantic search
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS embeddings (
                entity_id TEXT PRIMARY KEY,
                entity_type TEXT NOT NULL,
                embedding BLOB NOT NULL,
                text_hash TEXT NOT NULL,
                computed_at TEXT NOT NULL
            )",
            [],
        )?;

        // Index for filtering embeddings by type
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_embeddings_type ON embeddings(entity_type)",
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
        self.conn.execute("DELETE FROM embeddings", [])?;
        self.conn.execute("DELETE FROM meta", [])?;
        Ok(())
    }

    // =========================================================================
    // Embedding Storage Methods
    // =========================================================================

    /// Store an embedding for an entity.
    /// The embedding is stored as a BLOB (f32 array serialized as bytes).
    pub fn store_embedding(
        &self,
        entity_id: &str,
        entity_type: &str,
        embedding: &[f32],
        text_hash: &str,
    ) -> Result<()> {
        // Convert f32 array to bytes
        let embedding_bytes = embedding_to_bytes(embedding);

        self.conn.execute(
            "INSERT OR REPLACE INTO embeddings
             (entity_id, entity_type, embedding, text_hash, computed_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                entity_id,
                entity_type,
                embedding_bytes,
                text_hash,
                Utc::now().to_rfc3339(),
            ],
        )?;

        Ok(())
    }

    /// Get an embedding for an entity.
    /// Returns None if no embedding exists.
    pub fn get_embedding(&self, entity_id: &str) -> Result<Option<Vec<f32>>> {
        let result: Option<Vec<u8>> = self
            .conn
            .query_row(
                "SELECT embedding FROM embeddings WHERE entity_id = ?1",
                [entity_id],
                |row| row.get(0),
            )
            .optional()?;

        Ok(result.map(|bytes| bytes_to_embedding(&bytes)))
    }

    /// Get the text hash for an entity's embedding.
    /// Used to check if content has changed and embedding needs recomputing.
    pub fn get_embedding_text_hash(&self, entity_id: &str) -> Result<Option<String>> {
        let result: Option<String> = self
            .conn
            .query_row(
                "SELECT text_hash FROM embeddings WHERE entity_id = ?1",
                [entity_id],
                |row| row.get(0),
            )
            .optional()?;
        Ok(result)
    }

    /// Delete an embedding for an entity.
    pub fn delete_embedding(&self, entity_id: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM embeddings WHERE entity_id = ?1", [entity_id])?;
        Ok(())
    }

    /// List all embeddings for a specific entity type.
    /// Returns tuples of (entity_id, embedding).
    pub fn list_embeddings_by_type(&self, entity_type: &str) -> Result<Vec<(String, Vec<f32>)>> {
        let mut stmt = self
            .conn
            .prepare("SELECT entity_id, embedding FROM embeddings WHERE entity_type = ?1")?;

        let results = stmt
            .query_map([entity_type], |row| {
                let entity_id: String = row.get(0)?;
                let embedding_bytes: Vec<u8> = row.get(1)?;
                Ok((entity_id, bytes_to_embedding(&embedding_bytes)))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(results)
    }

    /// List all embeddings (optionally filtered by type).
    /// Returns tuples of (entity_id, entity_type, embedding).
    pub fn list_all_embeddings(
        &self,
        entity_type: Option<&str>,
    ) -> Result<Vec<(String, String, Vec<f32>)>> {
        if let Some(etype) = entity_type {
            let mut stmt = self.conn.prepare(
                "SELECT entity_id, entity_type, embedding FROM embeddings WHERE entity_type = ?1",
            )?;
            let results = stmt
                .query_map([etype], |row| {
                    let entity_id: String = row.get(0)?;
                    let entity_type: String = row.get(1)?;
                    let embedding_bytes: Vec<u8> = row.get(2)?;
                    Ok((entity_id, entity_type, bytes_to_embedding(&embedding_bytes)))
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            Ok(results)
        } else {
            let mut stmt = self
                .conn
                .prepare("SELECT entity_id, entity_type, embedding FROM embeddings")?;
            let results = stmt
                .query_map([], |row| {
                    let entity_id: String = row.get(0)?;
                    let entity_type: String = row.get(1)?;
                    let embedding_bytes: Vec<u8> = row.get(2)?;
                    Ok((entity_id, entity_type, bytes_to_embedding(&embedding_bytes)))
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            Ok(results)
        }
    }

    /// Get the count of embeddings in the cache.
    pub fn count_embeddings(&self) -> Result<usize> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM embeddings", [], |row| row.get(0))?;
        Ok(count as usize)
    }

    /// Get cache statistics for monitoring and threshold warnings.
    pub fn get_stats(&self) -> Result<CacheStats> {
        let decisions: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM decisions", [], |row| row.get(0))?;
        let tasks: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM tasks", [], |row| row.get(0))?;
        let notes: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM notes", [], |row| row.get(0))?;
        let prompts: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM prompts", [], |row| row.get(0))?;
        let components: i64 =
            self.conn
                .query_row("SELECT COUNT(*) FROM components", [], |row| row.get(0))?;
        let links: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM links", [], |row| row.get(0))?;
        let relations: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM relations", [], |row| row.get(0))?;
        let embeddings: i64 =
            self.conn
                .query_row("SELECT COUNT(*) FROM embeddings", [], |row| row.get(0))?;

        let entity_count = (decisions + tasks + notes + prompts + components + links) as usize;

        Ok(CacheStats {
            entity_count,
            embedding_count: embeddings as usize,
            decisions: decisions as usize,
            tasks: tasks as usize,
            notes: notes as usize,
            prompts: prompts as usize,
            components: components as usize,
            links: links as usize,
            relations: relations as usize,
        })
    }

    /// Compute and store an embedding for an entity if the content has changed.
    /// Returns true if a new embedding was computed, false if skipped (unchanged).
    ///
    /// Uses the Embedder to compute embeddings for the entity's embeddable text
    /// (title + content + tags). Skips computation if the text hash matches
    /// the previously stored hash.
    pub fn compute_and_store_embedding_if_changed(
        &self,
        entity_id: &str,
        entity_type: &str,
        title: &str,
        content: Option<&str>,
        tags: &[String],
        embedder: &crate::embeddings::Embedder,
    ) -> Result<bool> {
        let text = embeddable_text(title, content, tags);
        let text_hash = compute_text_hash(&text);

        // Check if we already have an embedding with this hash
        if let Some(stored_hash) = self.get_embedding_text_hash(entity_id)? {
            if stored_hash == text_hash {
                // Content hasn't changed, skip embedding computation
                return Ok(false);
            }
        }

        // Compute new embedding
        let embedding = embedder.embed(&text)?;

        // Store it
        self.store_embedding(entity_id, entity_type, &embedding, &text_hash)?;

        Ok(true)
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

    /// Search a specific entity type with full-text search.
    pub fn search_by_type(
        &self,
        entity_type: &str,
        query: &str,
        limit: i64,
    ) -> Result<Vec<SearchResult>> {
        match entity_type {
            "decision" => {
                let results = self.search_decisions(query, limit)?;
                Ok(results.into_iter().map(SearchResult::Decision).collect())
            }
            "task" => {
                let results = self.search_tasks(query, limit)?;
                Ok(results.into_iter().map(SearchResult::Task).collect())
            }
            "note" => {
                let results = self.search_notes(query, limit)?;
                Ok(results.into_iter().map(SearchResult::Note).collect())
            }
            "prompt" => {
                let results = self.search_prompts(query, limit)?;
                Ok(results.into_iter().map(SearchResult::Prompt).collect())
            }
            "component" => {
                let results = self.search_components(query, limit)?;
                Ok(results.into_iter().map(SearchResult::Component).collect())
            }
            "link" => {
                let results = self.search_links(query, limit)?;
                Ok(results.into_iter().map(SearchResult::Link).collect())
            }
            _ => Ok(Vec::new()),
        }
    }

    // =========================================================================
    // Semantic Search
    // =========================================================================

    /// Search entities by semantic similarity using vector embeddings.
    ///
    /// Computes cosine similarity between the query embedding and all stored
    /// embeddings, returning the top results above the threshold.
    ///
    /// # Arguments
    /// * `query_embedding` - The embedding vector for the query text
    /// * `entity_type` - Optional filter to only search specific entity types
    /// * `limit` - Maximum number of results to return
    /// * `threshold` - Minimum similarity score (0.0 to 1.0) for results
    pub fn search_semantic(
        &self,
        query_embedding: &[f32],
        entity_type: Option<&str>,
        limit: usize,
        threshold: f32,
    ) -> Result<Vec<SemanticSearchResult>> {
        // Load all embeddings (optionally filtered by type)
        let embeddings = self.list_all_embeddings(entity_type)?;

        // Compute similarity and collect results
        let mut results: Vec<SemanticSearchResult> = embeddings
            .into_iter()
            .map(|(entity_id, entity_type, embedding)| {
                let score = cosine_similarity(query_embedding, &embedding);
                (entity_id, entity_type, score)
            })
            .filter(|(_, _, score)| *score >= threshold)
            .collect::<Vec<_>>()
            .into_iter()
            .filter_map(|(entity_id, entity_type, score)| {
                // Look up metadata for this entity
                let metadata = self.get_entity_metadata(&entity_id, &entity_type).ok()?;
                metadata.map(|(seq, title)| SemanticSearchResult {
                    entity_id,
                    entity_type,
                    sequence_number: seq,
                    title,
                    score,
                })
            })
            .collect();

        // Sort by score descending
        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Limit results
        results.truncate(limit);

        Ok(results)
    }

    /// Get basic metadata (sequence_number, title) for an entity.
    fn get_entity_metadata(
        &self,
        entity_id: &str,
        entity_type: &str,
    ) -> Result<Option<(u32, String)>> {
        let table = match entity_type {
            "decision" => "decisions",
            "task" => "tasks",
            "note" => "notes",
            "prompt" => "prompts",
            "component" => "components",
            "link" => "links",
            _ => return Ok(None),
        };

        let query = format!("SELECT sequence_number, title FROM {} WHERE id = ?1", table);

        let result: Option<(u32, String)> = self
            .conn
            .query_row(&query, [entity_id], |row| Ok((row.get(0)?, row.get(1)?)))
            .optional()?;

        Ok(result)
    }

    /// Get filter-relevant metadata for an entity.
    /// Returns status (if applicable), tags, and created_at for filter matching.
    pub fn get_filter_metadata(
        &self,
        entity_id: &str,
        entity_type: &str,
    ) -> Result<Option<FilterMetadata>> {
        // Build query based on entity type (different tables have different columns)
        let (query, has_status) = match entity_type {
            "decision" => (
                "SELECT status, tags, created_at FROM decisions WHERE id = ?1",
                true,
            ),
            "task" => (
                "SELECT status, tags, created_at FROM tasks WHERE id = ?1",
                true,
            ),
            "component" => (
                "SELECT status, tags, created_at FROM components WHERE id = ?1",
                true,
            ),
            "note" => (
                "SELECT NULL as status, tags, created_at FROM notes WHERE id = ?1",
                false,
            ),
            "prompt" => (
                "SELECT NULL as status, tags, created_at FROM prompts WHERE id = ?1",
                false,
            ),
            "link" => (
                "SELECT NULL as status, tags, created_at FROM links WHERE id = ?1",
                false,
            ),
            _ => return Ok(None),
        };

        let result: Option<(Option<String>, Option<String>, String)> = self
            .conn
            .query_row(query, [entity_id], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })
            .optional()?;

        Ok(result.map(|(status_opt, tags_str, created_at_str)| {
            // Parse tags from comma-separated string
            let tags = tags_str
                .map(|s| {
                    s.split(',')
                        .map(|t| t.trim().to_string())
                        .filter(|t| !t.is_empty())
                        .collect()
                })
                .unwrap_or_default();

            // Parse created_at
            let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
                .ok()
                .map(|dt| dt.with_timezone(&Utc));

            FilterMetadata {
                status: if has_status { status_opt } else { None },
                tags,
                created_at,
            }
        }))
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

    /// Sync the cache with the Loro store (decisions and relations only - legacy)
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

    /// Sync the cache with all entity types from the Loro store
    /// Returns true if a full reindex was performed
    #[allow(clippy::too_many_arguments)]
    pub fn sync_from_loro_full(
        &self,
        decisions: &[Decision],
        tasks: &[Task],
        notes: &[Note],
        prompts: &[Prompt],
        components: &[Component],
        links: &[Link],
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

        for task in tasks {
            self.index_task(task)?;
        }

        for note in notes {
            self.index_note(note)?;
        }

        for prompt in prompts {
            self.index_prompt(prompt)?;
        }

        for component in components {
            self.index_component(component)?;
        }

        for link in links {
            self.index_link(link)?;
        }

        for relation in relations {
            self.index_relation(relation)?;
        }

        self.set_loro_version(loro_version)?;

        Ok(true)
    }

    // =========================================================================
    // Task Queue Queries (Beads Parity)
    // =========================================================================

    /// Get tasks that are ready to work on (no unresolved blockers).
    ///
    /// A task is "ready" if:
    /// - Its status is not "done"
    /// - It has no incoming "blocks" relations from tasks that are not "done"
    ///
    /// Results are sorted by:
    /// 1. Priority (urgent > high > normal > low)
    /// 2. Due date (earliest first, nulls last)
    /// 3. Sequence number (oldest first)
    pub fn get_ready_tasks(&self, limit: Option<u32>) -> Result<Vec<ReadyTask>> {
        let limit = limit.unwrap_or(50).min(100) as i64;

        // Query for tasks that:
        // 1. Are not done
        // 2. Have no blocking relations from non-done tasks
        //
        // The subquery finds all task IDs that ARE blocked by non-done tasks,
        // and we exclude those from our results.
        let mut stmt = self.conn.prepare(
            "SELECT t.id, t.sequence_number, t.title, t.status, t.priority, t.due_date, t.assignee
             FROM tasks t
             WHERE t.status != 'done'
               AND t.id NOT IN (
                   -- Tasks that have at least one non-done blocker
                   SELECT r.target_id
                   FROM relations r
                   JOIN tasks blocker ON blocker.id = r.source_id
                   WHERE r.relation_type = 'blocks'
                     AND blocker.status != 'done'
               )
             ORDER BY
               CASE t.priority
                 WHEN 'urgent' THEN 1
                 WHEN 'high' THEN 2
                 WHEN 'normal' THEN 3
                 WHEN 'low' THEN 4
                 ELSE 5
               END,
               CASE WHEN t.due_date IS NULL THEN 1 ELSE 0 END,
               t.due_date,
               t.sequence_number
             LIMIT ?1",
        )?;

        let results = stmt
            .query_map(params![limit], |row: &rusqlite::Row| {
                Ok(ReadyTask {
                    id: row.get(0)?,
                    sequence_number: row.get(1)?,
                    title: row.get(2)?,
                    status: row.get(3)?,
                    priority: row.get(4)?,
                    due_date: row.get(5)?,
                    assignee: row.get(6)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(results)
    }

    /// Get all blocked tasks with their blockers.
    ///
    /// A task is "blocked" if:
    /// - Its status is not "done"
    /// - It has at least one incoming "blocks" relation from a task that is not "done"
    ///
    /// Each blocked task includes a list of the tasks that are blocking it.
    pub fn get_blocked_tasks(&self, limit: Option<u32>) -> Result<Vec<BlockedTask>> {
        let limit = limit.unwrap_or(50).min(100) as i64;

        // First, get all blocked tasks (tasks with non-done blockers)
        let mut task_stmt = self.conn.prepare(
            "SELECT DISTINCT t.id, t.sequence_number, t.title, t.status, t.priority, t.due_date, t.assignee
             FROM tasks t
             WHERE t.status != 'done'
               AND t.id IN (
                   -- Tasks that have at least one non-done blocker
                   SELECT r.target_id
                   FROM relations r
                   JOIN tasks blocker ON blocker.id = r.source_id
                   WHERE r.relation_type = 'blocks'
                     AND blocker.status != 'done'
               )
             ORDER BY
               CASE t.priority
                 WHEN 'urgent' THEN 1
                 WHEN 'high' THEN 2
                 WHEN 'normal' THEN 3
                 WHEN 'low' THEN 4
                 ELSE 5
               END,
               t.sequence_number
             LIMIT ?1",
        )?;

        #[allow(clippy::type_complexity)]
        let tasks: Vec<(
            String,
            u32,
            String,
            String,
            String,
            Option<String>,
            Option<String>,
        )> = task_stmt
            .query_map(params![limit], |row: &rusqlite::Row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                ))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        // For each blocked task, get its blockers
        let mut blocker_stmt = self.conn.prepare(
            "SELECT blocker.id, blocker.sequence_number, blocker.title, blocker.status
             FROM relations r
             JOIN tasks blocker ON blocker.id = r.source_id
             WHERE r.relation_type = 'blocks'
               AND r.target_id = ?1
               AND blocker.status != 'done'
             ORDER BY blocker.sequence_number",
        )?;

        let mut results = Vec::new();
        for (id, seq, title, status, priority, due_date, assignee) in tasks {
            let blockers: Vec<TaskBlocker> = blocker_stmt
                .query_map(params![&id], |row: &rusqlite::Row| {
                    Ok(TaskBlocker {
                        id: row.get(0)?,
                        sequence_number: row.get(1)?,
                        title: row.get(2)?,
                        status: row.get(3)?,
                    })
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?;

            results.push(BlockedTask {
                id,
                sequence_number: seq,
                title,
                status,
                priority,
                due_date,
                assignee,
                blockers,
            });
        }

        Ok(results)
    }

    /// Get the blockers for a specific task.
    ///
    /// Returns all non-done tasks that block the specified task.
    /// Returns an empty list if the task has no blockers or doesn't exist.
    pub fn get_task_blockers(&self, task_id: &str) -> Result<Vec<TaskBlocker>> {
        let mut stmt = self.conn.prepare(
            "SELECT blocker.id, blocker.sequence_number, blocker.title, blocker.status
             FROM relations r
             JOIN tasks blocker ON blocker.id = r.source_id
             WHERE r.relation_type = 'blocks'
               AND r.target_id = ?1
               AND blocker.status != 'done'
             ORDER BY blocker.sequence_number",
        )?;

        let results = stmt
            .query_map(params![task_id], |row: &rusqlite::Row| {
                Ok(TaskBlocker {
                    id: row.get(0)?,
                    sequence_number: row.get(1)?,
                    title: row.get(2)?,
                    status: row.get(3)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(results)
    }

    /// Get the single highest-priority ready task.
    ///
    /// Convenience method that returns the first task from `get_ready_tasks(limit=1)`.
    pub fn get_next_task(&self) -> Result<Option<ReadyTask>> {
        let tasks = self.get_ready_tasks(Some(1))?;
        Ok(tasks.into_iter().next())
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

/// Result from semantic similarity search
#[derive(Debug, Clone, serde::Serialize)]
pub struct SemanticSearchResult {
    pub entity_id: String,
    pub entity_type: String,
    pub sequence_number: u32,
    pub title: String,
    pub score: f32,
}

/// Metadata for filter matching
#[derive(Debug, Clone)]
pub struct FilterMetadata {
    pub status: Option<String>,
    pub tags: Vec<String>,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
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

/// A task that is ready to work on (no unresolved blockers)
#[derive(Debug, Clone, serde::Serialize)]
pub struct ReadyTask {
    pub id: String,
    pub sequence_number: u32,
    pub title: String,
    pub status: String,
    pub priority: String,
    pub due_date: Option<String>,
    pub assignee: Option<String>,
}

/// A blocked task with information about what blocks it
#[derive(Debug, Clone, serde::Serialize)]
pub struct BlockedTask {
    pub id: String,
    pub sequence_number: u32,
    pub title: String,
    pub status: String,
    pub priority: String,
    pub due_date: Option<String>,
    pub assignee: Option<String>,
    pub blockers: Vec<TaskBlocker>,
}

/// Information about a task that blocks another task
#[derive(Debug, Clone, serde::Serialize)]
pub struct TaskBlocker {
    pub id: String,
    pub sequence_number: u32,
    pub title: String,
    pub status: String,
}

// =========================================================================
// Cache Statistics
// =========================================================================

/// Warning threshold for entity count.
pub const ENTITY_WARNING_THRESHOLD: usize = 1000;

/// Warning threshold for loro.db size in bytes (10MB).
pub const LORO_SIZE_WARNING_THRESHOLD: u64 = 10 * 1024 * 1024;

/// Cache statistics for monitoring and warnings.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CacheStats {
    /// Total count of all entities
    pub entity_count: usize,
    /// Total count of embeddings
    pub embedding_count: usize,
    /// Count of decisions
    pub decisions: usize,
    /// Count of tasks
    pub tasks: usize,
    /// Count of notes
    pub notes: usize,
    /// Count of prompts
    pub prompts: usize,
    /// Count of components
    pub components: usize,
    /// Count of links
    pub links: usize,
    /// Count of relations
    pub relations: usize,
}

// =========================================================================
// Embedding Helper Functions
// =========================================================================

/// Compute cosine similarity between two vectors.
/// Returns a value between -1 and 1, where 1 means identical direction.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot / (norm_a * norm_b)
    }
}

/// Extract embeddable text from an entity's fields.
/// Combines title, content, and tags into a single string for embedding.
pub fn embeddable_text(title: &str, content: Option<&str>, tags: &[String]) -> String {
    let mut text = title.to_string();
    if let Some(c) = content {
        text.push_str("\n\n");
        text.push_str(c);
    }
    if !tags.is_empty() {
        text.push_str("\n\n");
        text.push_str(&tags.join(" "));
    }
    text
}

/// Convert an f32 embedding vector to bytes for storage.
fn embedding_to_bytes(embedding: &[f32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(embedding.len() * 4);
    for &value in embedding {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes
}

/// Convert bytes back to an f32 embedding vector.
fn bytes_to_embedding(bytes: &[u8]) -> Vec<f32> {
    bytes
        .chunks_exact(4)
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect()
}

/// Compute a simple hash of text for change detection.
/// Uses a fast non-cryptographic hash.
pub fn compute_text_hash(text: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
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

    // =========================================================================
    // Task Queue Tests (Beads Parity)
    // =========================================================================

    use crate::entity::{RelationType, TaskPriority, TaskStatus};

    fn create_task(title: &str, seq: u32, status: TaskStatus, priority: TaskPriority) -> Task {
        let mut task = Task::new(title.to_string(), seq);
        task.status = status;
        task.priority = priority;
        task
    }

    #[test]
    fn test_get_ready_tasks_no_blockers() {
        let tmp = TempDir::new().unwrap();
        let cache = SqliteCache::open(tmp.path()).unwrap();

        // Create some tasks
        let task1 = create_task("Task 1", 1, TaskStatus::Todo, TaskPriority::Normal);
        let task2 = create_task("Task 2", 2, TaskStatus::InProgress, TaskPriority::High);
        let task3 = create_task("Task 3", 3, TaskStatus::Done, TaskPriority::Urgent);

        cache.index_task(&task1).unwrap();
        cache.index_task(&task2).unwrap();
        cache.index_task(&task3).unwrap();

        // Get ready tasks
        let ready = cache.get_ready_tasks(None).unwrap();

        // Should have 2 ready tasks (task3 is done)
        assert_eq!(ready.len(), 2);

        // Should be sorted by priority (high before normal)
        assert_eq!(ready[0].title, "Task 2"); // High priority
        assert_eq!(ready[1].title, "Task 1"); // Normal priority
    }

    #[test]
    fn test_get_ready_tasks_with_blockers() {
        let tmp = TempDir::new().unwrap();
        let cache = SqliteCache::open(tmp.path()).unwrap();

        // Create tasks
        let task1 = create_task("Blocker Task", 1, TaskStatus::Todo, TaskPriority::Normal);
        let task2 = create_task("Blocked Task", 2, TaskStatus::Todo, TaskPriority::Urgent);
        let task3 = create_task("Free Task", 3, TaskStatus::Todo, TaskPriority::Low);

        cache.index_task(&task1).unwrap();
        cache.index_task(&task2).unwrap();
        cache.index_task(&task3).unwrap();

        // Task1 blocks Task2
        let relation = Relation::new(
            task1.base.id,
            "task".to_string(),
            task2.base.id,
            "task".to_string(),
            RelationType::Blocks,
        );
        cache.index_relation(&relation).unwrap();

        // Get ready tasks
        let ready = cache.get_ready_tasks(None).unwrap();

        // Should have 2 ready tasks (task2 is blocked)
        assert_eq!(ready.len(), 2);

        // Task2 (urgent but blocked) should NOT be in the list
        let titles: Vec<&str> = ready.iter().map(|t| t.title.as_str()).collect();
        assert!(!titles.contains(&"Blocked Task"));
        assert!(titles.contains(&"Blocker Task"));
        assert!(titles.contains(&"Free Task"));
    }

    #[test]
    fn test_get_ready_tasks_done_blocker_releases_task() {
        let tmp = TempDir::new().unwrap();
        let cache = SqliteCache::open(tmp.path()).unwrap();

        // Create tasks where blocker is done
        let task1 = create_task("Done Blocker", 1, TaskStatus::Done, TaskPriority::Normal);
        let task2 = create_task("Released Task", 2, TaskStatus::Todo, TaskPriority::High);

        cache.index_task(&task1).unwrap();
        cache.index_task(&task2).unwrap();

        // Task1 blocks Task2, but Task1 is done
        let relation = Relation::new(
            task1.base.id,
            "task".to_string(),
            task2.base.id,
            "task".to_string(),
            RelationType::Blocks,
        );
        cache.index_relation(&relation).unwrap();

        // Get ready tasks
        let ready = cache.get_ready_tasks(None).unwrap();

        // Task2 should be ready because its blocker is done
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].title, "Released Task");
    }

    #[test]
    fn test_get_ready_tasks_priority_and_due_date_ordering() {
        let tmp = TempDir::new().unwrap();
        let cache = SqliteCache::open(tmp.path()).unwrap();

        use chrono::NaiveDate;

        // Create tasks with various priorities and due dates
        let task1 = create_task("Normal no date", 1, TaskStatus::Todo, TaskPriority::Normal);
        let mut task2 = create_task(
            "Normal early date",
            2,
            TaskStatus::Todo,
            TaskPriority::Normal,
        );
        task2.due_date = Some(NaiveDate::from_ymd_opt(2025, 1, 15).unwrap());
        let mut task3 = create_task(
            "Normal late date",
            3,
            TaskStatus::Todo,
            TaskPriority::Normal,
        );
        task3.due_date = Some(NaiveDate::from_ymd_opt(2025, 2, 15).unwrap());
        let task4 = create_task("High no date", 4, TaskStatus::Todo, TaskPriority::High);

        cache.index_task(&task1).unwrap();
        cache.index_task(&task2).unwrap();
        cache.index_task(&task3).unwrap();
        cache.index_task(&task4).unwrap();

        let ready = cache.get_ready_tasks(None).unwrap();

        assert_eq!(ready.len(), 4);
        // High priority first
        assert_eq!(ready[0].title, "High no date");
        // Then normal priority, sorted by due date (earliest first, nulls last)
        assert_eq!(ready[1].title, "Normal early date");
        assert_eq!(ready[2].title, "Normal late date");
        assert_eq!(ready[3].title, "Normal no date");
    }

    #[test]
    fn test_get_blocked_tasks() {
        let tmp = TempDir::new().unwrap();
        let cache = SqliteCache::open(tmp.path()).unwrap();

        // Create tasks
        let task1 = create_task("Blocker 1", 1, TaskStatus::Todo, TaskPriority::Normal);
        let task2 = create_task("Blocker 2", 2, TaskStatus::InProgress, TaskPriority::Normal);
        let task3 = create_task("Blocked Task", 3, TaskStatus::Todo, TaskPriority::Urgent);
        let task4 = create_task("Free Task", 4, TaskStatus::Todo, TaskPriority::Normal);

        cache.index_task(&task1).unwrap();
        cache.index_task(&task2).unwrap();
        cache.index_task(&task3).unwrap();
        cache.index_task(&task4).unwrap();

        // Task1 and Task2 both block Task3
        let relation1 = Relation::new(
            task1.base.id,
            "task".to_string(),
            task3.base.id,
            "task".to_string(),
            RelationType::Blocks,
        );
        let relation2 = Relation::new(
            task2.base.id,
            "task".to_string(),
            task3.base.id,
            "task".to_string(),
            RelationType::Blocks,
        );
        cache.index_relation(&relation1).unwrap();
        cache.index_relation(&relation2).unwrap();

        // Get blocked tasks
        let blocked = cache.get_blocked_tasks(None).unwrap();

        // Should have 1 blocked task
        assert_eq!(blocked.len(), 1);
        assert_eq!(blocked[0].title, "Blocked Task");
        assert_eq!(blocked[0].blockers.len(), 2);

        // Blockers should be sorted by sequence number
        assert_eq!(blocked[0].blockers[0].title, "Blocker 1");
        assert_eq!(blocked[0].blockers[1].title, "Blocker 2");
    }

    #[test]
    fn test_get_task_blockers() {
        let tmp = TempDir::new().unwrap();
        let cache = SqliteCache::open(tmp.path()).unwrap();

        // Create tasks
        let task1 = create_task("Blocker", 1, TaskStatus::Todo, TaskPriority::Normal);
        let task2 = create_task("Blocked", 2, TaskStatus::Todo, TaskPriority::Normal);

        cache.index_task(&task1).unwrap();
        cache.index_task(&task2).unwrap();

        // Task1 blocks Task2
        let relation = Relation::new(
            task1.base.id,
            "task".to_string(),
            task2.base.id,
            "task".to_string(),
            RelationType::Blocks,
        );
        cache.index_relation(&relation).unwrap();

        // Get blockers for Task2
        let blockers = cache.get_task_blockers(&task2.base.id.to_string()).unwrap();

        assert_eq!(blockers.len(), 1);
        assert_eq!(blockers[0].title, "Blocker");
    }

    #[test]
    fn test_get_task_blockers_nonexistent_task() {
        let tmp = TempDir::new().unwrap();
        let cache = SqliteCache::open(tmp.path()).unwrap();

        // Get blockers for non-existent task
        let blockers = cache.get_task_blockers("nonexistent-id").unwrap();

        // Should return empty list, not error
        assert_eq!(blockers.len(), 0);
    }

    #[test]
    fn test_get_next_task() {
        let tmp = TempDir::new().unwrap();
        let cache = SqliteCache::open(tmp.path()).unwrap();

        // Create tasks
        let task1 = create_task("Low priority", 1, TaskStatus::Todo, TaskPriority::Low);
        let task2 = create_task("Urgent priority", 2, TaskStatus::Todo, TaskPriority::Urgent);

        cache.index_task(&task1).unwrap();
        cache.index_task(&task2).unwrap();

        // Get next task
        let next = cache.get_next_task().unwrap();

        assert!(next.is_some());
        assert_eq!(next.unwrap().title, "Urgent priority");
    }

    #[test]
    fn test_get_next_task_no_tasks() {
        let tmp = TempDir::new().unwrap();
        let cache = SqliteCache::open(tmp.path()).unwrap();

        // Get next task when none exist
        let next = cache.get_next_task().unwrap();

        assert!(next.is_none());
    }

    #[test]
    fn test_get_ready_tasks_limit() {
        let tmp = TempDir::new().unwrap();
        let cache = SqliteCache::open(tmp.path()).unwrap();

        // Create 5 tasks
        for i in 1..=5 {
            let task = create_task(
                &format!("Task {}", i),
                i,
                TaskStatus::Todo,
                TaskPriority::Normal,
            );
            cache.index_task(&task).unwrap();
        }

        // Get with limit
        let ready = cache.get_ready_tasks(Some(3)).unwrap();
        assert_eq!(ready.len(), 3);
    }

    // =========================================================================
    // Embedding Tests
    // =========================================================================

    #[test]
    fn test_embedding_to_bytes_roundtrip() {
        let original = vec![1.0f32, 2.5, -3.7, 0.0, 4.2];
        let bytes = embedding_to_bytes(&original);
        let recovered = bytes_to_embedding(&bytes);
        assert_eq!(original.len(), recovered.len());
        for (a, b) in original.iter().zip(recovered.iter()) {
            assert!((a - b).abs() < f32::EPSILON);
        }
    }

    #[test]
    fn test_store_and_get_embedding() {
        let tmp = TempDir::new().unwrap();
        let cache = SqliteCache::open(tmp.path()).unwrap();

        let embedding = vec![0.1f32, 0.2, 0.3, 0.4, 0.5];
        let entity_id = "test-entity-123";
        let entity_type = "decision";
        let text_hash = "abc123";

        // Store embedding
        cache
            .store_embedding(entity_id, entity_type, &embedding, text_hash)
            .unwrap();

        // Retrieve embedding
        let retrieved = cache.get_embedding(entity_id).unwrap();
        assert!(retrieved.is_some());

        let retrieved = retrieved.unwrap();
        assert_eq!(embedding.len(), retrieved.len());
        for (a, b) in embedding.iter().zip(retrieved.iter()) {
            assert!((a - b).abs() < f32::EPSILON);
        }
    }

    #[test]
    fn test_get_embedding_nonexistent() {
        let tmp = TempDir::new().unwrap();
        let cache = SqliteCache::open(tmp.path()).unwrap();

        let retrieved = cache.get_embedding("nonexistent").unwrap();
        assert!(retrieved.is_none());
    }

    #[test]
    fn test_delete_embedding() {
        let tmp = TempDir::new().unwrap();
        let cache = SqliteCache::open(tmp.path()).unwrap();

        let embedding = vec![0.1f32, 0.2, 0.3];
        cache
            .store_embedding("test-id", "decision", &embedding, "hash")
            .unwrap();

        // Verify it exists
        assert!(cache.get_embedding("test-id").unwrap().is_some());

        // Delete it
        cache.delete_embedding("test-id").unwrap();

        // Verify it's gone
        assert!(cache.get_embedding("test-id").unwrap().is_none());
    }

    #[test]
    fn test_list_embeddings_by_type() {
        let tmp = TempDir::new().unwrap();
        let cache = SqliteCache::open(tmp.path()).unwrap();

        // Store embeddings for different types
        cache
            .store_embedding("d1", "decision", &[1.0, 2.0], "h1")
            .unwrap();
        cache
            .store_embedding("d2", "decision", &[3.0, 4.0], "h2")
            .unwrap();
        cache
            .store_embedding("t1", "task", &[5.0, 6.0], "h3")
            .unwrap();

        // List only decisions
        let decisions = cache.list_embeddings_by_type("decision").unwrap();
        assert_eq!(decisions.len(), 2);

        // List only tasks
        let tasks = cache.list_embeddings_by_type("task").unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].0, "t1");
    }

    #[test]
    fn test_list_all_embeddings() {
        let tmp = TempDir::new().unwrap();
        let cache = SqliteCache::open(tmp.path()).unwrap();

        cache
            .store_embedding("d1", "decision", &[1.0], "h1")
            .unwrap();
        cache.store_embedding("t1", "task", &[2.0], "h2").unwrap();

        // List all
        let all = cache.list_all_embeddings(None).unwrap();
        assert_eq!(all.len(), 2);

        // List filtered
        let decisions = cache.list_all_embeddings(Some("decision")).unwrap();
        assert_eq!(decisions.len(), 1);
    }

    #[test]
    fn test_count_embeddings() {
        let tmp = TempDir::new().unwrap();
        let cache = SqliteCache::open(tmp.path()).unwrap();

        assert_eq!(cache.count_embeddings().unwrap(), 0);

        cache
            .store_embedding("e1", "decision", &[1.0], "h1")
            .unwrap();
        cache.store_embedding("e2", "task", &[2.0], "h2").unwrap();

        assert_eq!(cache.count_embeddings().unwrap(), 2);
    }

    #[test]
    fn test_embedding_text_hash() {
        let tmp = TempDir::new().unwrap();
        let cache = SqliteCache::open(tmp.path()).unwrap();

        cache
            .store_embedding("e1", "decision", &[1.0], "original_hash")
            .unwrap();

        let hash = cache.get_embedding_text_hash("e1").unwrap();
        assert_eq!(hash, Some("original_hash".to_string()));

        // Update with different hash
        cache
            .store_embedding("e1", "decision", &[2.0], "new_hash")
            .unwrap();

        let hash = cache.get_embedding_text_hash("e1").unwrap();
        assert_eq!(hash, Some("new_hash".to_string()));
    }

    #[test]
    fn test_compute_text_hash_deterministic() {
        let text = "Hello, world!";
        let hash1 = compute_text_hash(text);
        let hash2 = compute_text_hash(text);
        assert_eq!(hash1, hash2);

        let different_text = "Different text";
        let hash3 = compute_text_hash(different_text);
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_clear_also_clears_embeddings() {
        let tmp = TempDir::new().unwrap();
        let cache = SqliteCache::open(tmp.path()).unwrap();

        cache
            .store_embedding("e1", "decision", &[1.0], "h1")
            .unwrap();
        assert_eq!(cache.count_embeddings().unwrap(), 1);

        cache.clear().unwrap();
        assert_eq!(cache.count_embeddings().unwrap(), 0);
    }

    #[test]
    fn test_embeddable_text_title_only() {
        let text = embeddable_text("My Title", None, &[]);
        assert_eq!(text, "My Title");
    }

    #[test]
    fn test_embeddable_text_with_content() {
        let text = embeddable_text("Title", Some("Content here"), &[]);
        assert_eq!(text, "Title\n\nContent here");
    }

    #[test]
    fn test_embeddable_text_with_tags() {
        let text = embeddable_text("Title", None, &["tag1".to_string(), "tag2".to_string()]);
        assert_eq!(text, "Title\n\ntag1 tag2");
    }

    #[test]
    fn test_embeddable_text_full() {
        let text = embeddable_text(
            "Title",
            Some("Content"),
            &["rust".to_string(), "database".to_string()],
        );
        assert_eq!(text, "Title\n\nContent\n\nrust database");
    }

    // =========================================================================
    // Semantic Search Tests
    // =========================================================================

    #[test]
    fn test_cosine_similarity_identical() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        let sim = cosine_similarity(&a, &b);
        assert!((sim - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        let sim = cosine_similarity(&a, &b);
        assert!(sim.abs() < f32::EPSILON);
    }

    #[test]
    fn test_cosine_similarity_opposite() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![-1.0, 0.0, 0.0];
        let sim = cosine_similarity(&a, &b);
        assert!((sim - (-1.0)).abs() < f32::EPSILON);
    }

    #[test]
    fn test_cosine_similarity_empty() {
        let a: Vec<f32> = vec![];
        let b: Vec<f32> = vec![];
        let sim = cosine_similarity(&a, &b);
        assert_eq!(sim, 0.0);
    }

    #[test]
    fn test_search_semantic_basic() {
        let tmp = TempDir::new().unwrap();
        let cache = SqliteCache::open(tmp.path()).unwrap();

        // Create a decision for metadata lookup
        let decision = Decision::new("Test Decision".to_string(), 1);
        cache.index_decision(&decision).unwrap();

        // Store embedding with known vector
        let entity_id = decision.base.id.to_string();
        let embedding = vec![1.0, 0.0, 0.0]; // 3-dimensional for simplicity
        cache
            .store_embedding(&entity_id, "decision", &embedding, "hash1")
            .unwrap();

        // Search with identical vector
        let query_embedding = vec![1.0, 0.0, 0.0];
        let results = cache
            .search_semantic(&query_embedding, None, 10, 0.0)
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].entity_id, entity_id);
        assert!((results[0].score - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_search_semantic_threshold() {
        let tmp = TempDir::new().unwrap();
        let cache = SqliteCache::open(tmp.path()).unwrap();

        // Create decisions
        let d1 = Decision::new("Decision One".to_string(), 1);
        let d2 = Decision::new("Decision Two".to_string(), 2);
        cache.index_decision(&d1).unwrap();
        cache.index_decision(&d2).unwrap();

        // Store embeddings
        cache
            .store_embedding(&d1.base.id.to_string(), "decision", &[1.0, 0.0, 0.0], "h1")
            .unwrap();
        cache
            .store_embedding(&d2.base.id.to_string(), "decision", &[0.0, 1.0, 0.0], "h2")
            .unwrap();

        // Search with vector similar to d1
        let query = vec![0.9, 0.1, 0.0];
        let results = cache.search_semantic(&query, None, 10, 0.8).unwrap();

        // Only d1 should be above 0.8 threshold
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].entity_id, d1.base.id.to_string());
    }

    #[test]
    fn test_search_semantic_type_filter() {
        let tmp = TempDir::new().unwrap();
        let cache = SqliteCache::open(tmp.path()).unwrap();

        // Create a decision and a task
        let decision = Decision::new("Test Decision".to_string(), 1);
        let task = Task::new("Test Task".to_string(), 2);
        cache.index_decision(&decision).unwrap();
        cache.index_task(&task).unwrap();

        // Store similar embeddings for both
        let embedding = vec![1.0, 0.0, 0.0];
        cache
            .store_embedding(&decision.base.id.to_string(), "decision", &embedding, "h1")
            .unwrap();
        cache
            .store_embedding(&task.base.id.to_string(), "task", &embedding, "h2")
            .unwrap();

        // Search only decisions
        let results = cache
            .search_semantic(&embedding, Some("decision"), 10, 0.0)
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].entity_type, "decision");
    }

    #[test]
    fn test_search_semantic_ordering() {
        let tmp = TempDir::new().unwrap();
        let cache = SqliteCache::open(tmp.path()).unwrap();

        // Create multiple decisions
        let d1 = Decision::new("Decision One".to_string(), 1);
        let d2 = Decision::new("Decision Two".to_string(), 2);
        let d3 = Decision::new("Decision Three".to_string(), 3);
        cache.index_decision(&d1).unwrap();
        cache.index_decision(&d2).unwrap();
        cache.index_decision(&d3).unwrap();

        // Store embeddings with decreasing similarity to query
        cache
            .store_embedding(&d1.base.id.to_string(), "decision", &[0.9, 0.1, 0.0], "h1")
            .unwrap();
        cache
            .store_embedding(&d2.base.id.to_string(), "decision", &[0.7, 0.3, 0.0], "h2")
            .unwrap();
        cache
            .store_embedding(&d3.base.id.to_string(), "decision", &[0.5, 0.5, 0.0], "h3")
            .unwrap();

        // Search
        let query = vec![1.0, 0.0, 0.0];
        let results = cache.search_semantic(&query, None, 10, 0.0).unwrap();

        // Should be ordered by score descending
        assert_eq!(results.len(), 3);
        assert!(results[0].score >= results[1].score);
        assert!(results[1].score >= results[2].score);
    }

    #[test]
    fn test_search_semantic_limit() {
        let tmp = TempDir::new().unwrap();
        let cache = SqliteCache::open(tmp.path()).unwrap();

        // Create many decisions
        for i in 1..=10 {
            let d = Decision::new(format!("Decision {}", i), i);
            cache.index_decision(&d).unwrap();
            cache
                .store_embedding(
                    &d.base.id.to_string(),
                    "decision",
                    &[1.0, 0.0, 0.0],
                    &format!("h{}", i),
                )
                .unwrap();
        }

        // Search with limit
        let query = vec![1.0, 0.0, 0.0];
        let results = cache.search_semantic(&query, None, 3, 0.0).unwrap();

        assert_eq!(results.len(), 3);
    }
}
