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

//! Python bindings for vectorized backtesting.

use pyo3::prelude::*;
use pyo3::types::PyList;
use nautilus_model::data::TradeTick;
use nautilus_indicators::orderflow::vectorized::TickArrays;

use crate::vectorized::{
    VectorizedBacktest, BacktestConfig, BacktestResult,
    PerformanceStats,
};

/// Python wrapper for BacktestConfig
#[pyclass(name = "BacktestConfig")]
#[derive(Debug, Clone)]
pub struct PyBacktestConfig {
    #[pyo3(get, set)]
    pub vwap_window: usize,
    #[pyo3(get, set)]
    pub volume_profile_window: usize,
    #[pyo3(get, set)]
    pub footprint_window: usize,
    #[pyo3(get, set)]
    pub ib_period_minutes: u64,
    #[pyo3(get, set)]
    pub imbalance_min_stack: usize,
    #[pyo3(get, set)]
    pub imbalance_ratio: f64,
    #[pyo3(get, set)]
    pub poi_tolerance: f64,
    #[pyo3(get, set)]
    pub tick_size: f64,
    #[pyo3(get, set)]
    pub take_profit_ticks: f64,
    #[pyo3(get, set)]
    pub stop_loss_ticks: f64,
    #[pyo3(get, set)]
    pub trailing_stop_ticks: f64,
    #[pyo3(get, set)]
    pub warmup_ticks: usize,
    #[pyo3(get, set)]
    pub price_range_min: f64,
    #[pyo3(get, set)]
    pub price_range_max: f64,
}

#[pymethods]
impl PyBacktestConfig {
    #[new]
    #[pyo3(signature = (
        vwap_window=1000,
        volume_profile_window=1000,
        footprint_window=100,
        ib_period_minutes=60,
        imbalance_min_stack=3,
        imbalance_ratio=1.5,
        poi_tolerance=3.0,
        tick_size=0.01,
        take_profit_ticks=0.30,
        stop_loss_ticks=0.35,
        trailing_stop_ticks=0.20,
        warmup_ticks=1000,
        price_range_min=0.0,
        price_range_max=10000.0,
    ))]
    #[allow(clippy::too_many_arguments)]
    fn py_new(
        vwap_window: usize,
        volume_profile_window: usize,
        footprint_window: usize,
        ib_period_minutes: u64,
        imbalance_min_stack: usize,
        imbalance_ratio: f64,
        poi_tolerance: f64,
        tick_size: f64,
        take_profit_ticks: f64,
        stop_loss_ticks: f64,
        trailing_stop_ticks: f64,
        warmup_ticks: usize,
        price_range_min: f64,
        price_range_max: f64,
    ) -> Self {
        Self {
            vwap_window,
            volume_profile_window,
            footprint_window,
            ib_period_minutes,
            imbalance_min_stack,
            imbalance_ratio,
            poi_tolerance,
            tick_size,
            take_profit_ticks,
            stop_loss_ticks,
            trailing_stop_ticks,
            warmup_ticks,
            price_range_min,
            price_range_max,
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "BacktestConfig(vwap_window={}, volume_profile_window={}, footprint_window={}, \
             ib_period_minutes={}, imbalance_min_stack={}, imbalance_ratio={}, \
             poi_tolerance={}, tick_size={}, take_profit_ticks={}, stop_loss_ticks={}, \
             trailing_stop_ticks={}, warmup_ticks={}, price_range=({}, {}))",
            self.vwap_window,
            self.volume_profile_window,
            self.footprint_window,
            self.ib_period_minutes,
            self.imbalance_min_stack,
            self.imbalance_ratio,
            self.poi_tolerance,
            self.tick_size,
            self.take_profit_ticks,
            self.stop_loss_ticks,
            self.trailing_stop_ticks,
            self.warmup_ticks,
            self.price_range_min,
            self.price_range_max,
        )
    }
}

impl From<PyBacktestConfig> for BacktestConfig {
    fn from(py_config: PyBacktestConfig) -> Self {
        BacktestConfig {
            vwap_window: py_config.vwap_window,
            volume_profile_window: py_config.volume_profile_window,
            footprint_window: py_config.footprint_window,
            ib_period_minutes: py_config.ib_period_minutes,
            imbalance_min_stack: py_config.imbalance_min_stack,
            imbalance_ratio: py_config.imbalance_ratio,
            poi_tolerance: py_config.poi_tolerance,
            tick_size: py_config.tick_size,
            take_profit_ticks: py_config.take_profit_ticks,
            stop_loss_ticks: py_config.stop_loss_ticks,
            trailing_stop_ticks: py_config.trailing_stop_ticks,
            warmup_ticks: py_config.warmup_ticks,
            price_range: (py_config.price_range_min, py_config.price_range_max),
        }
    }
}

/// Python wrapper for PerformanceStats
#[pyclass(name = "PerformanceStats")]
#[derive(Debug, Clone)]
pub struct PyPerformanceStats {
    #[pyo3(get)]
    pub total_trades: usize,
    #[pyo3(get)]
    pub winning_trades: usize,
    #[pyo3(get)]
    pub losing_trades: usize,
    #[pyo3(get)]
    pub win_rate: f64,
    #[pyo3(get)]
    pub total_pnl: f64,
    #[pyo3(get)]
    pub avg_win: f64,
    #[pyo3(get)]
    pub avg_loss: f64,
    #[pyo3(get)]
    pub profit_factor: f64,
    #[pyo3(get)]
    pub sharpe_ratio: f64,
    #[pyo3(get)]
    pub max_drawdown: f64,
}

#[pymethods]
impl PyPerformanceStats {
    fn __repr__(&self) -> String {
        format!(
            "PerformanceStats(total_trades={}, win_rate={:.2}%, total_pnl={:.2}, \
             profit_factor={:.2}, sharpe_ratio={:.2}, max_drawdown={:.2})",
            self.total_trades,
            self.win_rate,  // Already in percentage (0-100)
            self.total_pnl,
            self.profit_factor,
            self.sharpe_ratio,
            self.max_drawdown,
        )
    }
}

impl From<PerformanceStats> for PyPerformanceStats {
    fn from(stats: PerformanceStats) -> Self {
        Self {
            total_trades: stats.total_trades,
            winning_trades: stats.winning_trades,
            losing_trades: stats.losing_trades,
            win_rate: stats.win_rate,
            total_pnl: stats.total_pnl,
            avg_win: stats.avg_win,
            avg_loss: stats.avg_loss,
            profit_factor: stats.profit_factor,
            sharpe_ratio: stats.sharpe_ratio,
            max_drawdown: stats.max_drawdown,
        }
    }
}

/// Python wrapper for BacktestResult
#[pyclass(name = "BacktestResult")]
#[derive(Debug, Clone)]
pub struct PyBacktestResult {
    #[pyo3(get)]
    pub stats: PyPerformanceStats,
    #[pyo3(get)]
    pub elapsed_seconds: f64,
    #[pyo3(get)]
    pub ticks_per_second: f64,
}

#[pymethods]
impl PyBacktestResult {
    fn __repr__(&self) -> String {
        format!(
            "BacktestResult(stats={}, elapsed={:.2}s, ticks_per_sec={:.0})",
            self.stats.__repr__(),
            self.elapsed_seconds,
            self.ticks_per_second,
        )
    }
}

impl From<BacktestResult> for PyBacktestResult {
    fn from(result: BacktestResult) -> Self {
        Self {
            stats: result.stats.into(),
            elapsed_seconds: result.elapsed_seconds,
            ticks_per_second: result.ticks_per_second,
        }
    }
}

/// Python wrapper for VectorizedBacktest
#[pyclass(name = "VectorizedBacktest")]
#[derive(Debug)]
pub struct PyVectorizedBacktest {
    inner: VectorizedBacktest,
}

#[pymethods]
impl PyVectorizedBacktest {
    #[new]
    fn py_new(config: PyBacktestConfig) -> Self {
        Self {
            inner: VectorizedBacktest::new(config.into()),
        }
    }

    /// Run the vectorized backtest on a list of TradeTick objects.
    ///
    /// # Arguments
    ///
    /// * `ticks` - A Python list of TradeTick objects
    ///
    /// # Returns
    ///
    /// A BacktestResult containing performance statistics
    ///
    /// # Errors
    ///
    /// Returns a `PyErr` if tick conversion fails
    fn run(&self, ticks: &Bound<'_, PyList>) -> PyResult<PyBacktestResult> {
        // Convert Python list of TradeTick to Vec<TradeTick>
        // Use from_pyobject to handle Cython TradeTick objects
        let tick_vec: Vec<TradeTick> = ticks
            .iter()
            .map(|item| TradeTick::from_pyobject(&item))
            .collect::<PyResult<Vec<_>>>()?;

        // Convert to TickArrays
        let tick_arrays = TickArrays::from_ticks(&tick_vec);

        // Run the backtest
        let result = self.inner.run(&tick_arrays);

        Ok(result.into())
    }

    fn __repr__(&self) -> String {
        format!("VectorizedBacktest({:?})", self.inner)
    }
}

