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

use nautilus_model::data::{Bar, QuoteTick, TradeTick};

use crate::indicator::Indicator;

/// Initial Balance indicator for the first hour of NY trading session.
///
/// Calculates the Initial Balance range from the first hour of the NY trading session:
/// - IB High: Highest price during the first hour
/// - IB Low: Lowest price during the first hour
/// - IB Mid: Midpoint of IB range
/// - Extensions: Multiple extensions above IB High and below IB Low
///
/// Handles US daylight saving time:
/// - EST (Winter): NY session starts at 14:30 UTC
/// - EDT (Summer): NY session starts at 13:30 UTC
#[repr(C)]
#[derive(Debug)]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "nautilus_trader.core.nautilus_pyo3.indicators")
)]
pub struct InitialBalance {
    pub num_extensions: usize,
    pub extension_multiplier: f64,
    pub ib_duration_minutes: i32,
    pub ib_high: f64,
    pub ib_low: f64,
    pub ib_mid: f64,
    pub ib_range: f64,
    pub extensions_above: Vec<f64>,
    pub extensions_below: Vec<f64>,
    pub initialized: bool,
    has_inputs: bool,
    current_date: i32,
    ib_forming: bool,
    ib_complete: bool,
    ib_start_time: Option<u64>,
}

impl Display for InitialBalance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}({},{},{})",
            self.name(),
            self.num_extensions,
            self.extension_multiplier,
            self.ib_duration_minutes
        )
    }
}

impl Indicator for InitialBalance {
    fn name(&self) -> String {
        stringify!(InitialBalance).to_string()
    }

    fn has_inputs(&self) -> bool {
        self.has_inputs
    }

    fn initialized(&self) -> bool {
        self.initialized
    }

    fn handle_quote(&mut self, _quote: &QuoteTick) {
        // Not applicable for initial balance
    }

    fn handle_trade(&mut self, trade: &TradeTick) {
        if self.check_ib_window(trade.ts_event.as_u64()) {
            let price = trade.price.as_f64();
            self.update_ib(price, price);
        }

        if !self.has_inputs && self.ib_complete {
            self.has_inputs = true;
        }
        if !self.initialized && self.ib_complete {
            self.initialized = true;
        }
    }

    fn handle_bar(&mut self, bar: &Bar) {
        if self.check_ib_window(bar.ts_event.as_u64()) {
            self.update_ib(bar.high.as_f64(), bar.low.as_f64());
        }

        if !self.has_inputs && self.ib_complete {
            self.has_inputs = true;
        }
        if !self.initialized && self.ib_complete {
            self.initialized = true;
        }
    }

    fn reset(&mut self) {
        self.reset_ib();
        self.current_date = -1;
        self.has_inputs = false;
        self.initialized = false;
    }
}

impl InitialBalance {
    /// Creates a new [`InitialBalance`] instance.
    ///
    /// # Panics
    ///
    /// Panics if `num_extensions` is not positive.
    /// Panics if `extension_multiplier` is not positive.
    /// Panics if `ib_duration_minutes` is not positive.
    #[must_use]
    pub fn new(num_extensions: usize, extension_multiplier: f64, ib_duration_minutes: i32) -> Self {
        assert!(
            num_extensions > 0,
            "InitialBalance::new → `num_extensions` must be positive (> 0); got {num_extensions}"
        );
        assert!(
            extension_multiplier > 0.0,
            "InitialBalance::new → `extension_multiplier` must be positive (> 0.0); got {extension_multiplier}"
        );
        assert!(
            ib_duration_minutes > 0,
            "InitialBalance::new → `ib_duration_minutes` must be positive (> 0); got {ib_duration_minutes}"
        );

        Self {
            num_extensions,
            extension_multiplier,
            ib_duration_minutes,
            ib_high: 0.0,
            ib_low: f64::INFINITY,
            ib_mid: 0.0,
            ib_range: 0.0,
            extensions_above: vec![0.0; num_extensions],
            extensions_below: vec![0.0; num_extensions],
            initialized: false,
            has_inputs: false,
            current_date: -1,
            ib_forming: false,
            ib_complete: false,
            ib_start_time: None,
        }
    }

    fn is_us_dst(&self, ts_event: u64) -> bool {
        // Convert nanoseconds to seconds
        let secs = ts_event / 1_000_000_000;
        let days_since_epoch = secs / 86400;

        // Simplified DST check: March 8-14 to November 1-7
        // This is approximate but good enough for IB calculation
        let day_of_year = (days_since_epoch % 365) as i32;
        day_of_year >= 67 && day_of_year < 305 // Roughly March 8 to Nov 1
    }

    fn get_ib_start_hour_utc(&self, ts_event: u64) -> (i32, i32) {
        if self.is_us_dst(ts_event) {
            (13, 30) // EDT: 9:30 AM EDT = 13:30 UTC
        } else {
            (14, 30) // EST: 9:30 AM EST = 14:30 UTC
        }
    }

    fn check_ib_window(&mut self, ts_event: u64) -> bool {
        let secs = ts_event / 1_000_000_000;
        let current_date = (secs / 86400) as i32;

        // New day - reset IB
        if current_date != self.current_date {
            self.reset_ib();
            self.current_date = current_date;
        }

        if self.ib_complete {
            return false;
        }

        let (start_hour, start_min) = self.get_ib_start_hour_utc(ts_event);
        let current_hour = ((secs % 86400) / 3600) as i32;
        let current_min = ((secs % 3600) / 60) as i32;

        // Calculate IB window
        let ib_start_mins = start_hour * 60 + start_min;
        let ib_end_mins = ib_start_mins + self.ib_duration_minutes;
        let current_mins = current_hour * 60 + current_min;

        if current_mins >= ib_start_mins && current_mins < ib_end_mins {
            if !self.ib_forming {
                self.ib_forming = true;
                self.ib_start_time = Some(ts_event);
            }
            return true;
        } else if current_mins >= ib_end_mins && self.ib_forming {
            self.ib_complete = true;
            self.ib_forming = false;
            return false;
        }

        false
    }

    fn reset_ib(&mut self) {
        self.ib_forming = false;
        self.ib_complete = false;
        self.ib_start_time = None;
        self.ib_high = 0.0;
        self.ib_low = f64::INFINITY;
        self.ib_mid = 0.0;
        self.ib_range = 0.0;
        self.extensions_above = vec![0.0; self.num_extensions];
        self.extensions_below = vec![0.0; self.num_extensions];
    }

    fn update_ib(&mut self, high: f64, low: f64) {
        if high > self.ib_high {
            self.ib_high = high;
        }
        if low < self.ib_low {
            self.ib_low = low;
        }

        self.calculate_extensions();
    }

    fn calculate_extensions(&mut self) {
        if self.ib_high == 0.0 || self.ib_low == f64::INFINITY {
            return;
        }

        self.ib_range = self.ib_high - self.ib_low;
        self.ib_mid = (self.ib_high + self.ib_low) / 2.0;

        let extension_size = self.ib_range * self.extension_multiplier;

        for i in 0..self.num_extensions {
            let multiplier = (i + 1) as f64;
            self.extensions_above[i] = self.ib_high + (extension_size * multiplier);
            self.extensions_below[i] = self.ib_low - (extension_size * multiplier);
        }
    }

    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.ib_complete
    }

    #[must_use]
    pub fn is_forming(&self) -> bool {
        self.ib_forming
    }
}

