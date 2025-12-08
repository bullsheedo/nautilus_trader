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

//! Vectorized VWAP Bands calculation.

use super::TickArrays;

#[derive(Debug, Clone)]
pub struct VWAPResult {
    pub vwap: Vec<f64>,
    pub upper_bands: Vec<Vec<f64>>,  // [+1std, +2std, +3std]
    pub lower_bands: Vec<Vec<f64>>,  // [-1std, -2std, -3std]
}

/// Calculate VWAP and bands for all ticks at once
pub fn calculate(ticks: &TickArrays) -> VWAPResult {
    let len = ticks.len;
    let mut vwap = Vec::with_capacity(len);
    let mut upper_1std = Vec::with_capacity(len);
    let mut upper_2std = Vec::with_capacity(len);
    let mut upper_3std = Vec::with_capacity(len);
    let mut lower_1std = Vec::with_capacity(len);
    let mut lower_2std = Vec::with_capacity(len);
    let mut lower_3std = Vec::with_capacity(len);
    
    if len == 0 {
        return VWAPResult {
            vwap,
            upper_bands: vec![upper_1std, upper_2std, upper_3std],
            lower_bands: vec![lower_1std, lower_2std, lower_3std],
        };
    }
    
    let mut sum_pq = 0.0;  // Sum of price * quantity
    let mut sum_q = 0.0;   // Sum of quantity
    let mut sum_pq_sq = 0.0;  // Sum of (price * quantity)^2 for variance
    
    for i in 0..len {
        let price = ticks.prices[i];
        let qty = ticks.quantities[i];
        let pq = price * qty;
        
        sum_pq += pq;
        sum_q += qty;
        sum_pq_sq += pq * pq;
        
        // Calculate VWAP
        let vwap_val = if sum_q > 0.0 {
            sum_pq / sum_q
        } else {
            price
        };
        
        // Calculate standard deviation
        // Var = E[X^2] - E[X]^2
        let variance = if sum_q > 0.0 {
            let mean_pq = sum_pq / sum_q;
            let mean_pq_sq = sum_pq_sq / sum_q;
            (mean_pq_sq - mean_pq * mean_pq).max(0.0)
        } else {
            0.0
        };
        
        let std_dev = variance.sqrt();
        
        vwap.push(vwap_val);
        upper_1std.push(vwap_val + std_dev);
        upper_2std.push(vwap_val + 2.0 * std_dev);
        upper_3std.push(vwap_val + 3.0 * std_dev);
        lower_1std.push(vwap_val - std_dev);
        lower_2std.push(vwap_val - 2.0 * std_dev);
        lower_3std.push(vwap_val - 3.0 * std_dev);
    }
    
    VWAPResult {
        vwap,
        upper_bands: vec![upper_1std, upper_2std, upper_3std],
        lower_bands: vec![lower_1std, lower_2std, lower_3std],
    }
}

/// Calculate VWAP with rolling window (session-based)
pub fn calculate_rolling(ticks: &TickArrays, window_size: usize) -> VWAPResult {
    let len = ticks.len;
    let mut vwap = Vec::with_capacity(len);
    let mut upper_1std = Vec::with_capacity(len);
    let mut upper_2std = Vec::with_capacity(len);
    let mut upper_3std = Vec::with_capacity(len);
    let mut lower_1std = Vec::with_capacity(len);
    let mut lower_2std = Vec::with_capacity(len);
    let mut lower_3std = Vec::with_capacity(len);
    
    if len == 0 {
        return VWAPResult {
            vwap,
            upper_bands: vec![upper_1std, upper_2std, upper_3std],
            lower_bands: vec![lower_1std, lower_2std, lower_3std],
        };
    }
    
    for i in 0..len {
        let start_idx = if i >= window_size { i - window_size + 1 } else { 0 };
        
        let mut sum_pq = 0.0;
        let mut sum_q = 0.0;
        let mut sum_pq_sq = 0.0;
        
        for j in start_idx..=i {
            let price = ticks.prices[j];
            let qty = ticks.quantities[j];
            let pq = price * qty;
            
            sum_pq += pq;
            sum_q += qty;
            sum_pq_sq += pq * pq;
        }
        
        let vwap_val = if sum_q > 0.0 {
            sum_pq / sum_q
        } else {
            ticks.prices[i]
        };
        
        let variance = if sum_q > 0.0 {
            let mean_pq = sum_pq / sum_q;
            let mean_pq_sq = sum_pq_sq / sum_q;
            (mean_pq_sq - mean_pq * mean_pq).max(0.0)
        } else {
            0.0
        };
        
        let std_dev = variance.sqrt();
        
        vwap.push(vwap_val);
        upper_1std.push(vwap_val + std_dev);
        upper_2std.push(vwap_val + 2.0 * std_dev);
        upper_3std.push(vwap_val + 3.0 * std_dev);
        lower_1std.push(vwap_val - std_dev);
        lower_2std.push(vwap_val - 2.0 * std_dev);
        lower_3std.push(vwap_val - 3.0 * std_dev);
    }
    
    VWAPResult {
        vwap,
        upper_bands: vec![upper_1std, upper_2std, upper_3std],
        lower_bands: vec![lower_1std, lower_2std, lower_3std],
    }
}

