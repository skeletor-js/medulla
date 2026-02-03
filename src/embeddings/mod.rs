pub mod config;

use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use std::sync::OnceLock;

use crate::error::{MedullaError, Result};

static MODEL: OnceLock<std::result::Result<TextEmbedding, String>> = OnceLock::new();

/// Wrapper around the embedding model for computing text embeddings.
/// Uses lazy initialization to load the model on first use.
pub struct Embedder {
    model: &'static TextEmbedding,
}

impl Embedder {
    /// Create a new Embedder with lazy model initialization.
    /// The embedding model (~50MB) is downloaded on first use.
    pub fn new() -> Result<Self> {
        let result = MODEL.get_or_init(|| {
            TextEmbedding::try_new(InitOptions::new(EmbeddingModel::AllMiniLML6V2))
                .map_err(|e| e.to_string())
        });
        match result {
            Ok(model) => Ok(Self { model }),
            Err(e) => Err(MedullaError::Embedding(e.clone())),
        }
    }

    /// Embed a single text, returns 384-dimensional vector.
    pub fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let embeddings = self.model.embed(vec![text], None)?;
        embeddings
            .into_iter()
            .next()
            .ok_or_else(|| MedullaError::Embedding("No embedding returned".to_string()))
    }

    /// Embed multiple texts in batch (more efficient for multiple texts).
    pub fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }
        Ok(self.model.embed(texts.to_vec(), None)?)
    }

    /// Get the embedding dimension (384 for all-MiniLM-L6-v2).
    pub fn dimension(&self) -> usize {
        384
    }
}

impl From<fastembed::Error> for MedullaError {
    fn from(e: fastembed::Error) -> Self {
        MedullaError::Embedding(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires downloading embedding model (~50MB)"]
    fn test_embedder_produces_384_dimensional_vectors() {
        let embedder = Embedder::new().unwrap();
        let embedding = embedder.embed("Hello, world!").unwrap();
        assert_eq!(embedding.len(), 384);
    }

    #[test]
    #[ignore = "requires downloading embedding model (~50MB)"]
    fn test_embedder_batch() {
        let embedder = Embedder::new().unwrap();
        let texts = vec![
            "First text".to_string(),
            "Second text".to_string(),
            "Third text".to_string(),
        ];
        let embeddings = embedder.embed_batch(&texts).unwrap();
        assert_eq!(embeddings.len(), 3);
        for embedding in embeddings {
            assert_eq!(embedding.len(), 384);
        }
    }

    #[test]
    #[ignore = "requires downloading embedding model (~50MB)"]
    fn test_embedder_empty_batch() {
        let embedder = Embedder::new().unwrap();
        let texts: Vec<String> = vec![];
        let embeddings = embedder.embed_batch(&texts).unwrap();
        assert!(embeddings.is_empty());
    }

    #[test]
    #[ignore = "requires downloading embedding model (~50MB)"]
    fn test_embedder_dimension() {
        let embedder = Embedder::new().unwrap();
        assert_eq!(embedder.dimension(), 384);
    }

    #[test]
    #[ignore = "requires downloading embedding model (~50MB)"]
    fn test_similar_texts_have_similar_embeddings() {
        let embedder = Embedder::new().unwrap();
        let emb1 = embedder.embed("I love programming in Rust").unwrap();
        let emb2 = embedder.embed("I enjoy coding in Rust").unwrap();
        let emb3 = embedder.embed("The weather is sunny today").unwrap();

        // Compute cosine similarity
        fn cosine_sim(a: &[f32], b: &[f32]) -> f32 {
            let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
            let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
            let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
            dot / (norm_a * norm_b)
        }

        let sim_similar = cosine_sim(&emb1, &emb2);
        let sim_different = cosine_sim(&emb1, &emb3);

        // Similar texts should have higher similarity than different texts
        assert!(
            sim_similar > sim_different,
            "Similar texts should have higher cosine similarity: {} vs {}",
            sim_similar,
            sim_different
        );
    }
}
