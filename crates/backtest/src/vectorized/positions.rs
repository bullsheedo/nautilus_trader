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

//! Vectorized position tracking for backtesting.

use super::signals::{Signal, SignalType};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PositionSide {
    Long,
    Short,
    Flat,
}

#[derive(Debug, Clone)]
pub struct Position {
    pub side: PositionSide,
    pub entry_price: f64,
    pub entry_index: usize,
    pub exit_price: Option<f64>,
    pub exit_index: Option<usize>,
    pub stop_loss: Option<f64>,
    pub take_profit: Option<f64>,
    pub pnl: f64,
    pub pnl_pct: f64,
}

#[derive(Debug, Clone)]
pub struct PositionTracker {
    pub positions: Vec<Position>,
    pub current_position: Option<Position>,
}

impl PositionTracker {
    pub fn new() -> Self {
        Self {
            positions: Vec::new(),
            current_position: None,
        }
    }
    
    /// Process all ticks and signals to track positions
    pub fn process_ticks(
        &mut self,
        prices: &[f64],
        signals: &[Signal],
    ) {
        let mut signal_idx = 0;
        
        for i in 0..prices.len() {
            let price = prices[i];
            
            // Check if we have a signal at this index
            while signal_idx < signals.len() && signals[signal_idx].index == i {
                let signal = &signals[signal_idx];
                self.process_signal(signal, price, i);
                signal_idx += 1;
            }
            
            // Check stop loss / take profit
            if let Some(ref mut pos) = self.current_position {
                if let Some(sl) = pos.stop_loss {
                    let hit_sl = match pos.side {
                        PositionSide::Long => price <= sl,
                        PositionSide::Short => price >= sl,
                        PositionSide::Flat => false,
                    };
                    
                    if hit_sl {
                        self.close_position(sl, i);
                        continue;
                    }
                }
                
                if let Some(tp) = pos.take_profit {
                    let hit_tp = match pos.side {
                        PositionSide::Long => price >= tp,
                        PositionSide::Short => price <= tp,
                        PositionSide::Flat => false,
                    };
                    
                    if hit_tp {
                        self.close_position(tp, i);
                        continue;
                    }
                }
            }
        }
        
        // Close any remaining position at last price
        if self.current_position.is_some() {
            let last_price = prices[prices.len() - 1];
            let last_index = prices.len() - 1;
            self.close_position(last_price, last_index);
        }
    }
    
    fn process_signal(&mut self, signal: &Signal, price: f64, index: usize) {
        match signal.signal_type {
            SignalType::Long => {
                // Close any existing short position
                if let Some(ref pos) = self.current_position {
                    if pos.side == PositionSide::Short {
                        self.close_position(price, index);
                    }
                }
                
                // Open long position
                self.current_position = Some(Position {
                    side: PositionSide::Long,
                    entry_price: price,
                    entry_index: index,
                    exit_price: None,
                    exit_index: None,
                    stop_loss: signal.stop_loss,
                    take_profit: signal.take_profit,
                    pnl: 0.0,
                    pnl_pct: 0.0,
                });
            }
            SignalType::Short => {
                // Close any existing long position
                if let Some(ref pos) = self.current_position {
                    if pos.side == PositionSide::Long {
                        self.close_position(price, index);
                    }
                }
                
                // Open short position
                self.current_position = Some(Position {
                    side: PositionSide::Short,
                    entry_price: price,
                    entry_index: index,
                    exit_price: None,
                    exit_index: None,
                    stop_loss: signal.stop_loss,
                    take_profit: signal.take_profit,
                    pnl: 0.0,
                    pnl_pct: 0.0,
                });
            }
            SignalType::ExitLong | SignalType::ExitShort => {
                if self.current_position.is_some() {
                    self.close_position(price, index);
                }
            }
            SignalType::None => {}
        }
    }
    
    fn close_position(&mut self, exit_price: f64, exit_index: usize) {
        if let Some(mut pos) = self.current_position.take() {
            pos.exit_price = Some(exit_price);
            pos.exit_index = Some(exit_index);
            
            // Calculate P&L
            pos.pnl = match pos.side {
                PositionSide::Long => exit_price - pos.entry_price,
                PositionSide::Short => pos.entry_price - exit_price,
                PositionSide::Flat => 0.0,
            };
            
            pos.pnl_pct = (pos.pnl / pos.entry_price) * 100.0;
            
            self.positions.push(pos);
        }
    }
}

impl Default for PositionTracker {
    fn default() -> Self {
        Self::new()
    }
}

