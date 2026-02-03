use serde::{Deserialize, Serialize};

/// Configuration for the embedding system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    /// Provider for embeddings: "local" (fastembed) or "openai" (future)
    pub provider: String,
    /// Model name: "all-MiniLM-L6-v2" for local
    pub model: String,
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            provider: "local".to_string(),
            model: "all-MiniLM-L6-v2".to_string(),
        }
    }
}

impl EmbeddingConfig {
    /// Create a new config with the default local embedding model.
    pub fn local() -> Self {
        Self::default()
    }

    /// Check if using local embeddings (fastembed).
    pub fn is_local(&self) -> bool {
        self.provider == "local"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = EmbeddingConfig::default();
        assert_eq!(config.provider, "local");
        assert_eq!(config.model, "all-MiniLM-L6-v2");
        assert!(config.is_local());
    }

    #[test]
    fn test_local_config() {
        let config = EmbeddingConfig::local();
        assert!(config.is_local());
    }

    #[test]
    fn test_serialization() {
        let config = EmbeddingConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let parsed: EmbeddingConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.provider, config.provider);
        assert_eq!(parsed.model, config.model);
    }
}
