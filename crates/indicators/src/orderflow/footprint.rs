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

use std::{collections::HashMap, fmt::Display};

use nautilus_model::{
    data::{Bar, QuoteTick, TradeTick},
    enums::AggressorSide,
};

use crate::indicator::Indicator;

/// Data at a single price level in the footprint.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "nautilus_trader.core.nautilus_pyo3.indicators")
)]
pub struct FootprintLevel {
    pub bid_volume: f64,
    pub ask_volume: f64,
    pub trade_count: u64,
}

impl FootprintLevel {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn delta(&self) -> f64 {
        self.ask_volume - self.bid_volume
    }

    #[must_use]
    pub fn total_volume(&self) -> f64 {
        self.bid_volume + self.ask_volume
    }

    #[must_use]
    pub fn imbalance_ratio(&self) -> f64 {
        let total = self.total_volume();
        if total == 0.0 {
            return 0.0;
        }
        self.delta() / total
    }
}

/// Aggregates trade ticks into footprint candle data.
///
/// Aggregates trade data into a footprint structure showing:
/// - Bid volume (sellers hitting the bid)
/// - Ask volume (buyers lifting the ask)
/// - Delta at each price level
/// - Imbalance detection at each level
#[repr(C)]
#[derive(Debug)]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "nautilus_trader.core.nautilus_pyo3.indicators")
)]
pub struct FootprintAggregator {
    pub tick_size: f64,
    pub imbalance_threshold: f64,
    pub high: f64,
    pub low: f64,
    pub open: f64,
    pub close: f64,
    pub total_delta: f64,
    pub total_volume: f64,
    pub buy_volume: f64,
    pub sell_volume: f64,
    pub poc_price: f64,
    pub poc_volume: f64,
    pub initialized: bool,
    has_inputs: bool,
    levels: HashMap<i64, FootprintLevel>,
    cached_imbalanced_levels: HashMap<i64, String>,
    imbalanced_levels_dirty: bool,
}

impl Display for FootprintAggregator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}({},{})",
            self.name(),
            self.tick_size,
            self.imbalance_threshold
        )
    }
}

impl Indicator for FootprintAggregator {
    fn name(&self) -> String {
        stringify!(FootprintAggregator).to_string()
    }

    fn has_inputs(&self) -> bool {
        self.has_inputs
    }

    fn initialized(&self) -> bool {
        self.initialized
    }

    fn handle_quote(&mut self, _quote: &QuoteTick) {
        // Not applicable for footprint
    }

    fn handle_trade(&mut self, trade: &TradeTick) {
        self.update_trade(trade);
    }

    fn handle_bar(&mut self, _bar: &Bar) {
        // Not applicable for footprint
    }

    fn reset(&mut self) {
        self.clear_footprint();
        self.has_inputs = false;
        self.initialized = false;
    }
}

impl FootprintAggregator {
    /// Creates a new [`FootprintAggregator`] instance.
    ///
    /// # Panics
    ///
    /// Panics if `tick_size` is not positive.
    /// Panics if `imbalance_threshold` is not positive.
    #[must_use]
    pub fn new(tick_size: f64, imbalance_threshold: f64) -> Self {
        assert!(
            tick_size > 0.0,
            "FootprintAggregator::new → `tick_size` must be positive (> 0.0); got {tick_size}"
        );
        assert!(
            imbalance_threshold > 0.0,
            "FootprintAggregator::new → `imbalance_threshold` must be positive (> 0.0); got {imbalance_threshold}"
        );

        Self {
            tick_size,
            imbalance_threshold,
            high: 0.0,
            low: f64::INFINITY,
            open: 0.0,
            close: 0.0,
            total_delta: 0.0,
            total_volume: 0.0,
            buy_volume: 0.0,
            sell_volume: 0.0,
            poc_price: 0.0,
            poc_volume: 0.0,
            initialized: false,
            has_inputs: false,
            levels: HashMap::new(),
            cached_imbalanced_levels: HashMap::new(),
            imbalanced_levels_dirty: true,
        }
    }

    fn round_to_tick(&self, price: f64) -> i64 {
        (price / self.tick_size).round() as i64
    }

    fn tick_to_price(&self, tick: i64) -> f64 {
        tick as f64 * self.tick_size
    }

    pub fn update_trade(&mut self, trade: &TradeTick) {
        let price = trade.price.as_f64();
        let rounded_tick = self.round_to_tick(price);
        let volume = trade.size.as_f64();

        // Update OHLC
        if self.open == 0.0 {
            self.open = price;
        }
        self.close = price;
        if price > self.high {
            self.high = price;
        }
        if price < self.low {
            self.low = price;
        }

        // Get or create level
        let level = self.levels.entry(rounded_tick).or_insert_with(FootprintLevel::new);
        level.trade_count += 1;

        // Update volume based on aggressor side
        match trade.aggressor_side {
            AggressorSide::Buyer => {
                level.ask_volume += volume;
                self.buy_volume += volume;
                self.total_delta += volume;
            }
            AggressorSide::Seller => {
                level.bid_volume += volume;
                self.sell_volume += volume;
                self.total_delta -= volume;
            }
            _ => {}
        }

        self.total_volume += volume;

        // Update POC (incremental - O(1))
        if level.total_volume() > self.poc_volume {
            self.poc_volume = level.total_volume();
            self.poc_price = self.tick_to_price(rounded_tick);
        }

        // Mark imbalanced levels as dirty (lazy evaluation)
        self.imbalanced_levels_dirty = true;

        if !self.has_inputs {
            self.has_inputs = true;
        }
        if !self.initialized {
            self.initialized = true;
        }
    }

    #[must_use]
    pub fn get_level(&self, price: f64) -> FootprintLevel {
        let rounded_tick = self.round_to_tick(price);
        self.levels.get(&rounded_tick).copied().unwrap_or_default()
    }

    #[must_use]
    pub fn get_imbalanced_levels(&mut self) -> &HashMap<i64, String> {
        if self.imbalanced_levels_dirty {
            self.calculate_imbalanced_levels();
        }
        &self.cached_imbalanced_levels
    }

    fn calculate_imbalanced_levels(&mut self) {
        self.cached_imbalanced_levels.clear();

        for (&tick, level) in &self.levels {
            if level.ask_volume > 0.0 && level.bid_volume > 0.0 {
                let ratio = level.ask_volume / level.bid_volume;
                if ratio >= self.imbalance_threshold {
                    self.cached_imbalanced_levels.insert(tick, "ASK".to_string());
                } else if ratio <= 1.0 / self.imbalance_threshold {
                    self.cached_imbalanced_levels.insert(tick, "BID".to_string());
                }
            } else if level.ask_volume > 0.0 && level.bid_volume == 0.0 {
                self.cached_imbalanced_levels.insert(tick, "ASK".to_string());
            } else if level.bid_volume > 0.0 && level.ask_volume == 0.0 {
                self.cached_imbalanced_levels.insert(tick, "BID".to_string());
            }
        }

        self.imbalanced_levels_dirty = false;
    }

    pub fn clear_footprint(&mut self) {
        self.levels.clear();
        self.high = 0.0;
        self.low = f64::INFINITY;
        self.open = 0.0;
        self.close = 0.0;
        self.total_delta = 0.0;
        self.total_volume = 0.0;
        self.buy_volume = 0.0;
        self.sell_volume = 0.0;
        self.poc_price = 0.0;
        self.poc_volume = 0.0;
        self.cached_imbalanced_levels.clear();
        self.imbalanced_levels_dirty = true;
    }
}

