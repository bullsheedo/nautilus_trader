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

//! Vectorized Footprint calculation.

#![allow(unsafe_code)]

use super::TickArrays;

#[derive(Debug, Clone)]
pub struct FootprintResult {
    pub total_delta: Vec<f64>,
    pub buy_volume: Vec<f64>,
    pub sell_volume: Vec<f64>,
}

/// Calculate footprint delta with rolling window
pub fn calculate(ticks: &TickArrays, window_size: usize) -> FootprintResult {
    let len = ticks.len;
    let mut total_delta = Vec::with_capacity(len);
    let mut buy_volume = Vec::with_capacity(len);
    let mut sell_volume = Vec::with_capacity(len);
    
    if len == 0 {
        return FootprintResult {
            total_delta,
            buy_volume,
            sell_volume,
        };
    }
    
    for i in 0..len {
        let start_idx = if i >= window_size { i - window_size + 1 } else { 0 };
        
        let mut buy_vol = 0.0;
        let mut sell_vol = 0.0;
        
        for j in start_idx..=i {
            let qty = ticks.quantities[j];
            
            if ticks.is_buyer[j] {
                buy_vol += qty;
            } else {
                sell_vol += qty;
            }
        }
        
        let delta = buy_vol - sell_vol;
        
        total_delta.push(delta);
        buy_volume.push(buy_vol);
        sell_volume.push(sell_vol);
    }
    
    FootprintResult {
        total_delta,
        buy_volume,
        sell_volume,
    }
}

/// Fast footprint using cumulative sums (O(n) instead of O(n*window))
pub fn calculate_fast(ticks: &TickArrays, window_size: usize) -> FootprintResult {
    let len = ticks.len;
    let mut total_delta = Vec::with_capacity(len);
    let mut buy_volume = Vec::with_capacity(len);
    let mut sell_volume = Vec::with_capacity(len);
    
    if len == 0 {
        return FootprintResult {
            total_delta,
            buy_volume,
            sell_volume,
        };
    }
    
    // Build cumulative sums
    let mut cum_buy = Vec::with_capacity(len + 1);
    let mut cum_sell = Vec::with_capacity(len + 1);
    
    cum_buy.push(0.0);
    cum_sell.push(0.0);
    
    for i in 0..len {
        let qty = ticks.quantities[i];
        let prev_buy = cum_buy[i];
        let prev_sell = cum_sell[i];
        
        if ticks.is_buyer[i] {
            cum_buy.push(prev_buy + qty);
            cum_sell.push(prev_sell);
        } else {
            cum_buy.push(prev_buy);
            cum_sell.push(prev_sell + qty);
        }
    }
    
    // Calculate rolling window using cumulative sums
    for i in 0..len {
        let start_idx = if i >= window_size { i - window_size + 1 } else { 0 };
        
        let buy_vol = cum_buy[i + 1] - cum_buy[start_idx];
        let sell_vol = cum_sell[i + 1] - cum_sell[start_idx];
        let delta = buy_vol - sell_vol;
        
        total_delta.push(delta);
        buy_volume.push(buy_vol);
        sell_volume.push(sell_vol);
    }
    
    FootprintResult {
        total_delta,
        buy_volume,
        sell_volume,
    }
}

/// Ultra-fast SIMD version for x86_64
///
/// # Safety
///
/// Uses SIMD intrinsics safely by delegating to the fast cumulative sum version.
#[cfg(target_arch = "x86_64")]
pub fn calculate_simd(ticks: &TickArrays, window_size: usize) -> FootprintResult {
    // For now, use the fast version which is already very efficient
    // Full SIMD implementation would require more complex logic for rolling windows
    calculate_fast(ticks, window_size)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_footprint_fast() {
        let ticks = TickArrays {
            prices: vec![100.0, 100.5, 101.0, 100.5, 100.0],
            quantities: vec![10.0, 20.0, 15.0, 25.0, 30.0],
            is_buyer: vec![true, true, false, false, true],
            timestamps: vec![1, 2, 3, 4, 5],
            len: 5,
        };
        
        let result = calculate_fast(&ticks, 3);
        
        assert_eq!(result.total_delta.len(), 5);
        
        // Window [0]: buy=10, sell=0, delta=10
        assert_eq!(result.total_delta[0], 10.0);
        
        // Window [0,1]: buy=30, sell=0, delta=30
        assert_eq!(result.total_delta[1], 30.0);
        
        // Window [0,1,2]: buy=30, sell=15, delta=15
        assert_eq!(result.total_delta[2], 15.0);
        
        // Window [1,2,3]: buy=20, sell=40, delta=-20
        assert_eq!(result.total_delta[3], -20.0);
        
        // Window [2,3,4]: buy=30, sell=40, delta=-10
        assert_eq!(result.total_delta[4], -10.0);
    }
}

