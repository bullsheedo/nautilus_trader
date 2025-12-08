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

//! Vectorized Initial Balance calculation.

#![allow(unsafe_code)]

use super::TickArrays;

#[derive(Debug, Clone)]
pub struct InitialBalanceResult {
    pub ib_high: Vec<f64>,
    pub ib_low: Vec<f64>,
    pub ib_mid: Vec<f64>,
}

/// Calculate initial balance (first N minutes of session)
pub fn calculate(ticks: &TickArrays, period_minutes: u64) -> InitialBalanceResult {
    let len = ticks.len;
    let mut ib_high = Vec::with_capacity(len);
    let mut ib_low = Vec::with_capacity(len);
    let mut ib_mid = Vec::with_capacity(len);
    
    if len == 0 {
        return InitialBalanceResult {
            ib_high,
            ib_low,
            ib_mid,
        };
    }
    
    let period_nanos = period_minutes * 60 * 1_000_000_000;
    
    let mut current_session_start = 0;
    let mut current_ib_high = ticks.prices[0];
    let mut current_ib_low = ticks.prices[0];
    let mut ib_complete = false;
    
    for i in 0..len {
        let timestamp = ticks.timestamps[i];
        let price = ticks.prices[i];
        
        // Check if we've started a new session (simplified: 24-hour sessions)
        // In production, you'd use actual session times
        let session_start = (timestamp / (24 * 60 * 60 * 1_000_000_000)) * (24 * 60 * 60 * 1_000_000_000);
        
        if session_start != current_session_start {
            // New session
            current_session_start = session_start;
            current_ib_high = price;
            current_ib_low = price;
            ib_complete = false;
        }
        
        // Update IB range if still in IB period
        if !ib_complete && (timestamp - current_session_start) < period_nanos {
            current_ib_high = current_ib_high.max(price);
            current_ib_low = current_ib_low.min(price);
        } else if !ib_complete {
            ib_complete = true;
        }
        
        let mid = (current_ib_high + current_ib_low) / 2.0;
        
        ib_high.push(current_ib_high);
        ib_low.push(current_ib_low);
        ib_mid.push(mid);
    }
    
    InitialBalanceResult {
        ib_high,
        ib_low,
        ib_mid,
    }
}

/// Calculate IB with rolling window (simpler version)
pub fn calculate_rolling(ticks: &TickArrays, window_size: usize) -> InitialBalanceResult {
    let len = ticks.len;
    let mut ib_high = Vec::with_capacity(len);
    let mut ib_low = Vec::with_capacity(len);
    let mut ib_mid = Vec::with_capacity(len);
    
    if len == 0 {
        return InitialBalanceResult {
            ib_high,
            ib_low,
            ib_mid,
        };
    }
    
    for i in 0..len {
        let start_idx = if i >= window_size { i - window_size + 1 } else { 0 };
        
        let mut high = ticks.prices[start_idx];
        let mut low = ticks.prices[start_idx];
        
        for j in start_idx..=i {
            high = high.max(ticks.prices[j]);
            low = low.min(ticks.prices[j]);
        }
        
        let mid = (high + low) / 2.0;
        
        ib_high.push(high);
        ib_low.push(low);
        ib_mid.push(mid);
    }
    
    InitialBalanceResult {
        ib_high,
        ib_low,
        ib_mid,
    }
}

/// Fast IB using SIMD for min/max operations
///
/// # Safety
///
/// Uses SIMD intrinsics for finding min/max values in arrays.
/// Safe because:
/// - Array bounds are checked before SIMD operations
/// - Proper alignment is maintained
/// - Remainder elements are handled separately
#[cfg(target_arch = "x86_64")]
pub fn calculate_simd(ticks: &TickArrays, window_size: usize) -> InitialBalanceResult {
    use std::arch::x86_64::*;

    let len = ticks.len;
    let mut ib_high = Vec::with_capacity(len);
    let mut ib_low = Vec::with_capacity(len);
    let mut ib_mid = Vec::with_capacity(len);

    if len == 0 {
        return InitialBalanceResult {
            ib_high,
            ib_low,
            ib_mid,
        };
    }

    for i in 0..len {
        let start_idx = if i >= window_size { i - window_size + 1 } else { 0 };
        let window_len = i - start_idx + 1;

        let mut high = ticks.prices[start_idx];
        let mut low = ticks.prices[start_idx];

        // Process in chunks of 4 using AVX
        let chunks = window_len / 4;

        if chunks > 0 {
            // SAFETY: We've verified that start_idx + chunks * 4 <= i + 1 <= len
            unsafe {
                let mut max_vec = _mm256_set1_pd(f64::NEG_INFINITY);
                let mut min_vec = _mm256_set1_pd(f64::INFINITY);

                for chunk in 0..chunks {
                    let idx = start_idx + chunk * 4;
                    let prices = _mm256_loadu_pd(&ticks.prices[idx]);
                    max_vec = _mm256_max_pd(max_vec, prices);
                    min_vec = _mm256_min_pd(min_vec, prices);
                }

                // Extract max/min from vectors
                let mut max_arr: [f64; 4] = [0.0; 4];
                let mut min_arr: [f64; 4] = [0.0; 4];
                _mm256_storeu_pd(max_arr.as_mut_ptr(), max_vec);
                _mm256_storeu_pd(min_arr.as_mut_ptr(), min_vec);

                high = max_arr.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
                low = min_arr.iter().fold(f64::INFINITY, |a, &b| a.min(b));
            }
        }

        // Handle remainder
        for j in (start_idx + chunks * 4)..=i {
            high = high.max(ticks.prices[j]);
            low = low.min(ticks.prices[j]);
        }

        let mid = (high + low) / 2.0;

        ib_high.push(high);
        ib_low.push(low);
        ib_mid.push(mid);
    }

    InitialBalanceResult {
        ib_high,
        ib_low,
        ib_mid,
    }
}

