//! Pure-Rust Vector Math, Serialization, and Similarity Metrics.
//!
//! Enforces zero-panic arithmetic with saturating/checked operations.
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::unimplemented)]
#![deny(clippy::todo)]

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum VectorMetric {
    Cosine,
    DotProduct,
    L2,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VectorMatch {
    pub id: i64,
    pub score: f32,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FtsMatch {
    pub id: i64,
    pub rank: f64,
    pub snippet: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HybridMatch {
    pub id: i64,
    pub rrf_score: f64,
    pub vector_score: Option<f32>,
    pub fts_rank: Option<f64>,
    pub metadata: serde_json::Value,
}

/// Convert a slice of f32 into packed little-endian bytes.
pub fn pack_vector(v: &[f32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(v.len().saturating_mul(4));
    for &val in v {
        bytes.extend_from_slice(&val.to_le_bytes());
    }
    bytes
}

/// Unpack little-endian bytes back into a Vec<f32>.
pub fn unpack_vector(bytes: &[u8]) -> Vec<f32> {
    let (chunks, _) = bytes.as_chunks::<4>();
    let mut out = Vec::with_capacity(chunks.len());
    for &chunk in chunks {
        out.push(f32::from_le_bytes(chunk));
    }
    out
}

/// Calculate dot product between two float vectors.
pub fn dot_product(a: &[f32], b: &[f32]) -> f32 {
    let len = a.len().min(b.len());
    let mut sum = 0.0f32;
    for i in 0..len {
        let val_a = a.get(i).copied().unwrap_or(0.0);
        let val_b = b.get(i).copied().unwrap_or(0.0);
        sum += val_a * val_b;
    }
    sum
}

/// Calculate Euclidean (L2) distance between two float vectors.
pub fn l2_distance(a: &[f32], b: &[f32]) -> f32 {
    let len = a.len().min(b.len());
    let mut sum_sq = 0.0f32;
    for i in 0..len {
        let val_a = a.get(i).copied().unwrap_or(0.0);
        let val_b = b.get(i).copied().unwrap_or(0.0);
        let diff = val_a - val_b;
        sum_sq += diff * diff;
    }
    sum_sq.sqrt()
}

/// Calculate Cosine similarity between two float vectors (range -1.0 to 1.0).
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let len = a.len().min(b.len());
    if len == 0 {
        return 0.0;
    }
    let mut dot = 0.0f32;
    let mut norm_a = 0.0f32;
    let mut norm_b = 0.0f32;
    for i in 0..len {
        let val_a = a.get(i).copied().unwrap_or(0.0);
        let val_b = b.get(i).copied().unwrap_or(0.0);
        dot += val_a * val_b;
        norm_a += val_a * val_a;
        norm_b += val_b * val_b;
    }
    let denom = (norm_a * norm_b).sqrt();
    if denom <= f32::EPSILON {
        0.0
    } else {
        dot / denom
    }
}

/// Generate a deterministic, unit-normalized semantic embedding vector from text.
///
/// Uses subword character n-gram feature projection (Count-Sketch random projection)
/// to project arbitrary text into a dense vector space of specified dimensionality (e.g. 128 or 384).
/// Completely offline, zero C++ or Python dependencies, and SIMD-friendly.
pub fn generate_text_embedding(text: &str, dimensions: usize) -> Vec<f32> {
    let dim = dimensions.max(1);
    let mut vec = vec![0.0f32; dim];

    let clean = text.trim().to_lowercase();
    if clean.is_empty() {
        return vec;
    }

    fn fnv1a_hash(bytes: &[u8]) -> u64 {
        let mut hash = 0xcbf29ce484222325u64;
        for &b in bytes {
            hash ^= b as u64;
            hash = hash.wrapping_mul(0x100000001b3u64);
        }
        hash
    }

    // 1. Process whole word tokens (weight = 2.0)
    for word in clean.split_whitespace() {
        let h = fnv1a_hash(word.as_bytes());
        let idx = (h % (dim as u64)) as usize;
        let sign = if (h >> 32) & 1 == 0 { 2.0f32 } else { -2.0f32 };
        if let Some(slot) = vec.get_mut(idx) {
            *slot += sign;
        }

        // 2. Character n-grams (3-grams and 4-grams, weight = 1.0)
        let chars = word.chars().collect::<Vec<_>>();
        for n in 3..=4 {
            if chars.len() >= n {
                for window in chars.windows(n) {
                    let s: String = window.iter().collect();
                    let gh = fnv1a_hash(s.as_bytes());
                    let g_idx = (gh % (dim as u64)) as usize;
                    let g_sign = if (gh >> 32) & 1 == 0 { 1.0f32 } else { -1.0f32 };
                    if let Some(slot) = vec.get_mut(g_idx) {
                        *slot += g_sign;
                    }
                }
            }
        }
    }

    // 3. Normalize to unit length (L2 norm = 1.0)
    let mut sum_sq = 0.0f32;
    for &val in &vec {
        sum_sq += val * val;
    }
    let norm = sum_sq.sqrt();
    if norm > f32::EPSILON {
        for val in &mut vec {
            *val /= norm;
        }
    }

    vec
}
