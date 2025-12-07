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

use crate::{indicator::Indicator, orderflow::cumulative_delta::CumulativeDelta};

#[pymethods]
impl CumulativeDelta {
    #[new]
    #[pyo3(signature = (reset_hour_utc=-1))]
    fn py_new(reset_hour_utc: i32) -> Self {
        Self::new(reset_hour_utc)
    }

    fn __repr__(&self) -> String {
        format!("CumulativeDelta({})", self.reset_hour_utc)
    }

    #[getter]
    #[pyo3(name = "name")]
    fn py_name(&self) -> String {
        self.name()
    }

    #[getter]
    #[pyo3(name = "reset_hour_utc")]
    const fn py_reset_hour_utc(&self) -> i32 {
        self.reset_hour_utc
    }

    #[getter]
    #[pyo3(name = "value")]
    const fn py_value(&self) -> f64 {
        self.value
    }

    #[getter]
    #[pyo3(name = "buy_volume")]
    const fn py_buy_volume(&self) -> f64 {
        self.buy_volume
    }

    #[getter]
    #[pyo3(name = "sell_volume")]
    const fn py_sell_volume(&self) -> f64 {
        self.sell_volume
    }

    #[getter]
    #[pyo3(name = "last_delta")]
    const fn py_last_delta(&self) -> f64 {
        self.last_delta
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

    #[pyo3(name = "delta_ratio")]
    fn py_delta_ratio(&self) -> f64 {
        self.delta_ratio()
    }

    #[pyo3(name = "buy_sell_ratio")]
    fn py_buy_sell_ratio(&self) -> f64 {
        self.buy_sell_ratio()
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

