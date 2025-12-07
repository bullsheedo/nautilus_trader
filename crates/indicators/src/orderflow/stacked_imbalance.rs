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

/// Type of imbalance detected.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "nautilus_trader.core.nautilus_pyo3.indicators")
)]
pub enum ImbalanceType {
    None = 0,
    AskImbalance = 1,  // More buying (bullish)
    BidImbalance = 2,  // More selling (bearish)
}

/// Represents a detected stacked imbalance.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "nautilus_trader.core.nautilus_pyo3.indicators")
)]
pub struct StackedImbalance {
    pub imbalance_type: ImbalanceType,
    pub start_price: f64,
    pub end_price: f64,
    pub num_levels: usize,
    pub total_delta: f64,
}

/// Detects stacked imbalances across consecutive price levels.
///
/// Detects consecutive price levels with significant imbalance in the same direction.
/// Stacked imbalances indicate strong institutional activity:
/// - Stacked Ask Imbalances: Strong buying pressure (bullish)
/// - Stacked Bid Imbalances: Strong selling pressure (bearish)
#[repr(C)]
#[derive(Debug)]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "nautilus_trader.core.nautilus_pyo3.indicators")
)]
pub struct StackedImbalanceDetector {
    pub tick_size: f64,
    pub imbalance_ratio: f64,
    pub min_stack_count: usize,
    pub min_volume_per_level: f64,
    pub initialized: bool,
    has_inputs: bool,
    bid_volume: HashMap<i64, f64>,
    ask_volume: HashMap<i64, f64>,
    cached_stacked_ask_imbalances: Vec<StackedImbalance>,
    cached_stacked_bid_imbalances: Vec<StackedImbalance>,
    cached_last_signal: ImbalanceType,
    cached_last_stacked_imbalance: Option<StackedImbalance>,
    stacked_imbalances_dirty: bool,
}

impl Display for StackedImbalanceDetector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}({},{},{},{})",
            self.name(),
            self.tick_size,
            self.imbalance_ratio,
            self.min_stack_count,
            self.min_volume_per_level
        )
    }
}

impl Indicator for StackedImbalanceDetector {
    fn name(&self) -> String {
        stringify!(StackedImbalanceDetector).to_string()
    }

    fn has_inputs(&self) -> bool {
        self.has_inputs
    }

    fn initialized(&self) -> bool {
        self.initialized
    }

    fn handle_quote(&mut self, _quote: &QuoteTick) {
        // Not applicable for stacked imbalance
    }

    fn handle_trade(&mut self, trade: &TradeTick) {
        self.update_trade(trade);
    }

    fn handle_bar(&mut self, _bar: &Bar) {
        // Not applicable for stacked imbalance
    }

    fn reset(&mut self) {
        self.clear();
        self.has_inputs = false;
        self.initialized = false;
    }
}

impl StackedImbalanceDetector {
    /// Creates a new [`StackedImbalanceDetector`] instance.
    ///
    /// # Panics
    ///
    /// Panics if `tick_size` is not positive.
    /// Panics if `imbalance_ratio` is not positive.
    /// Panics if `min_stack_count` is not positive.
    #[must_use]
    pub fn new(
        tick_size: f64,
        imbalance_ratio: f64,
        min_stack_count: usize,
        min_volume_per_level: f64,
    ) -> Self {
        assert!(
            tick_size > 0.0,
            "StackedImbalanceDetector::new → `tick_size` must be positive (> 0.0); got {tick_size}"
        );
        assert!(
            imbalance_ratio > 0.0,
            "StackedImbalanceDetector::new → `imbalance_ratio` must be positive (> 0.0); got {imbalance_ratio}"
        );
        assert!(
            min_stack_count > 0,
            "StackedImbalanceDetector::new → `min_stack_count` must be positive (> 0); got {min_stack_count}"
        );

        Self {
            tick_size,
            imbalance_ratio,
            min_stack_count,
            min_volume_per_level,
            initialized: false,
            has_inputs: false,
            bid_volume: HashMap::new(),
            ask_volume: HashMap::new(),
            cached_stacked_ask_imbalances: Vec::new(),
            cached_stacked_bid_imbalances: Vec::new(),
            cached_last_signal: ImbalanceType::None,
            cached_last_stacked_imbalance: None,
            stacked_imbalances_dirty: true,
        }
    }

    fn round_to_tick(&self, price: f64) -> i64 {
        (price / self.tick_size).round() as i64
    }

    fn tick_to_price(&self, tick: i64) -> f64 {
        tick as f64 * self.tick_size
    }

    pub fn update_trade(&mut self, trade: &TradeTick) {
        let tick = self.round_to_tick(trade.price.as_f64());
        let volume = trade.size.as_f64();

        match trade.aggressor_side {
            AggressorSide::Buyer => {
                *self.ask_volume.entry(tick).or_insert(0.0) += volume;
            }
            AggressorSide::Seller => {
                *self.bid_volume.entry(tick).or_insert(0.0) += volume;
            }
            _ => {}
        }

        self.stacked_imbalances_dirty = true;

        if !self.has_inputs {
            self.has_inputs = true;
        }
        if !self.initialized {
            self.initialized = true;
        }
    }

    fn get_imbalance_type(&self, tick: i64) -> ImbalanceType {
        let ask_vol = self.ask_volume.get(&tick).copied().unwrap_or(0.0);
        let bid_vol = self.bid_volume.get(&tick).copied().unwrap_or(0.0);
        let total_vol = ask_vol + bid_vol;

        if total_vol < self.min_volume_per_level {
            return ImbalanceType::None;
        }

        if ask_vol > 0.0 && bid_vol == 0.0 {
            return ImbalanceType::AskImbalance;
        }
        if bid_vol > 0.0 && ask_vol == 0.0 {
            return ImbalanceType::BidImbalance;
        }

        if bid_vol > 0.0 {
            let ratio = ask_vol / bid_vol;
            if ratio >= self.imbalance_ratio {
                return ImbalanceType::AskImbalance;
            }
        }
        if ask_vol > 0.0 {
            let ratio = bid_vol / ask_vol;
            if ratio >= self.imbalance_ratio {
                return ImbalanceType::BidImbalance;
            }
        }

        ImbalanceType::None
    }

    #[must_use]
    pub fn stacked_ask_imbalances(&mut self) -> &[StackedImbalance] {
        if self.stacked_imbalances_dirty {
            self.detect_stacked_imbalances();
        }
        &self.cached_stacked_ask_imbalances
    }

    #[must_use]
    pub fn stacked_bid_imbalances(&mut self) -> &[StackedImbalance] {
        if self.stacked_imbalances_dirty {
            self.detect_stacked_imbalances();
        }
        &self.cached_stacked_bid_imbalances
    }

    #[must_use]
    pub fn last_signal(&mut self) -> ImbalanceType {
        if self.stacked_imbalances_dirty {
            self.detect_stacked_imbalances();
        }
        self.cached_last_signal
    }

    #[must_use]
    pub fn last_stacked_imbalance(&mut self) -> Option<StackedImbalance> {
        if self.stacked_imbalances_dirty {
            self.detect_stacked_imbalances();
        }
        self.cached_last_stacked_imbalance
    }

    fn detect_stacked_imbalances(&mut self) {
        self.cached_stacked_ask_imbalances.clear();
        self.cached_stacked_bid_imbalances.clear();
        self.cached_last_signal = ImbalanceType::None;
        self.cached_last_stacked_imbalance = None;

        // Get all price ticks and sort them
        let mut all_ticks: Vec<i64> = self
            .bid_volume
            .keys()
            .chain(self.ask_volume.keys())
            .copied()
            .collect();
        all_ticks.sort_unstable();
        all_ticks.dedup();

        if all_ticks.len() < self.min_stack_count {
            self.stacked_imbalances_dirty = false;
            return;
        }

        // Scan for consecutive imbalances
        let mut current_type = ImbalanceType::None;
        let mut stack_start_tick = 0i64;
        let mut stack_count = 0usize;
        let mut stack_delta = 0.0f64;

        for (i, &tick) in all_ticks.iter().enumerate() {
            let imb_type = self.get_imbalance_type(tick);
            let ask_vol = self.ask_volume.get(&tick).copied().unwrap_or(0.0);
            let bid_vol = self.bid_volume.get(&tick).copied().unwrap_or(0.0);
            let level_delta = ask_vol - bid_vol;

            // Check if consecutive (within one tick)
            let is_consecutive = if i == 0 {
                true
            } else {
                (tick - all_ticks[i - 1] - 1).abs() < 1
            };

            if imb_type != ImbalanceType::None && imb_type == current_type && is_consecutive {
                stack_count += 1;
                stack_delta += level_delta;
            } else {
                // Check if previous stack qualifies
                if stack_count >= self.min_stack_count {
                    let stacked = StackedImbalance {
                        imbalance_type: current_type,
                        start_price: self.tick_to_price(stack_start_tick),
                        end_price: if i > 0 {
                            self.tick_to_price(all_ticks[i - 1])
                        } else {
                            self.tick_to_price(stack_start_tick)
                        },
                        num_levels: stack_count,
                        total_delta: stack_delta,
                    };

                    match current_type {
                        ImbalanceType::AskImbalance => {
                            self.cached_stacked_ask_imbalances.push(stacked);
                        }
                        ImbalanceType::BidImbalance => {
                            self.cached_stacked_bid_imbalances.push(stacked);
                        }
                        ImbalanceType::None => {}
                    }

                    self.cached_last_signal = current_type;
                    self.cached_last_stacked_imbalance = Some(stacked);
                }

                // Start new stack
                if imb_type != ImbalanceType::None {
                    current_type = imb_type;
                    stack_start_tick = tick;
                    stack_count = 1;
                    stack_delta = level_delta;
                } else {
                    current_type = ImbalanceType::None;
                    stack_count = 0;
                    stack_delta = 0.0;
                }
            }
        }

        // Check final stack
        if stack_count >= self.min_stack_count {
            let stacked = StackedImbalance {
                imbalance_type: current_type,
                start_price: self.tick_to_price(stack_start_tick),
                end_price: self.tick_to_price(*all_ticks.last().unwrap()),
                num_levels: stack_count,
                total_delta: stack_delta,
            };

            match current_type {
                ImbalanceType::AskImbalance => {
                    self.cached_stacked_ask_imbalances.push(stacked);
                }
                ImbalanceType::BidImbalance => {
                    self.cached_stacked_bid_imbalances.push(stacked);
                }
                ImbalanceType::None => {}
            }

            self.cached_last_signal = current_type;
            self.cached_last_stacked_imbalance = Some(stacked);
        }

        self.stacked_imbalances_dirty = false;
    }

    #[must_use]
    pub fn has_bullish_signal(&mut self) -> bool {
        !self.stacked_ask_imbalances().is_empty()
    }

    #[must_use]
    pub fn has_bearish_signal(&mut self) -> bool {
        !self.stacked_bid_imbalances().is_empty()
    }

    pub fn clear(&mut self) {
        self.bid_volume.clear();
        self.ask_volume.clear();
        self.cached_stacked_ask_imbalances.clear();
        self.cached_stacked_bid_imbalances.clear();
        self.cached_last_signal = ImbalanceType::None;
        self.cached_last_stacked_imbalance = None;
        self.stacked_imbalances_dirty = true;
    }
}

