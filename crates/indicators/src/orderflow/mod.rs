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

//! Order flow indicators for analyzing market microstructure.
//!
//! This module provides high-performance order flow analysis indicators including:
//! - **CumulativeDelta**: Tracks buy/sell aggressor volume delta
//! - **VWAPBands**: Volume-weighted average price with standard deviation bands
//! - **InitialBalance**: First hour trading range with extensions
//! - **FootprintAggregator**: Price-level bid/ask volume aggregation
//! - **StackedImbalanceDetector**: Detects consecutive imbalanced price levels
//! - **VolumeProfile**: Volume distribution across price levels (POC, VAH, VAL, HVN, LVN)

pub mod cumulative_delta;
pub mod footprint;
pub mod initial_balance;
pub mod stacked_imbalance;
pub mod volume_profile;
pub mod vwap_bands;

// Vectorized versions for ultra-fast backtesting
pub mod vectorized;

pub use cumulative_delta::CumulativeDelta;
pub use footprint::{FootprintAggregator, FootprintLevel};
pub use initial_balance::InitialBalance;
pub use stacked_imbalance::{ImbalanceType, StackedImbalance, StackedImbalanceDetector};
pub use volume_profile::VolumeProfile;
pub use vwap_bands::VWAPBands;

