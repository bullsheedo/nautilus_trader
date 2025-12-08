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

//! Main vectorized backtesting engine.

use nautilus_indicators::orderflow::vectorized::{
    TickArrays, IndicatorArrays,
    cumulative_delta, vwap_bands, volume_profile, footprint,
    initial_balance, stacked_imbalance,
};
use super::signals::{SignalGenerator, SignalConfig};
use super::positions::PositionTracker;
use super::statistics::PerformanceStats;

#[derive(Debug, Clone)]
pub struct BacktestConfig {
    // Indicator parameters
    pub vwap_window: usize,
    pub volume_profile_window: usize,
    pub footprint_window: usize,
    pub ib_period_minutes: u64,
    pub imbalance_min_stack: usize,
    pub imbalance_ratio: f64,

    // Strategy parameters
    pub poi_tolerance: f64,
    pub tick_size: f64,
    pub take_profit_ticks: f64,
    pub stop_loss_ticks: f64,
    pub trailing_stop_ticks: f64,
    pub warmup_ticks: usize,  // Number of ticks to wait before trading

    // Price range for volume profile
    pub price_range: (f64, f64),
}

impl Default for BacktestConfig {
    fn default() -> Self {
        Self {
            vwap_window: 1000,
            volume_profile_window: 1000,
            footprint_window: 100,
            ib_period_minutes: 60,
            imbalance_min_stack: 3,
            imbalance_ratio: 1.5,
            poi_tolerance: 3.0,
            tick_size: 0.01,
            take_profit_ticks: 0.30,
            stop_loss_ticks: 0.35,
            trailing_stop_ticks: 0.20,
            warmup_ticks: 1000,  // Wait 1000 ticks before trading
            price_range: (0.0, 10000.0),
        }
    }
}

#[derive(Debug, Clone)]
pub struct BacktestResult {
    pub stats: PerformanceStats,
    pub elapsed_seconds: f64,
    pub ticks_per_second: f64,
}

#[derive(Debug)]
pub struct VectorizedBacktest {
    config: BacktestConfig,
}

impl VectorizedBacktest {
    pub fn new(config: BacktestConfig) -> Self {
        Self { config }
    }
    
    /// Run the backtest on tick data
    pub fn run(&self, ticks: &TickArrays) -> BacktestResult {
        let start_time = std::time::Instant::now();

        // Step 1: Calculate all indicators at once (vectorized)
        let indicators = self.calculate_indicators(ticks);

        // Step 2: Generate all signals at once (vectorized)
        let signal_config = SignalConfig {
            poi_tolerance: self.config.poi_tolerance,
            tick_size: self.config.tick_size,
            take_profit_ticks: self.config.take_profit_ticks,
            stop_loss_ticks: self.config.stop_loss_ticks,
            trailing_stop_ticks: self.config.trailing_stop_ticks,
            warmup_ticks: self.config.warmup_ticks,
        };

        let signal_generator = SignalGenerator::new(signal_config);
        let signals = signal_generator.generate_signals(&ticks.prices, &indicators);

        // Step 3: Track positions and calculate P&L
        let mut position_tracker = PositionTracker::new();
        position_tracker.process_ticks(&ticks.prices, &signals);

        // Step 4: Calculate statistics
        let stats = PerformanceStats::calculate(&position_tracker.positions);

        let elapsed = start_time.elapsed();
        let elapsed_seconds = elapsed.as_secs_f64();
        let ticks_per_second = if elapsed_seconds > 0.0 {
            ticks.len as f64 / elapsed_seconds
        } else {
            0.0
        };

        BacktestResult {
            stats,
            elapsed_seconds,
            ticks_per_second,
        }
    }
    
    /// Calculate all indicators
    fn calculate_indicators(&self, ticks: &TickArrays) -> IndicatorArrays {
        let mut indicators = IndicatorArrays::with_capacity(ticks.len);
        
        // Calculate each indicator
        indicators.cumulative_delta = cumulative_delta::calculate(ticks);
        
        let vwap_result = vwap_bands::calculate_rolling(ticks, self.config.vwap_window);
        indicators.vwap = vwap_result.vwap;
        indicators.vwap_upper_1std = vwap_result.upper_bands[0].clone();
        indicators.vwap_upper_2std = vwap_result.upper_bands[1].clone();
        indicators.vwap_upper_3std = vwap_result.upper_bands[2].clone();
        indicators.vwap_lower_1std = vwap_result.lower_bands[0].clone();
        indicators.vwap_lower_2std = vwap_result.lower_bands[1].clone();
        indicators.vwap_lower_3std = vwap_result.lower_bands[2].clone();
        
        let vp_result = volume_profile::calculate_fast(
            ticks,
            self.config.tick_size,
            self.config.volume_profile_window,
            self.config.price_range,
        );
        indicators.vp_poc = vp_result.poc;
        indicators.vp_vah = vp_result.vah;
        indicators.vp_val = vp_result.val;
        
        let footprint_result = footprint::calculate_fast(ticks, self.config.footprint_window);
        indicators.footprint_delta = footprint_result.total_delta;
        
        let ib_result = initial_balance::calculate_rolling(ticks, self.config.volume_profile_window);
        indicators.ib_high = ib_result.ib_high;
        indicators.ib_low = ib_result.ib_low;
        indicators.ib_mid = ib_result.ib_mid;
        
        let imbalance_result = stacked_imbalance::calculate_rolling(
            ticks,
            self.config.tick_size,
            self.config.volume_profile_window,
            self.config.imbalance_min_stack,
            self.config.imbalance_ratio,
        );
        indicators.has_bullish_imbalance = imbalance_result.has_bullish_signal;
        indicators.has_bearish_imbalance = imbalance_result.has_bearish_signal;
        
        indicators
    }
}

