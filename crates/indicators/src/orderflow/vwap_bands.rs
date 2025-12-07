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

/// VWAP with Standard Deviation Bands.
///
/// This indicator calculates:
/// - VWAP (Volume Weighted Average Price)
/// - Upper bands at 1, 2, 3 standard deviations
/// - Lower bands at 1, 2, 3 standard deviations
/// - Resets at specified UTC hour (default 00:00 UTC for crypto)
#[repr(C)]
#[derive(Debug)]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "nautilus_trader.core.nautilus_pyo3.indicators")
)]
pub struct VWAPBands {
    pub reset_hour_utc: i32,
    pub num_std_bands: usize,
    pub vwap: f64,
    pub std_dev: f64,
    pub upper_bands: Vec<f64>,
    pub lower_bands: Vec<f64>,
    pub initialized: bool,
    has_inputs: bool,
    sum_price_volume: f64,
    sum_volume: f64,
    sum_price_sq_volume: f64,
    last_reset_day: i32,
}

impl Display for VWAPBands {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}({},{})",
            self.name(),
            self.reset_hour_utc,
            self.num_std_bands
        )
    }
}

impl Indicator for VWAPBands {
    fn name(&self) -> String {
        stringify!(VWAPBands).to_string()
    }

    fn has_inputs(&self) -> bool {
        self.has_inputs
    }

    fn initialized(&self) -> bool {
        self.initialized
    }

    fn handle_quote(&mut self, _quote: &QuoteTick) {
        // Not applicable for VWAP
    }

    fn handle_trade(&mut self, trade: &TradeTick) {
        self.check_reset(trade.ts_event.as_u64());
        let price = trade.price.as_f64();
        let volume = trade.size.as_f64();
        self.update_vwap(price, volume);
    }

    fn handle_bar(&mut self, bar: &Bar) {
        self.check_reset(bar.ts_event.as_u64());
        let typical_price =
            (bar.high.as_f64() + bar.low.as_f64() + bar.close.as_f64()) / 3.0;
        let volume = bar.volume.as_f64();
        self.update_vwap(typical_price, volume);
    }

    fn reset(&mut self) {
        self.reset_vwap();
        self.last_reset_day = -1;
        self.has_inputs = false;
        self.initialized = false;
    }
}

impl VWAPBands {
    /// Creates a new [`VWAPBands`] instance.
    ///
    /// # Panics
    ///
    /// Panics if `reset_hour_utc` is not in range 0-23.
    /// Panics if `num_std_bands` is not positive.
    #[must_use]
    pub fn new(reset_hour_utc: i32, num_std_bands: usize) -> Self {
        assert!(
            (0..=23).contains(&reset_hour_utc),
            "VWAPBands::new → `reset_hour_utc` must be in range 0-23; got {reset_hour_utc}"
        );
        assert!(
            num_std_bands > 0,
            "VWAPBands::new → `num_std_bands` must be positive (> 0); got {num_std_bands}"
        );

        Self {
            reset_hour_utc,
            num_std_bands,
            vwap: 0.0,
            std_dev: 0.0,
            upper_bands: vec![0.0; num_std_bands],
            lower_bands: vec![0.0; num_std_bands],
            initialized: false,
            has_inputs: false,
            sum_price_volume: 0.0,
            sum_volume: 0.0,
            sum_price_sq_volume: 0.0,
            last_reset_day: -1,
        }
    }

    fn check_reset(&mut self, ts_event: u64) {
        let secs = ts_event / 1_000_000_000;
        let current_day = (secs / 86400) as i32;
        let current_hour = ((secs % 86400) / 3600) as i32;

        if current_day != self.last_reset_day && current_hour >= self.reset_hour_utc {
            self.reset_vwap();
            self.last_reset_day = current_day;
        }
    }

    fn reset_vwap(&mut self) {
        self.sum_price_volume = 0.0;
        self.sum_volume = 0.0;
        self.sum_price_sq_volume = 0.0;
        self.vwap = 0.0;
        self.std_dev = 0.0;
        self.upper_bands = vec![0.0; self.num_std_bands];
        self.lower_bands = vec![0.0; self.num_std_bands];
    }

    fn update_vwap(&mut self, price: f64, volume: f64) {
        if volume <= 0.0 {
            return;
        }

        self.sum_price_volume += price * volume;
        self.sum_volume += volume;
        self.sum_price_sq_volume += price * price * volume;

        if self.sum_volume > 0.0 {
            self.vwap = self.sum_price_volume / self.sum_volume;

            // Calculate variance: E[X^2] - E[X]^2
            let mean_sq = self.sum_price_sq_volume / self.sum_volume;
            let variance = mean_sq - (self.vwap * self.vwap);

            // Avoid negative variance due to floating point errors
            self.std_dev = variance.max(0.0).sqrt();

            // Calculate bands
            for i in 0..self.num_std_bands {
                let band_multiplier = (i + 1) as f64;
                self.upper_bands[i] = self.vwap + (band_multiplier * self.std_dev);
                self.lower_bands[i] = self.vwap - (band_multiplier * self.std_dev);
            }
        }

        if !self.has_inputs {
            self.has_inputs = true;
        }
        if !self.initialized {
            self.initialized = true;
        }
    }
}

