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

use nautilus_model::data::{Bar, QuoteTick, TradeTick};

use crate::indicator::Indicator;

/// Volume Profile indicator that tracks volume distribution across price levels.
///
/// This indicator aggregates volume at each price level and calculates:
/// - POC (Point of Control): Price level with the highest volume
/// - VAH (Value Area High): Upper bound of the value area (default 70% of volume)
/// - VAL (Value Area Low): Lower bound of the value area
/// - HVN (High Volume Nodes): Price levels with significantly high volume
/// - LVN (Low Volume Nodes): Price levels with significantly low volume
#[repr(C)]
#[derive(Debug)]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "nautilus_trader.core.nautilus_pyo3.indicators")
)]
pub struct VolumeProfile {
    pub tick_size: f64,
    pub value_area_pct: f64,
    pub hvn_threshold: f64,
    pub lvn_threshold: f64,
    pub reset_hour_utc: i32,
    pub total_volume: f64,
    pub initialized: bool,
    has_inputs: bool,
    volume_at_price: HashMap<i64, f64>,
    last_reset_day: i32,
    poc_price: f64,
    poc_volume: f64,
    cached_vah: f64,
    cached_val: f64,
    cached_hvn_levels: Vec<f64>,
    cached_lvn_levels: Vec<f64>,
    value_area_dirty: bool,
    volume_nodes_dirty: bool,
}

impl Display for VolumeProfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}({},{},{},{},{})",
            self.name(),
            self.tick_size,
            self.value_area_pct,
            self.hvn_threshold,
            self.lvn_threshold,
            self.reset_hour_utc
        )
    }
}

impl Indicator for VolumeProfile {
    fn name(&self) -> String {
        stringify!(VolumeProfile).to_string()
    }

    fn has_inputs(&self) -> bool {
        self.has_inputs
    }

    fn initialized(&self) -> bool {
        self.initialized
    }

    fn handle_quote(&mut self, _quote: &QuoteTick) {
        // Not applicable for volume profile
    }

    fn handle_trade(&mut self, trade: &TradeTick) {
        self.check_reset(trade.ts_event.as_u64());
        let price = trade.price.as_f64();
        let volume = trade.size.as_f64();
        self.update_volume(price, volume);
    }

    fn handle_bar(&mut self, bar: &Bar) {
        self.check_reset(bar.ts_event.as_u64());
        let typical_price =
            (bar.high.as_f64() + bar.low.as_f64() + bar.close.as_f64()) / 3.0;
        let volume = bar.volume.as_f64();
        self.update_volume(typical_price, volume);
    }

    fn reset(&mut self) {
        self.reset_profile();
        self.last_reset_day = -1;
        self.has_inputs = false;
        self.initialized = false;
    }
}

impl VolumeProfile {
    /// Creates a new [`VolumeProfile`] instance.
    ///
    /// # Panics
    ///
    /// Panics if `tick_size` is not positive.
    /// Panics if `value_area_pct` is not in range 0.0-1.0.
    #[must_use]
    pub fn new(
        tick_size: f64,
        value_area_pct: f64,
        hvn_threshold: f64,
        lvn_threshold: f64,
        reset_hour_utc: i32,
    ) -> Self {
        assert!(
            tick_size > 0.0,
            "VolumeProfile::new → `tick_size` must be positive (> 0.0); got {tick_size}"
        );
        assert!(
            (0.0..=1.0).contains(&value_area_pct),
            "VolumeProfile::new → `value_area_pct` must be in range 0.0-1.0; got {value_area_pct}"
        );

        Self {
            tick_size,
            value_area_pct,
            hvn_threshold,
            lvn_threshold,
            reset_hour_utc,
            total_volume: 0.0,
            initialized: false,
            has_inputs: false,
            volume_at_price: HashMap::new(),
            last_reset_day: -1,
            poc_price: 0.0,
            poc_volume: 0.0,
            cached_vah: 0.0,
            cached_val: 0.0,
            cached_hvn_levels: Vec::new(),
            cached_lvn_levels: Vec::new(),
            value_area_dirty: true,
            volume_nodes_dirty: true,
        }
    }

    fn round_to_tick(&self, price: f64) -> i64 {
        (price / self.tick_size).round() as i64
    }

    fn tick_to_price(&self, tick: i64) -> f64 {
        tick as f64 * self.tick_size
    }

    fn check_reset(&mut self, ts_event: u64) {
        if self.reset_hour_utc < 0 {
            return;
        }

        let secs = ts_event / 1_000_000_000;
        let current_day = (secs / 86400) as i32;
        let current_hour = ((secs % 86400) / 3600) as i32;

        if current_day != self.last_reset_day && current_hour >= self.reset_hour_utc {
            self.reset_profile();
            self.last_reset_day = current_day;
        }
    }

    fn reset_profile(&mut self) {
        self.volume_at_price.clear();
        self.total_volume = 0.0;
        self.poc_price = 0.0;
        self.poc_volume = 0.0;
        self.cached_vah = 0.0;
        self.cached_val = 0.0;
        self.cached_hvn_levels.clear();
        self.cached_lvn_levels.clear();
        self.value_area_dirty = true;
        self.volume_nodes_dirty = true;
    }

    fn update_volume(&mut self, price: f64, volume: f64) {
        let tick = self.round_to_tick(price);
        let price_rounded = self.tick_to_price(tick);

        *self.volume_at_price.entry(tick).or_insert(0.0) += volume;
        self.total_volume += volume;

        // Incremental POC update (O(1) instead of O(n))
        let vol_at_tick = self.volume_at_price[&tick];
        if vol_at_tick > self.poc_volume {
            self.poc_price = price_rounded;
            self.poc_volume = vol_at_tick;
        }

        // Mark cached values as dirty (lazy evaluation)
        self.value_area_dirty = true;
        self.volume_nodes_dirty = true;

        if !self.has_inputs {
            self.has_inputs = true;
        }
        if !self.initialized {
            self.initialized = true;
        }
    }

    #[must_use]
    pub fn poc(&self) -> f64 {
        self.poc_price
    }

    #[must_use]
    pub fn vah(&mut self) -> f64 {
        if self.value_area_dirty {
            self.calculate_value_area();
        }
        self.cached_vah
    }

    #[must_use]
    pub fn val(&mut self) -> f64 {
        if self.value_area_dirty {
            self.calculate_value_area();
        }
        self.cached_val
    }

    #[must_use]
    pub fn hvn_levels(&mut self) -> &[f64] {
        if self.volume_nodes_dirty {
            self.calculate_volume_nodes();
        }
        &self.cached_hvn_levels
    }

    #[must_use]
    pub fn lvn_levels(&mut self) -> &[f64] {
        if self.volume_nodes_dirty {
            self.calculate_volume_nodes();
        }
        &self.cached_lvn_levels
    }

    fn calculate_value_area(&mut self) {
        if self.total_volume == 0.0 {
            self.cached_vah = 0.0;
            self.cached_val = 0.0;
            self.value_area_dirty = false;
            return;
        }

        let target_volume = self.total_volume * self.value_area_pct;
        let mut sorted_ticks: Vec<i64> = self.volume_at_price.keys().copied().collect();
        sorted_ticks.sort_unstable();

        if sorted_ticks.is_empty() {
            self.cached_vah = 0.0;
            self.cached_val = 0.0;
            self.value_area_dirty = false;
            return;
        }

        // Find POC tick
        let poc_tick = self.round_to_tick(self.poc_price);
        let poc_idx = sorted_ticks.iter().position(|&t| t == poc_tick).unwrap_or(0);

        let mut accumulated_volume = self.volume_at_price[&sorted_ticks[poc_idx]];
        let mut low_idx = poc_idx;
        let mut high_idx = poc_idx;

        while accumulated_volume < target_volume {
            let vol_above = if high_idx + 1 < sorted_ticks.len() {
                self.volume_at_price.get(&sorted_ticks[high_idx + 1]).copied().unwrap_or(0.0)
            } else {
                0.0
            };

            let vol_below = if low_idx > 0 {
                self.volume_at_price.get(&sorted_ticks[low_idx - 1]).copied().unwrap_or(0.0)
            } else {
                0.0
            };

            if vol_above == 0.0 && vol_below == 0.0 {
                break;
            }

            if vol_above >= vol_below && high_idx + 1 < sorted_ticks.len() {
                high_idx += 1;
                accumulated_volume += vol_above;
            } else if low_idx > 0 {
                low_idx -= 1;
                accumulated_volume += vol_below;
            } else if high_idx + 1 < sorted_ticks.len() {
                high_idx += 1;
                accumulated_volume += vol_above;
            } else {
                break;
            }
        }

        self.cached_val = self.tick_to_price(sorted_ticks[low_idx]);
        self.cached_vah = self.tick_to_price(sorted_ticks[high_idx]);
        self.value_area_dirty = false;
    }

    fn calculate_volume_nodes(&mut self) {
        if self.volume_at_price.is_empty() {
            self.cached_hvn_levels.clear();
            self.cached_lvn_levels.clear();
            self.volume_nodes_dirty = false;
            return;
        }

        let avg_volume = self.total_volume / self.volume_at_price.len() as f64;

        self.cached_hvn_levels = self
            .volume_at_price
            .iter()
            .filter(|&(_, volume)| *volume >= avg_volume * self.hvn_threshold)
            .map(|(&tick, _)| self.tick_to_price(tick))
            .collect();

        self.cached_lvn_levels = self
            .volume_at_price
            .iter()
            .filter(|&(_, volume)| *volume <= avg_volume * self.lvn_threshold)
            .map(|(&tick, _)| self.tick_to_price(tick))
            .collect();

        self.volume_nodes_dirty = false;
    }

    #[must_use]
    pub fn get_volume_at_price(&self, price: f64) -> f64 {
        let tick = self.round_to_tick(price);
        self.volume_at_price.get(&tick).copied().unwrap_or(0.0)
    }
}

