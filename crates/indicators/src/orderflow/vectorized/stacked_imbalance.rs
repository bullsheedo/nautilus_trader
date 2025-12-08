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

//! Vectorized Stacked Imbalance detection.

use super::TickArrays;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct StackedImbalanceResult {
    pub has_bullish_signal: Vec<bool>,
    pub has_bearish_signal: Vec<bool>,
    pub imbalance_strength: Vec<f64>,
}

/// Detect stacked imbalances (multiple consecutive price levels with buy/sell imbalance)
pub fn calculate(
    ticks: &TickArrays,
    tick_size: f64,
    min_stack_size: usize,
    imbalance_ratio: f64,
) -> StackedImbalanceResult {
    let len = ticks.len;
    let mut has_bullish_signal = Vec::with_capacity(len);
    let mut has_bearish_signal = Vec::with_capacity(len);
    let mut imbalance_strength = Vec::with_capacity(len);
    
    if len == 0 {
        return StackedImbalanceResult {
            has_bullish_signal,
            has_bearish_signal,
            imbalance_strength,
        };
    }
    
    // Track volume at each price level
    let mut price_buy_volume: HashMap<i64, f64> = HashMap::new();
    let mut price_sell_volume: HashMap<i64, f64> = HashMap::new();
    
    for i in 0..len {
        let price = ticks.prices[i];
        let qty = ticks.quantities[i];
        let price_level = (price / tick_size).round() as i64;
        
        if ticks.is_buyer[i] {
            *price_buy_volume.entry(price_level).or_insert(0.0) += qty;
        } else {
            *price_sell_volume.entry(price_level).or_insert(0.0) += qty;
        }
        
        // Check for stacked imbalances
        let (bullish, bearish, strength) = detect_stack(
            &price_buy_volume,
            &price_sell_volume,
            price_level,
            min_stack_size,
            imbalance_ratio,
        );
        
        has_bullish_signal.push(bullish);
        has_bearish_signal.push(bearish);
        imbalance_strength.push(strength);
    }
    
    StackedImbalanceResult {
        has_bullish_signal,
        has_bearish_signal,
        imbalance_strength,
    }
}

/// Detect if there's a stack of imbalanced levels
fn detect_stack(
    buy_volumes: &HashMap<i64, f64>,
    sell_volumes: &HashMap<i64, f64>,
    current_level: i64,
    min_stack_size: usize,
    imbalance_ratio: f64,
) -> (bool, bool, f64) {
    // Check for bullish stack (buy > sell for consecutive levels below current)
    let mut bullish_stack = 0;
    let mut bullish_strength = 0.0;
    
    for offset in 1..=min_stack_size {
        let level = current_level - offset as i64;
        let buy_vol = buy_volumes.get(&level).copied().unwrap_or(0.0);
        let sell_vol = sell_volumes.get(&level).copied().unwrap_or(0.0);
        
        if buy_vol > sell_vol * imbalance_ratio {
            bullish_stack += 1;
            bullish_strength += buy_vol - sell_vol;
        } else {
            break;
        }
    }
    
    // Check for bearish stack (sell > buy for consecutive levels above current)
    let mut bearish_stack = 0;
    let mut bearish_strength = 0.0;
    
    for offset in 1..=min_stack_size {
        let level = current_level + offset as i64;
        let buy_vol = buy_volumes.get(&level).copied().unwrap_or(0.0);
        let sell_vol = sell_volumes.get(&level).copied().unwrap_or(0.0);
        
        if sell_vol > buy_vol * imbalance_ratio {
            bearish_stack += 1;
            bearish_strength += sell_vol - buy_vol;
        } else {
            break;
        }
    }
    
    let has_bullish = bullish_stack >= min_stack_size;
    let has_bearish = bearish_stack >= min_stack_size;
    let strength = if has_bullish {
        bullish_strength
    } else if has_bearish {
        -bearish_strength
    } else {
        0.0
    };
    
    (has_bullish, has_bearish, strength)
}

/// Fast version using rolling window instead of full history
pub fn calculate_rolling(
    ticks: &TickArrays,
    tick_size: f64,
    window_size: usize,
    min_stack_size: usize,
    imbalance_ratio: f64,
) -> StackedImbalanceResult {
    let len = ticks.len;
    let mut has_bullish_signal = Vec::with_capacity(len);
    let mut has_bearish_signal = Vec::with_capacity(len);
    let mut imbalance_strength = Vec::with_capacity(len);
    
    if len == 0 {
        return StackedImbalanceResult {
            has_bullish_signal,
            has_bearish_signal,
            imbalance_strength,
        };
    }
    
    for i in 0..len {
        let start_idx = if i >= window_size { i - window_size + 1 } else { 0 };
        
        let mut price_buy_volume: HashMap<i64, f64> = HashMap::new();
        let mut price_sell_volume: HashMap<i64, f64> = HashMap::new();
        
        for j in start_idx..=i {
            let price = ticks.prices[j];
            let qty = ticks.quantities[j];
            let price_level = (price / tick_size).round() as i64;
            
            if ticks.is_buyer[j] {
                *price_buy_volume.entry(price_level).or_insert(0.0) += qty;
            } else {
                *price_sell_volume.entry(price_level).or_insert(0.0) += qty;
            }
        }
        
        let current_level = (ticks.prices[i] / tick_size).round() as i64;
        let (bullish, bearish, strength) = detect_stack(
            &price_buy_volume,
            &price_sell_volume,
            current_level,
            min_stack_size,
            imbalance_ratio,
        );
        
        has_bullish_signal.push(bullish);
        has_bearish_signal.push(bearish);
        imbalance_strength.push(strength);
    }
    
    StackedImbalanceResult {
        has_bullish_signal,
        has_bearish_signal,
        imbalance_strength,
    }
}

