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
    orderflow::footprint::{FootprintAggregator, FootprintLevel},
};

#[pymethods]
impl FootprintLevel {
    #[new]
    fn py_new() -> Self {
        Self::new()
    }

    #[getter]
    #[pyo3(name = "delta")]
    fn py_delta(&self) -> f64 {
        self.delta()
    }

    #[getter]
    #[pyo3(name = "total_volume")]
    fn py_total_volume(&self) -> f64 {
        self.total_volume()
    }

    #[getter]
    #[pyo3(name = "imbalance_ratio")]
    fn py_imbalance_ratio(&self) -> f64 {
        self.imbalance_ratio()
    }
}

#[pymethods]
impl FootprintAggregator {
    #[new]
    #[pyo3(signature = (tick_size, imbalance_threshold=3.0))]
    fn py_new(tick_size: f64, imbalance_threshold: f64) -> Self {
        Self::new(tick_size, imbalance_threshold)
    }

    fn __repr__(&self) -> String {
        format!(
            "FootprintAggregator({}, {})",
            self.tick_size, self.imbalance_threshold
        )
    }

    #[getter]
    #[pyo3(name = "name")]
    fn py_name(&self) -> String {
        self.name()
    }

    #[getter]
    #[pyo3(name = "poc_price")]
    const fn py_poc_price(&self) -> f64 {
        self.poc_price
    }

    #[getter]
    #[pyo3(name = "total_delta")]
    const fn py_total_delta(&self) -> f64 {
        self.total_delta
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

    #[pyo3(name = "get_level")]
    fn py_get_level(&self, price: f64) -> FootprintLevel {
        self.get_level(price)
    }

    #[pyo3(name = "clear_footprint")]
    fn py_clear_footprint(&mut self) {
        self.clear_footprint();
    }

    #[pyo3(name = "handle_quote_tick")]
    fn py_handle_quote_tick(&mut self, _quote: &QuoteTick) {
        // Not applicable
    }

    #[pyo3(name = "handle_trade_tick")]
    fn py_handle_trade_tick(&mut self, trade: &Bound<'_, PyAny>) -> PyResult<()> {
        // Convert Cython TradeTick to Rust TradeTick
        if let Ok(rust_trade) = trade.extract::<TradeTick>() {
            self.update_trade(&rust_trade);
        } else {
            let pyo3_trade = trade.call_method0("to_pyo3")?;
            let rust_trade: TradeTick = pyo3_trade.extract()?;
            self.update_trade(&rust_trade);
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

