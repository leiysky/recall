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

pub fn signature(vec: &[f32], bits: u8, seed: u64) -> u64 {
    let bits = bits.clamp(1, 63);
    let mut sig = 0u64;
    for bit in 0..bits {
        let mut sum = 0.0f32;
        for (i, v) in vec.iter().enumerate() {
            let rand = pseudo_rand(seed, bit as u64, i as u64);
            sum += v * rand;
        }
        if sum >= 0.0 {
            sig |= 1u64 << bit;
        }
    }
    sig
}

pub fn neighbor_signatures(sig: u64, bits: u8) -> Vec<u64> {
    let bits = bits.clamp(1, 63);
    let mut sigs = Vec::with_capacity(bits as usize + 1);
    sigs.push(sig);
    for bit in 0..bits {
        sigs.push(sig ^ (1u64 << bit));
    }
    sigs
}

fn pseudo_rand(seed: u64, bit: u64, idx: u64) -> f32 {
    let mut x = seed ^ (bit.wrapping_mul(0x9E37_79B9_7F4A_7C15)) ^ idx;
    x = splitmix64(x);
    let val = (x as f64) / (u64::MAX as f64);
    (val * 2.0 - 1.0) as f32
}

fn splitmix64(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = x;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}
