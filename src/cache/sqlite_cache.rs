use std::path::{Path, PathBuf};

use rusqlite::{params, Connection, OptionalExtension};

use crate::entity::{Decision, Relation};
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
        self.conn
            .execute("DELETE FROM relations WHERE composite_key = ?1", [composite_key])?;
        Ok(())
    }

    /// Clear all cached data (for full rebuild)
    pub fn clear(&self) -> Result<()> {
        self.conn.execute("DELETE FROM decisions", [])?;
        self.conn.execute("DELETE FROM relations", [])?;
        self.conn.execute("DELETE FROM meta", [])?;
        Ok(())
    }

    /// Full-text search for decisions
    pub fn search_decisions(&self, query: &str) -> Result<Vec<SearchResult>> {
        let mut stmt = self.conn.prepare(
            "SELECT d.id, d.sequence_number, d.title, d.status,
                    highlight(decisions_fts, 1, '<mark>', '</mark>') as title_highlight,
                    snippet(decisions_fts, 2, '<mark>', '</mark>', '...', 32) as content_snippet
             FROM decisions_fts f
             JOIN decisions d ON d.id = f.id
             WHERE decisions_fts MATCH ?1
             ORDER BY rank
             LIMIT 50",
        )?;

        let results = stmt
            .query_map([query], |row| {
                Ok(SearchResult {
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

/// Search result from full-text search
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub id: String,
    pub sequence_number: u32,
    pub title: String,
    pub status: String,
    pub title_highlight: Option<String>,
    pub content_snippet: Option<String>,
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
        assert_eq!(cache.get_loro_version().unwrap(), Some("abc123".to_string()));

        // Update version
        cache.set_loro_version("def456").unwrap();
        assert_eq!(cache.get_loro_version().unwrap(), Some("def456".to_string()));
    }

    #[test]
    fn test_index_and_search_decision() {
        let tmp = TempDir::new().unwrap();
        let cache = SqliteCache::open(tmp.path()).unwrap();

        let decision = crate::entity::Decision::new("Use PostgreSQL for database".to_string(), 1);
        cache.index_decision(&decision).unwrap();

        // Search for it
        let results = cache.search_decisions("PostgreSQL").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Use PostgreSQL for database");

        // Search for non-matching term
        let results = cache.search_decisions("MySQL").unwrap();
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
        let results = cache.search_decisions("Decision").unwrap();
        assert_eq!(results.len(), 2);

        // Same version should skip
        let reindexed = cache.sync_from_loro(&decisions, &relations, "v1").unwrap();
        assert!(!reindexed);

        // New version should reindex
        let reindexed = cache.sync_from_loro(&decisions, &relations, "v2").unwrap();
        assert!(reindexed);
    }
}
