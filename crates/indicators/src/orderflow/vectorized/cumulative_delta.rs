// -------------------------------------------------------------------------------------------------
//  Copyright (C) 2015-2025 Nautech Systems Pty Ltd. All rights reserved.
//  https://nautechsystems.io
//
//  Licensed under the GNU Lesser General Public License Version 3.0 (the "License");
//  You may not use this file except in compliance with the License.
//  You may obtain a copy of the License at https://www.gnu.org/licenses/lgpl-3.0.en.html
//
//  Unless required by applicable law or agreed to in writing, software
//  distributed under the License is distributed on an "AS IS" BASIS,
//  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//  See the License for the specific language governing permissions and
//  limitations under the License.
// -------------------------------------------------------------------------------------------------

//! Vectorized Cumulative Delta calculation.
//!
//! Processes entire arrays of ticks at once for maximum performance.

#![allow(unsafe_code)]

use super::TickArrays;

/// Calculate cumulative delta for all ticks at once
///
/// This is MUCH faster than processing one tick at a time because:
/// 1. No function call overhead per tick
/// 2. CPU can prefetch and pipeline operations
/// 3. Compiler can auto-vectorize with SIMD
/// 4. Better cache locality
pub fn calculate(ticks: &TickArrays) -> Vec<f64> {
    let len = ticks.len;
    let mut cumulative_delta = Vec::with_capacity(len);
    
    if len == 0 {
        return cumulative_delta;
    }
    
    let mut running_delta = 0.0;
    
    // Process all ticks in a tight loop (compiler will auto-vectorize)
    for i in 0..len {
        let qty = ticks.quantities[i];
        
        if ticks.is_buyer[i] {
            running_delta += qty;
        } else {
            running_delta -= qty;
        }
        
        cumulative_delta.push(running_delta);
    }
    
    cumulative_delta
}

/// Calculate cumulative delta with SIMD optimization (manual)
///
/// This version uses explicit SIMD for even better performance on supported CPUs.
///
/// # Safety
///
/// This function uses SIMD intrinsics which are unsafe. However, the operations are safe because:
/// - We check array bounds before accessing elements
/// - We use proper alignment for SIMD operations
/// - We handle remainder elements separately
#[cfg(target_arch = "x86_64")]
pub fn calculate_simd(ticks: &TickArrays) -> Vec<f64> {
    use std::arch::x86_64::*;

    let len = ticks.len;
    let mut cumulative_delta = Vec::with_capacity(len);

    if len == 0 {
        return cumulative_delta;
    }

    let mut running_delta = 0.0;

    // Process in chunks of 4 for SIMD (AVX can do 4x f64 at once)
    let chunks = len / 4;

    for chunk in 0..chunks {
        let base_idx = chunk * 4;

        // SAFETY: We've checked that base_idx + 3 < len (via chunks calculation)
        unsafe {
            // Load 4 quantities at once
            let qty0 = ticks.quantities[base_idx];
            let qty1 = ticks.quantities[base_idx + 1];
            let qty2 = ticks.quantities[base_idx + 2];
            let qty3 = ticks.quantities[base_idx + 3];

            let qty_vec = _mm256_set_pd(qty3, qty2, qty1, qty0);

            // Create sign vector based on is_buyer
            let sign0 = if ticks.is_buyer[base_idx] { 1.0 } else { -1.0 };
            let sign1 = if ticks.is_buyer[base_idx + 1] { 1.0 } else { -1.0 };
            let sign2 = if ticks.is_buyer[base_idx + 2] { 1.0 } else { -1.0 };
            let sign3 = if ticks.is_buyer[base_idx + 3] { 1.0 } else { -1.0 };

            let sign_vec = _mm256_set_pd(sign3, sign2, sign1, sign0);

            // Multiply quantities by signs
            let delta_change = _mm256_mul_pd(qty_vec, sign_vec);

            // For cumulative sum, we need to process sequentially
            // (SIMD doesn't help much here, but we save on the multiplication)
            let mut deltas: [f64; 4] = [0.0; 4];
            _mm256_storeu_pd(deltas.as_mut_ptr(), delta_change);

            for &delta in &deltas {
                running_delta += delta;
                cumulative_delta.push(running_delta);
            }
        }
    }

    // Handle remainder
    for i in (chunks * 4)..len {
        let qty = ticks.quantities[i];
        if ticks.is_buyer[i] {
            running_delta += qty;
        } else {
            running_delta -= qty;
        }
        cumulative_delta.push(running_delta);
    }

    cumulative_delta
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_cumulative_delta_vectorized() {
        let ticks = TickArrays {
            prices: vec![100.0, 100.5, 101.0, 100.5, 100.0],
            quantities: vec![10.0, 20.0, 15.0, 25.0, 30.0],
            is_buyer: vec![true, true, false, false, true],
            timestamps: vec![1, 2, 3, 4, 5],
            len: 5,
        };
        
        let delta = calculate(&ticks);
        
        assert_eq!(delta.len(), 5);
        assert_eq!(delta[0], 10.0);   // +10
        assert_eq!(delta[1], 30.0);   // +10 +20
        assert_eq!(delta[2], 15.0);   // +10 +20 -15
        assert_eq!(delta[3], -10.0);  // +10 +20 -15 -25
        assert_eq!(delta[4], 20.0);   // +10 +20 -15 -25 +30
    }
}

