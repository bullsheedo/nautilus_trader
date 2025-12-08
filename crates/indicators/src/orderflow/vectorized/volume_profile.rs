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

//! Vectorized Volume Profile calculation.

use super::TickArrays;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct VolumeProfileResult {
    pub poc: Vec<f64>,  // Point of Control
    pub vah: Vec<f64>,  // Value Area High
    pub val: Vec<f64>,  // Value Area Low
}

/// Calculate volume profile with rolling window
pub fn calculate(ticks: &TickArrays, tick_size: f64, window_size: usize) -> VolumeProfileResult {
    let len = ticks.len;
    let mut poc = Vec::with_capacity(len);
    let mut vah = Vec::with_capacity(len);
    let mut val = Vec::with_capacity(len);
    
    if len == 0 {
        return VolumeProfileResult { poc, vah, val };
    }
    
    for i in 0..len {
        let start_idx = if i >= window_size { i - window_size + 1 } else { 0 };
        
        // Build volume profile for this window
        let mut price_volumes: HashMap<i64, f64> = HashMap::new();
        let mut total_volume = 0.0;
        
        for j in start_idx..=i {
            let price = ticks.prices[j];
            let qty = ticks.quantities[j];
            
            // Round price to tick size
            let price_level = (price / tick_size).round() as i64;
            
            *price_volumes.entry(price_level).or_insert(0.0) += qty;
            total_volume += qty;
        }
        
        if price_volumes.is_empty() {
            poc.push(ticks.prices[i]);
            vah.push(ticks.prices[i]);
            val.push(ticks.prices[i]);
            continue;
        }
        
        // Find POC (price level with highest volume)
        let (poc_level, _) = price_volumes
            .iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .unwrap();
        
        let poc_price = (*poc_level as f64) * tick_size;
        
        // Calculate Value Area (70% of volume)
        let value_area_volume = total_volume * 0.70;
        let mut sorted_levels: Vec<_> = price_volumes.iter().collect();
        sorted_levels.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap());
        
        let mut accumulated_volume = 0.0;
        let mut min_level = *poc_level;
        let mut max_level = *poc_level;
        
        for (level, volume) in sorted_levels {
            accumulated_volume += volume;
            min_level = min_level.min(*level);
            max_level = max_level.max(*level);
            
            if accumulated_volume >= value_area_volume {
                break;
            }
        }
        
        let vah_price = (max_level as f64) * tick_size;
        let val_price = (min_level as f64) * tick_size;
        
        poc.push(poc_price);
        vah.push(vah_price);
        val.push(val_price);
    }
    
    VolumeProfileResult { poc, vah, val }
}

/// Fast volume profile using pre-allocated arrays (avoids HashMap overhead)
pub fn calculate_fast(ticks: &TickArrays, tick_size: f64, window_size: usize, price_range: (f64, f64)) -> VolumeProfileResult {
    let len = ticks.len;
    let mut poc = Vec::with_capacity(len);
    let mut vah = Vec::with_capacity(len);
    let mut val = Vec::with_capacity(len);
    
    if len == 0 {
        return VolumeProfileResult { poc, vah, val };
    }
    
    // Pre-allocate array for price levels
    let min_price_level = (price_range.0 / tick_size).floor() as i64;
    let max_price_level = (price_range.1 / tick_size).ceil() as i64;
    let num_levels = (max_price_level - min_price_level + 1) as usize;
    
    let mut price_volumes = vec![0.0; num_levels];
    
    for i in 0..len {
        let start_idx = if i >= window_size { i - window_size + 1 } else { 0 };
        
        // Reset volumes
        price_volumes.fill(0.0);
        let mut total_volume = 0.0;
        
        // Build volume profile
        for j in start_idx..=i {
            let price = ticks.prices[j];
            let qty = ticks.quantities[j];
            let price_level = (price / tick_size).round() as i64;
            
            if price_level >= min_price_level && price_level <= max_price_level {
                let idx = (price_level - min_price_level) as usize;
                price_volumes[idx] += qty;
                total_volume += qty;
            }
        }
        
        // Find POC
        let (poc_idx, _) = price_volumes
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .unwrap();
        
        let poc_price = ((poc_idx as i64 + min_price_level) as f64) * tick_size;
        
        // Calculate Value Area (simplified - just use top 70% of levels)
        let value_area_volume = total_volume * 0.70;
        let mut accumulated = 0.0;
        let mut min_idx = poc_idx;
        let mut max_idx = poc_idx;
        
        // Expand from POC until we hit 70% volume
        let mut expand_up = true;
        let mut expand_down = true;
        
        while accumulated < value_area_volume && (expand_up || expand_down) {
            let up_vol = if max_idx + 1 < num_levels { price_volumes[max_idx + 1] } else { 0.0 };
            let down_vol = if min_idx > 0 { price_volumes[min_idx - 1] } else { 0.0 };
            
            if up_vol >= down_vol && expand_up {
                if max_idx + 1 < num_levels {
                    max_idx += 1;
                    accumulated += price_volumes[max_idx];
                } else {
                    expand_up = false;
                }
            } else if expand_down {
                if min_idx > 0 {
                    min_idx -= 1;
                    accumulated += price_volumes[min_idx];
                } else {
                    expand_down = false;
                }
            } else {
                break;
            }
        }
        
        let vah_price = ((max_idx as i64 + min_price_level) as f64) * tick_size;
        let val_price = ((min_idx as i64 + min_price_level) as f64) * tick_size;
        
        poc.push(poc_price);
        vah.push(vah_price);
        val.push(val_price);
    }
    
    VolumeProfileResult { poc, vah, val }
}

