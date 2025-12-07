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

use crate::{indicator::Indicator, orderflow::initial_balance::InitialBalance};

#[pymethods]
impl InitialBalance {
    #[new]
    #[pyo3(signature = (num_extensions=4, extension_multiplier=1.0, ib_duration_minutes=60))]
    fn py_new(num_extensions: usize, extension_multiplier: f64, ib_duration_minutes: i32) -> Self {
        Self::new(num_extensions, extension_multiplier, ib_duration_minutes)
    }

    fn __repr__(&self) -> String {
        format!(
            "InitialBalance({}, {}, {})",
            self.num_extensions, self.extension_multiplier, self.ib_duration_minutes
        )
    }

    #[getter]
    #[pyo3(name = "name")]
    fn py_name(&self) -> String {
        self.name()
    }

    #[getter]
    #[pyo3(name = "ib_high")]
    const fn py_ib_high(&self) -> f64 {
        self.ib_high
    }

    #[getter]
    #[pyo3(name = "ib_low")]
    const fn py_ib_low(&self) -> f64 {
        self.ib_low
    }

    #[getter]
    #[pyo3(name = "ib_mid")]
    const fn py_ib_mid(&self) -> f64 {
        self.ib_mid
    }

    #[getter]
    #[pyo3(name = "ib_range")]
    const fn py_ib_range(&self) -> f64 {
        self.ib_range
    }

    #[getter]
    #[pyo3(name = "extensions_above")]
    fn py_extensions_above(&self) -> Vec<f64> {
        self.extensions_above.clone()
    }

    #[getter]
    #[pyo3(name = "extensions_below")]
    fn py_extensions_below(&self) -> Vec<f64> {
        self.extensions_below.clone()
    }

    #[getter]
    #[pyo3(name = "is_complete")]
    fn py_is_complete(&self) -> bool {
        self.is_complete()
    }

    #[getter]
    #[pyo3(name = "is_forming")]
    fn py_is_forming(&self) -> bool {
        self.is_forming()
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
    fn py_handle_bar(&mut self, bar: &Bar) {
        self.handle_bar(bar);
    }

    #[pyo3(name = "reset")]
    fn py_reset(&mut self) {
        self.reset();
    }
}

