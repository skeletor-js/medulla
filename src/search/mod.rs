//! Search module for filter parsing and structured queries.

use chrono::{DateTime, Utc};

/// Parsed search filter from query string.
///
/// Filters can be specified in the query string using prefixes:
/// - `type:decision` - Filter by entity type
/// - `status:accepted` - Filter by status
/// - `tag:important` - Filter by tag (can specify multiple)
/// - `created:>2025-01-01` - Created after date
/// - `created:<2025-12-31` - Created before date
#[derive(Debug, Default, Clone)]
pub struct SearchFilter {
    /// Entity type filter (decision, task, note, etc.)
    pub entity_type: Option<String>,
    /// Status filter (for entities that have status)
    pub status: Option<String>,
    /// Tag filters (entity must have all specified tags)
    pub tags: Vec<String>,
    /// Created after this date/time
    pub created_after: Option<DateTime<Utc>>,
    /// Created before this date/time
    pub created_before: Option<DateTime<Utc>>,
}

impl SearchFilter {
    /// Create an empty filter.
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if filter has any constraints.
    pub fn is_empty(&self) -> bool {
        self.entity_type.is_none()
            && self.status.is_none()
            && self.tags.is_empty()
            && self.created_after.is_none()
            && self.created_before.is_none()
    }
}

/// Parse a raw query string into (remaining query text, filters).
///
/// # Examples
///
/// ```ignore
/// let (query, filter) = parse_query("type:decision status:accepted postgres database");
/// assert_eq!(query, "postgres database");
/// assert_eq!(filter.entity_type, Some("decision".to_string()));
/// assert_eq!(filter.status, Some("accepted".to_string()));
/// ```
pub fn parse_query(raw: &str) -> (String, SearchFilter) {
    let mut filter = SearchFilter::default();
    let mut remaining = Vec::new();

    for token in raw.split_whitespace() {
        if let Some(value) = token.strip_prefix("type:") {
            filter.entity_type = Some(value.to_string());
        } else if let Some(value) = token.strip_prefix("status:") {
            filter.status = Some(value.to_string());
        } else if let Some(value) = token.strip_prefix("tag:") {
            filter.tags.push(value.to_string());
        } else if let Some(value) = token.strip_prefix("created:>") {
            filter.created_after = parse_date(value);
        } else if let Some(value) = token.strip_prefix("created:<") {
            filter.created_before = parse_date(value);
        } else {
            remaining.push(token);
        }
    }

    (remaining.join(" "), filter)
}

/// Parse a date string into DateTime<Utc>.
/// Supports ISO 8601 date format (YYYY-MM-DD) or full datetime.
fn parse_date(s: &str) -> Option<DateTime<Utc>> {
    // Try full datetime first
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Some(dt.with_timezone(&Utc));
    }

    // Try date only (YYYY-MM-DD) - set to midnight UTC
    if let Ok(date) = chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        let datetime = date.and_hms_opt(0, 0, 0)?;
        return Some(DateTime::from_naive_utc_and_offset(datetime, Utc));
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Datelike;

    #[test]
    fn test_parse_query_no_filters() {
        let (query, filter) = parse_query("hello world");
        assert_eq!(query, "hello world");
        assert!(filter.is_empty());
    }

    #[test]
    fn test_parse_query_type_filter() {
        let (query, filter) = parse_query("type:decision postgres");
        assert_eq!(query, "postgres");
        assert_eq!(filter.entity_type, Some("decision".to_string()));
    }

    #[test]
    fn test_parse_query_status_filter() {
        let (query, filter) = parse_query("status:accepted database");
        assert_eq!(query, "database");
        assert_eq!(filter.status, Some("accepted".to_string()));
    }

    #[test]
    fn test_parse_query_multiple_tags() {
        let (query, filter) = parse_query("tag:important tag:urgent search term");
        assert_eq!(query, "search term");
        assert_eq!(filter.tags.len(), 2);
        assert!(filter.tags.contains(&"important".to_string()));
        assert!(filter.tags.contains(&"urgent".to_string()));
    }

    #[test]
    fn test_parse_query_date_filters() {
        let (query, filter) = parse_query("created:>2025-01-01 created:<2025-12-31 test");
        assert_eq!(query, "test");
        assert!(filter.created_after.is_some());
        assert!(filter.created_before.is_some());
    }

    #[test]
    fn test_parse_query_combined() {
        let (query, filter) = parse_query("type:task status:todo tag:backend priority API");
        assert_eq!(query, "priority API");
        assert_eq!(filter.entity_type, Some("task".to_string()));
        assert_eq!(filter.status, Some("todo".to_string()));
        assert_eq!(filter.tags, vec!["backend".to_string()]);
    }

    #[test]
    fn test_parse_query_only_filters() {
        let (query, filter) = parse_query("type:decision status:accepted");
        assert_eq!(query, "");
        assert_eq!(filter.entity_type, Some("decision".to_string()));
        assert_eq!(filter.status, Some("accepted".to_string()));
    }

    #[test]
    fn test_filter_is_empty() {
        let filter = SearchFilter::new();
        assert!(filter.is_empty());

        let mut filter_with_type = SearchFilter::new();
        filter_with_type.entity_type = Some("decision".to_string());
        assert!(!filter_with_type.is_empty());
    }

    #[test]
    fn test_parse_date_iso() {
        let date = parse_date("2025-06-15");
        assert!(date.is_some());
        let dt = date.unwrap();
        assert_eq!(dt.year(), 2025);
        assert_eq!(dt.month(), 6);
        assert_eq!(dt.day(), 15);
    }

    #[test]
    fn test_parse_date_invalid() {
        let date = parse_date("not-a-date");
        assert!(date.is_none());
    }
}
