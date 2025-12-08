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

//! Vectorized backtesting engine for ultra-fast strategy evaluation.
//!
//! This module provides a completely different approach to backtesting:
//! - Processes all ticks at once (vectorized)
//! - No event loop overhead
//! - SIMD optimizations where possible
//! - 100-1000x faster than event-driven backtesting

pub mod engine;
pub mod signals;
pub mod positions;
pub mod statistics;

#[cfg(test)]
mod tests;

pub use engine::{VectorizedBacktest, BacktestConfig, BacktestResult};
pub use signals::{SignalGenerator, Signal, SignalType};
pub use positions::{PositionTracker, Position, PositionSide};
pub use statistics::PerformanceStats;

