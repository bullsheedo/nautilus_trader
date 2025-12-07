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

use crate::{indicator::Indicator, orderflow::vwap_bands::VWAPBands};

#[pymethods]
impl VWAPBands {
    #[new]
    #[pyo3(signature = (reset_hour_utc=0, num_std_bands=3))]
    fn py_new(reset_hour_utc: i32, num_std_bands: usize) -> Self {
        Self::new(reset_hour_utc, num_std_bands)
    }

    fn __repr__(&self) -> String {
        format!("VWAPBands({}, {})", self.reset_hour_utc, self.num_std_bands)
    }

    #[getter]
    #[pyo3(name = "name")]
    fn py_name(&self) -> String {
        self.name()
    }

    #[getter]
    #[pyo3(name = "vwap")]
    const fn py_vwap(&self) -> f64 {
        self.vwap
    }

    #[getter]
    #[pyo3(name = "std_dev")]
    const fn py_std_dev(&self) -> f64 {
        self.std_dev
    }

    #[getter]
    #[pyo3(name = "upper_bands")]
    fn py_upper_bands(&self) -> Vec<f64> {
        self.upper_bands.clone()
    }

    #[getter]
    #[pyo3(name = "lower_bands")]
    fn py_lower_bands(&self) -> Vec<f64> {
        self.lower_bands.clone()
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

