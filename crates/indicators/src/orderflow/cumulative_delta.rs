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

use std::fmt::Display;

use nautilus_model::{
    data::{Bar, QuoteTick, TradeTick},
    enums::AggressorSide,
};

use crate::indicator::Indicator;

/// Cumulative Delta indicator based on trade aggressor side.
///
/// Tracks the running sum of:
/// - Positive delta when buyers aggress (lift the ask)
/// - Negative delta when sellers aggress (hit the bid)
///
/// Provides:
/// - Cumulative delta value
/// - Delta per bar/period
/// - Buy volume and sell volume tracking
#[repr(C)]
#[derive(Debug)]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "nautilus_trader.core.nautilus_pyo3.indicators")
)]
pub struct CumulativeDelta {
    pub reset_hour_utc: i32,
    pub value: f64,
    pub buy_volume: f64,
    pub sell_volume: f64,
    pub last_delta: f64,
    pub last_side: Option<AggressorSide>,
    pub initialized: bool,
    has_inputs: bool,
    last_reset_day: i32,
}

impl Display for CumulativeDelta {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}({})", self.name(), self.reset_hour_utc)
    }
}

impl Indicator for CumulativeDelta {
    fn name(&self) -> String {
        stringify!(CumulativeDelta).to_string()
    }

    fn has_inputs(&self) -> bool {
        self.has_inputs
    }

    fn initialized(&self) -> bool {
        self.initialized
    }

    fn handle_quote(&mut self, _quote: &QuoteTick) {
        // Not applicable for cumulative delta
    }

    fn handle_trade(&mut self, trade: &TradeTick) {
        self.update_trade(trade);
    }

    fn handle_bar(&mut self, _bar: &Bar) {
        // Not applicable for cumulative delta
    }

    fn reset(&mut self) {
        self.reset_delta();
        self.last_reset_day = -1;
        self.has_inputs = false;
        self.initialized = false;
    }
}

impl CumulativeDelta {
    /// Creates a new [`CumulativeDelta`] instance.
    #[must_use]
    pub fn new(reset_hour_utc: i32) -> Self {
        Self {
            reset_hour_utc,
            value: 0.0,
            buy_volume: 0.0,
            sell_volume: 0.0,
            last_delta: 0.0,
            last_side: None,
            initialized: false,
            has_inputs: false,
            last_reset_day: -1,
        }
    }

    fn check_reset(&mut self, ts_event: u64) {
        if self.reset_hour_utc < 0 {
            return;
        }

        // Convert nanoseconds to seconds, then to days
        let secs = ts_event / 1_000_000_000;
        let current_day = (secs / 86400) as i32;
        let current_hour = ((secs % 86400) / 3600) as i32;

        if current_day != self.last_reset_day && current_hour >= self.reset_hour_utc {
            self.reset_delta();
            self.last_reset_day = current_day;
        }
    }

    fn reset_delta(&mut self) {
        self.value = 0.0;
        self.buy_volume = 0.0;
        self.sell_volume = 0.0;
        self.last_delta = 0.0;
        self.last_side = None;
    }

    pub fn update_trade(&mut self, trade: &TradeTick) {
        self.check_reset(trade.ts_event.as_u64());

        let volume = trade.size.as_f64();
        self.last_side = Some(trade.aggressor_side);

        match trade.aggressor_side {
            AggressorSide::Buyer => {
                self.last_delta = volume;
                self.value += volume;
                self.buy_volume += volume;
            }
            AggressorSide::Seller => {
                self.last_delta = -volume;
                self.value -= volume;
                self.sell_volume += volume;
            }
            _ => {
                self.last_delta = 0.0;
            }
        }

        if !self.has_inputs {
            self.has_inputs = true;
        }
        if !self.initialized {
            self.initialized = true;
        }
    }

    /// Return the buy/sell volume ratio.
    /// Returns positive if more buying pressure, negative if more selling pressure.
    #[must_use]
    pub fn delta_ratio(&self) -> f64 {
        let total = self.buy_volume + self.sell_volume;
        if total == 0.0 {
            return 0.0;
        }
        (self.buy_volume - self.sell_volume) / total
    }

    /// Return buy volume / sell volume ratio.
    #[must_use]
    pub fn buy_sell_ratio(&self) -> f64 {
        if self.sell_volume == 0.0 {
            return if self.buy_volume > 0.0 {
                f64::INFINITY
            } else {
                0.0
            };
        }
        self.buy_volume / self.sell_volume
    }
}

