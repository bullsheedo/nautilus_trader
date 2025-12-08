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

//! Performance statistics calculation.

use super::positions::Position;

#[derive(Debug, Clone)]
pub struct PerformanceStats {
    pub total_trades: usize,
    pub winning_trades: usize,
    pub losing_trades: usize,
    pub win_rate: f64,
    pub total_pnl: f64,
    pub avg_win: f64,
    pub avg_loss: f64,
    pub largest_win: f64,
    pub largest_loss: f64,
    pub profit_factor: f64,
    pub sharpe_ratio: f64,
    pub max_drawdown: f64,
    pub avg_trade_duration: f64,
}

impl PerformanceStats {
    pub fn calculate(positions: &[Position]) -> Self {
        if positions.is_empty() {
            return Self::default();
        }
        
        let total_trades = positions.len();
        let mut winning_trades = 0;
        let mut losing_trades = 0;
        let mut total_pnl = 0.0;
        let mut total_wins = 0.0;
        let mut total_losses = 0.0;
        let mut largest_win = f64::NEG_INFINITY;
        let mut largest_loss = f64::INFINITY;
        let mut pnl_series = Vec::with_capacity(total_trades);
        let mut total_duration = 0;
        
        for pos in positions {
            total_pnl += pos.pnl;
            pnl_series.push(total_pnl);
            
            if pos.pnl > 0.0 {
                winning_trades += 1;
                total_wins += pos.pnl;
                largest_win = largest_win.max(pos.pnl);
            } else if pos.pnl < 0.0 {
                losing_trades += 1;
                total_losses += pos.pnl.abs();
                largest_loss = largest_loss.min(pos.pnl);
            }
            
            if let Some(exit_idx) = pos.exit_index {
                total_duration += exit_idx - pos.entry_index;
            }
        }
        
        let win_rate = if total_trades > 0 {
            (winning_trades as f64 / total_trades as f64) * 100.0
        } else {
            0.0
        };
        
        let avg_win = if winning_trades > 0 {
            total_wins / winning_trades as f64
        } else {
            0.0
        };
        
        let avg_loss = if losing_trades > 0 {
            total_losses / losing_trades as f64
        } else {
            0.0
        };
        
        let profit_factor = if total_losses > 0.0 {
            total_wins / total_losses
        } else if total_wins > 0.0 {
            f64::INFINITY
        } else {
            0.0
        };
        
        // Calculate Sharpe Ratio (simplified - assumes daily returns)
        let sharpe_ratio = if !pnl_series.is_empty() {
            let mean_return = total_pnl / pnl_series.len() as f64;
            let variance = pnl_series
                .iter()
                .map(|&x| {
                    let diff = x - mean_return;
                    diff * diff
                })
                .sum::<f64>()
                / pnl_series.len() as f64;
            let std_dev = variance.sqrt();
            
            if std_dev > 0.0 {
                mean_return / std_dev
            } else {
                0.0
            }
        } else {
            0.0
        };
        
        // Calculate Maximum Drawdown
        let max_drawdown = calculate_max_drawdown(&pnl_series);
        
        let avg_trade_duration = if total_trades > 0 {
            total_duration as f64 / total_trades as f64
        } else {
            0.0
        };
        
        Self {
            total_trades,
            winning_trades,
            losing_trades,
            win_rate,
            total_pnl,
            avg_win,
            avg_loss,
            largest_win: if largest_win.is_finite() { largest_win } else { 0.0 },
            largest_loss: if largest_loss.is_finite() { largest_loss } else { 0.0 },
            profit_factor,
            sharpe_ratio,
            max_drawdown,
            avg_trade_duration,
        }
    }
}

impl Default for PerformanceStats {
    fn default() -> Self {
        Self {
            total_trades: 0,
            winning_trades: 0,
            losing_trades: 0,
            win_rate: 0.0,
            total_pnl: 0.0,
            avg_win: 0.0,
            avg_loss: 0.0,
            largest_win: 0.0,
            largest_loss: 0.0,
            profit_factor: 0.0,
            sharpe_ratio: 0.0,
            max_drawdown: 0.0,
            avg_trade_duration: 0.0,
        }
    }
}

fn calculate_max_drawdown(pnl_series: &[f64]) -> f64 {
    if pnl_series.is_empty() {
        return 0.0;
    }
    
    let mut max_drawdown = 0.0;
    let mut peak = pnl_series[0];
    
    for &pnl in pnl_series {
        if pnl > peak {
            peak = pnl;
        }
        
        let drawdown = peak - pnl;
        if drawdown > max_drawdown {
            max_drawdown = drawdown;
        }
    }
    
    max_drawdown
}

