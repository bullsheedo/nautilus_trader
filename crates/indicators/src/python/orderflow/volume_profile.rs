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

use crate::{indicator::Indicator, orderflow::volume_profile::VolumeProfile};

#[pymethods]
impl VolumeProfile {
    #[new]
    #[pyo3(signature = (tick_size, value_area_pct=0.7, hvn_threshold=1.5, lvn_threshold=0.5, reset_hour_utc=0))]
    fn py_new(
        tick_size: f64,
        value_area_pct: f64,
        hvn_threshold: f64,
        lvn_threshold: f64,
        reset_hour_utc: i32,
    ) -> Self {
        Self::new(
            tick_size,
            value_area_pct,
            hvn_threshold,
            lvn_threshold,
            reset_hour_utc,
        )
    }

    fn __repr__(&self) -> String {
        format!(
            "VolumeProfile({}, {}, {}, {}, {})",
            self.tick_size,
            self.value_area_pct,
            self.hvn_threshold,
            self.lvn_threshold,
            self.reset_hour_utc
        )
    }

    #[getter]
    #[pyo3(name = "name")]
    fn py_name(&self) -> String {
        self.name()
    }

    #[getter]
    #[pyo3(name = "poc")]
    fn py_poc(&self) -> f64 {
        self.poc()
    }

    #[getter]
    #[pyo3(name = "vah")]
    fn py_vah(&mut self) -> f64 {
        self.vah()
    }

    #[getter]
    #[pyo3(name = "val")]
    fn py_val(&mut self) -> f64 {
        self.val()
    }

    #[getter]
    #[pyo3(name = "hvn_levels")]
    fn py_hvn_levels(&mut self) -> Vec<f64> {
        self.hvn_levels().to_vec()
    }

    #[getter]
    #[pyo3(name = "lvn_levels")]
    fn py_lvn_levels(&mut self) -> Vec<f64> {
        self.lvn_levels().to_vec()
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

    #[pyo3(name = "get_volume_at_price")]
    fn py_get_volume_at_price(&self, price: f64) -> f64 {
        self.get_volume_at_price(price)
    }

    #[pyo3(name = "handle_quote_tick")]
    fn py_handle_quote_tick(&mut self, _quote: &QuoteTick) {
        // Not applicable
    }

    #[pyo3(name = "handle_trade_tick")]
    fn py_handle_trade_tick(&mut self, trade: &Bound<'_, PyAny>) -> PyResult<()> {
        // Convert Cython TradeTick to Rust TradeTick
        // First try direct Rust TradeTick, then try Cython conversion
        if let Ok(rust_trade) = trade.extract::<TradeTick>() {
            self.handle_trade(&rust_trade);
        } else {
            // Call to_pyo3() method on Cython TradeTick
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

