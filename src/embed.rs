// Copyright 2026 Recall Authors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use sha2::Digest;
use sha2::Sha256;

pub trait Embedder {
    fn embed(&self, text: &str) -> Vec<f32>;
}

#[derive(Clone)]
pub struct HashEmbedder {
    dim: usize,
}

impl HashEmbedder {
    pub fn new(dim: usize) -> Self {
        Self { dim: dim.max(1) }
    }
}

impl Embedder for HashEmbedder {
    fn embed(&self, text: &str) -> Vec<f32> {
        let mut vec = vec![0.0f32; self.dim];
        for token in text.split_whitespace() {
            let mut hasher = Sha256::new();
            hasher.update(token.as_bytes());
            let hash = hasher.finalize();
            let mut bytes = [0u8; 8];
            bytes.copy_from_slice(&hash[..8]);
            let val = u64::from_le_bytes(bytes);
            let idx = (val as usize) % self.dim;
            let sign = if (val & (1 << 63)) != 0 { 1.0 } else { -1.0 };
            vec[idx] += sign;
        }
        l2_normalize(vec)
    }
}

fn l2_normalize(mut vec: Vec<f32>) -> Vec<f32> {
    let mut norm = 0.0f32;
    for v in &vec {
        norm += v * v;
    }
    if norm > 0.0 {
        let inv = 1.0 / norm.sqrt();
        for v in &mut vec {
            *v *= inv;
        }
    }
    vec
}

pub fn to_bytes(vec: &[f32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(vec.len() * 4);
    for v in vec {
        out.extend_from_slice(&v.to_le_bytes());
    }
    out
}

pub fn from_bytes(bytes: &[u8]) -> Vec<f32> {
    let mut vec = Vec::new();
    let mut i = 0;
    while i + 4 <= bytes.len() {
        let mut arr = [0u8; 4];
        arr.copy_from_slice(&bytes[i..i + 4]);
        vec.push(f32::from_le_bytes(arr));
        i += 4;
    }
    vec
}

pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.is_empty() || b.is_empty() || a.len() != b.len() {
        return 0.0;
    }
    let mut dot = 0.0f32;
    let mut norm_a = 0.0f32;
    let mut norm_b = 0.0f32;
    for i in 0..a.len() {
        dot += a[i] * b[i];
        norm_a += a[i] * a[i];
        norm_b += b[i] * b[i];
    }
    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot / (norm_a.sqrt() * norm_b.sqrt())
    }
}
