//! Performance threshold warnings for Medulla.
//!
//! This module provides warning generation and formatting for when
//! Medulla's data exceeds recommended thresholds.

use crate::cache::{CacheStats, ENTITY_WARNING_THRESHOLD, LORO_SIZE_WARNING_THRESHOLD};

/// A warning about potential performance issues.
#[derive(Debug, Clone)]
pub enum Warning {
    /// Entity count exceeds recommended threshold.
    HighEntityCount { count: usize, threshold: usize },
    /// loro.db file size exceeds recommended threshold.
    LargeLoroDb { size_mb: f64, threshold_mb: f64 },
}

/// Check thresholds and return any warnings.
///
/// # Arguments
/// * `stats` - Cache statistics from `SqliteCache::get_stats()`
/// * `loro_size` - Size of loro.db file in bytes
///
/// # Returns
/// A vector of warnings (empty if all thresholds are OK)
pub fn check_thresholds(stats: &CacheStats, loro_size: u64) -> Vec<Warning> {
    let mut warnings = Vec::new();

    if stats.entity_count > ENTITY_WARNING_THRESHOLD {
        warnings.push(Warning::HighEntityCount {
            count: stats.entity_count,
            threshold: ENTITY_WARNING_THRESHOLD,
        });
    }

    let size_mb = loro_size as f64 / (1024.0 * 1024.0);
    let threshold_mb = LORO_SIZE_WARNING_THRESHOLD as f64 / (1024.0 * 1024.0);
    if loro_size > LORO_SIZE_WARNING_THRESHOLD {
        warnings.push(Warning::LargeLoroDb {
            size_mb,
            threshold_mb,
        });
    }

    warnings
}

/// Format a warning for display.
pub fn format_warning(warning: &Warning) -> String {
    match warning {
        Warning::HighEntityCount { count, threshold } => {
            format!(
                "Warning: {} entities exceeds recommended {} - search may slow down",
                count, threshold
            )
        }
        Warning::LargeLoroDb {
            size_mb,
            threshold_mb,
        } => {
            format!(
                "Warning: loro.db size ({:.1}MB) exceeds recommended {:.0}MB",
                size_mb, threshold_mb
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_stats(entity_count: usize) -> CacheStats {
        CacheStats {
            entity_count,
            embedding_count: 0,
            decisions: 0,
            tasks: 0,
            notes: 0,
            prompts: 0,
            components: 0,
            links: 0,
            relations: 0,
        }
    }

    #[test]
    fn test_no_warnings_under_threshold() {
        let stats = mock_stats(500);
        let warnings = check_thresholds(&stats, 5 * 1024 * 1024);
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_high_entity_count_warning() {
        let stats = mock_stats(1500);
        let warnings = check_thresholds(&stats, 0);
        assert_eq!(warnings.len(), 1);
        match &warnings[0] {
            Warning::HighEntityCount { count, threshold } => {
                assert_eq!(*count, 1500);
                assert_eq!(*threshold, ENTITY_WARNING_THRESHOLD);
            }
            _ => panic!("Expected HighEntityCount warning"),
        }
    }

    #[test]
    fn test_large_loro_db_warning() {
        let stats = mock_stats(100);
        let warnings = check_thresholds(&stats, 15 * 1024 * 1024);
        assert_eq!(warnings.len(), 1);
        match &warnings[0] {
            Warning::LargeLoroDb { size_mb, .. } => {
                assert!(*size_mb > 14.0 && *size_mb < 16.0);
            }
            _ => panic!("Expected LargeLoroDb warning"),
        }
    }

    #[test]
    fn test_multiple_warnings() {
        let stats = mock_stats(2000);
        let warnings = check_thresholds(&stats, 20 * 1024 * 1024);
        assert_eq!(warnings.len(), 2);
    }

    #[test]
    fn test_format_high_entity_count() {
        let warning = Warning::HighEntityCount {
            count: 1500,
            threshold: 1000,
        };
        let msg = format_warning(&warning);
        assert!(msg.contains("1500"));
        assert!(msg.contains("1000"));
    }

    #[test]
    fn test_format_large_loro_db() {
        let warning = Warning::LargeLoroDb {
            size_mb: 15.5,
            threshold_mb: 10.0,
        };
        let msg = format_warning(&warning);
        assert!(msg.contains("15.5"));
        assert!(msg.contains("10"));
    }
}
