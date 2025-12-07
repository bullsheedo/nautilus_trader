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

use nautilus_model::data::{Bar, QuoteTick, TradeTick};
use pyo3::prelude::*;

use crate::{
    indicator::Indicator,
    orderflow::stacked_imbalance::{ImbalanceType, StackedImbalance, StackedImbalanceDetector},
};

#[pymethods]
impl StackedImbalance {
    #[getter]
    fn imbalance_type(&self) -> ImbalanceType {
        self.imbalance_type
    }

    #[getter]
    fn start_price(&self) -> f64 {
        self.start_price
    }

    #[getter]
    fn end_price(&self) -> f64 {
        self.end_price
    }

    #[getter]
    fn num_levels(&self) -> usize {
        self.num_levels
    }

    #[getter]
    fn total_delta(&self) -> f64 {
        self.total_delta
    }
}

#[pymethods]
impl StackedImbalanceDetector {
    #[new]
    #[pyo3(signature = (tick_size, imbalance_ratio=3.0, min_stack_count=3, min_volume_per_level=0.0))]
    fn py_new(
        tick_size: f64,
        imbalance_ratio: f64,
        min_stack_count: usize,
        min_volume_per_level: f64,
    ) -> Self {
        Self::new(
            tick_size,
            imbalance_ratio,
            min_stack_count,
            min_volume_per_level,
        )
    }

    fn __repr__(&self) -> String {
        format!(
            "StackedImbalanceDetector({}, {}, {}, {})",
            self.tick_size, self.imbalance_ratio, self.min_stack_count, self.min_volume_per_level
        )
    }

    #[getter]
    #[pyo3(name = "name")]
    fn py_name(&self) -> String {
        self.name()
    }

    #[getter]
    #[pyo3(name = "stacked_ask_imbalances")]
    fn py_stacked_ask_imbalances(&mut self) -> Vec<StackedImbalance> {
        self.stacked_ask_imbalances().to_vec()
    }

    #[getter]
    #[pyo3(name = "stacked_bid_imbalances")]
    fn py_stacked_bid_imbalances(&mut self) -> Vec<StackedImbalance> {
        self.stacked_bid_imbalances().to_vec()
    }

    #[getter]
    #[pyo3(name = "last_signal")]
    fn py_last_signal(&mut self) -> ImbalanceType {
        self.last_signal()
    }

    #[getter]
    #[pyo3(name = "has_bullish_signal")]
    fn py_has_bullish_signal(&mut self) -> bool {
        self.has_bullish_signal()
    }

    #[getter]
    #[pyo3(name = "has_bearish_signal")]
    fn py_has_bearish_signal(&mut self) -> bool {
        self.has_bearish_signal()
    }

    #[getter]
    #[pyo3(name = "has_inputs")]
    fn py_has_inputs(&self) -> bool {
        self.has_inputs()
    }

    #[getter]
    #[pyo3(name = "initialized")]
    const fn py_initialized(&self) -> bool {
        self.initialized
    }

    #[pyo3(name = "clear")]
    fn py_clear(&mut self) {
        self.clear();
    }

    #[pyo3(name = "handle_quote_tick")]
    fn py_handle_quote_tick(&mut self, _quote: &QuoteTick) {
        // Not applicable
    }

    #[pyo3(name = "handle_trade_tick")]
    fn py_handle_trade_tick(&mut self, trade: &Bound<'_, PyAny>) -> PyResult<()> {
        // Convert Cython TradeTick to Rust TradeTick
        if let Ok(rust_trade) = trade.extract::<TradeTick>() {
            self.handle_trade(&rust_trade);
        } else {
            let pyo3_trade = trade.call_method0("to_pyo3")?;
            let rust_trade: TradeTick = pyo3_trade.extract()?;
            self.handle_trade(&rust_trade);
        }
        Ok(())
    }

    #[pyo3(name = "handle_bar")]
    fn py_handle_bar(&mut self, _bar: &Bar) {
        // Not applicable
    }

    #[pyo3(name = "reset")]
    fn py_reset(&mut self) {
        self.reset();
    }
}

