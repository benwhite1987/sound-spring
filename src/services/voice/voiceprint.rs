//! On-disk enrolled voiceprint: a single speaker-embedding vector with a small
//! header. Format (little-endian) per the Phase 2 spec:
//!
//! ```text
//! offset  size  field
//! 0       4     magic  = b"SSPV"
//! 4       2     version (u16)
//! 6       2     dim     (u16, embedding length)
//! 8       8     timestamp (i64, unix seconds, enrollment time)
//! 16      4*dim f32 embedding values
//! ```
//!
//! The stored vector is L2-normalized so the verifier's cosine comparison is a
//! plain dot product.

use anyhow::{bail, Context, Result};
use std::fs;
use std::path::Path;

const MAGIC: &[u8; 4] = b"SSPV";
const VERSION: u16 = 1;
const HEADER_LEN: usize = 16;

#[derive(Debug, Clone)]
pub struct Voiceprint {
    pub version: u16,
    /// Unix seconds at enrollment time.
    pub timestamp: i64,
    /// L2-normalized embedding.
    pub vector: Vec<f32>,
}

impl Voiceprint {
    /// Build from a raw (not-necessarily-normalized) embedding, stamping the
    /// current time and L2-normalizing.
    pub fn from_embedding(embedding: &[f32]) -> Self {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        Self {
            version: VERSION,
            timestamp,
            vector: l2_normalize(embedding),
        }
    }

    pub fn dim(&self) -> usize {
        self.vector.len()
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("create voiceprint dir {}", parent.display()))?;
        }
        let mut bytes = Vec::with_capacity(HEADER_LEN + self.vector.len() * 4);
        bytes.extend_from_slice(MAGIC);
        bytes.extend_from_slice(&self.version.to_le_bytes());
        bytes.extend_from_slice(&(self.vector.len() as u16).to_le_bytes());
        bytes.extend_from_slice(&self.timestamp.to_le_bytes());
        for value in &self.vector {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        fs::write(path, bytes).with_context(|| format!("write voiceprint {}", path.display()))
    }

    pub fn load(path: &Path) -> Result<Self> {
        let bytes =
            fs::read(path).with_context(|| format!("read voiceprint {}", path.display()))?;
        Self::from_bytes(&bytes)
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < HEADER_LEN {
            bail!("voiceprint too small: {} bytes", bytes.len());
        }
        if &bytes[0..4] != MAGIC {
            bail!("bad voiceprint magic");
        }
        let version = u16::from_le_bytes([bytes[4], bytes[5]]);
        let dim = u16::from_le_bytes([bytes[6], bytes[7]]) as usize;
        let timestamp = i64::from_le_bytes(bytes[8..16].try_into().unwrap());
        let expected = HEADER_LEN + dim * 4;
        if bytes.len() < expected {
            bail!(
                "voiceprint truncated: have {} bytes, need {} for dim {}",
                bytes.len(),
                expected,
                dim
            );
        }
        let mut vector = Vec::with_capacity(dim);
        for chunk in bytes[HEADER_LEN..expected].chunks_exact(4) {
            vector.push(f32::from_le_bytes(chunk.try_into().unwrap()));
        }
        Ok(Self {
            version,
            timestamp,
            vector,
        })
    }
}

/// L2-normalize a vector; a zero vector is returned unchanged.
pub fn l2_normalize(v: &[f32]) -> Vec<f32> {
    let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm <= f32::EPSILON {
        return v.to_vec();
    }
    v.iter().map(|x| x / norm).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrips_through_bytes() {
        let vp = Voiceprint::from_embedding(&[3.0, 4.0, 0.0]);
        // 3-4-0 normalizes to magnitude 1.
        let norm = vp.vector.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-5, "stored vector should be unit norm");

        let dir = std::env::temp_dir().join(format!("sspv-test-{}.bin", std::process::id()));
        vp.save(&dir).unwrap();
        let loaded = Voiceprint::load(&dir).unwrap();
        std::fs::remove_file(&dir).ok();

        assert_eq!(loaded.version, VERSION);
        assert_eq!(loaded.dim(), 3);
        assert_eq!(loaded.timestamp, vp.timestamp);
        for (a, b) in loaded.vector.iter().zip(vp.vector.iter()) {
            assert!((a - b).abs() < 1e-6);
        }
    }

    #[test]
    fn rejects_bad_magic() {
        let mut bytes = vec![0u8; HEADER_LEN + 4];
        bytes[0..4].copy_from_slice(b"XXXX");
        assert!(Voiceprint::from_bytes(&bytes).is_err());
    }
}
