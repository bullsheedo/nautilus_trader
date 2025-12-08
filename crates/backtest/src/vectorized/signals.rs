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

//! Vectorized signal generation for orderflow strategy.

use nautilus_indicators::orderflow::vectorized::IndicatorArrays;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalType {
    Long,
    Short,
    ExitLong,
    ExitShort,
    None,
}

#[derive(Debug, Clone)]
pub struct Signal {
    pub signal_type: SignalType,
    pub price: f64,
    pub index: usize,
    pub stop_loss: Option<f64>,
    pub take_profit: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct SignalConfig {
    pub poi_tolerance: f64,
    pub tick_size: f64,
    pub take_profit_ticks: f64,
    pub stop_loss_ticks: f64,
    pub trailing_stop_ticks: f64,
}

#[derive(Debug)]
pub struct SignalGenerator {
    config: SignalConfig,
}

impl SignalGenerator {
    pub fn new(config: SignalConfig) -> Self {
        Self { config }
    }
    
    /// Generate all signals at once (vectorized)
    pub fn generate_signals(
        &self,
        prices: &[f64],
        indicators: &IndicatorArrays,
    ) -> Vec<Signal> {
        let len = prices.len();
        let mut signals = Vec::new();
        
        // Track current position state
        let mut in_position = false;
        let mut position_side = SignalType::None;
        let mut entry_price = 0.0;
        
        for i in 0..len {
            let price = prices[i];
            
            // Check for exit signals first
            if in_position {
                let exit_signal = self.check_exit(
                    i,
                    price,
                    entry_price,
                    position_side,
                    indicators,
                );
                
                if exit_signal.signal_type != SignalType::None {
                    signals.push(exit_signal);
                    in_position = false;
                    position_side = SignalType::None;
                    continue;
                }
            }
            
            // Check for entry signals if not in position
            if !in_position {
                let entry_signal = self.check_entry(i, price, indicators);
                
                if entry_signal.signal_type != SignalType::None {
                    signals.push(entry_signal.clone());
                    in_position = true;
                    position_side = entry_signal.signal_type;
                    entry_price = price;
                }
            }
        }
        
        signals
    }
    
    /// Check for entry signals at index i
    fn check_entry(&self, i: usize, price: f64, indicators: &IndicatorArrays) -> Signal {
        // Get POI context
        let at_poi = self.is_at_poi(i, price, indicators);
        
        if !at_poi {
            return Signal {
                signal_type: SignalType::None,
                price,
                index: i,
                stop_loss: None,
                take_profit: None,
            };
        }
        
        // Get orderflow bias
        let bias = self.get_orderflow_bias(i, indicators);
        
        let signal_type = match bias {
            1 => SignalType::Long,
            -1 => SignalType::Short,
            _ => SignalType::None,
        };
        
        if signal_type == SignalType::None {
            return Signal {
                signal_type: SignalType::None,
                price,
                index: i,
                stop_loss: None,
                take_profit: None,
            };
        }
        
        // Calculate stop loss and take profit
        let (stop_loss, take_profit) = match signal_type {
            SignalType::Long => (
                Some(price - self.config.stop_loss_ticks * self.config.tick_size),
                Some(price + self.config.take_profit_ticks * self.config.tick_size),
            ),
            SignalType::Short => (
                Some(price + self.config.stop_loss_ticks * self.config.tick_size),
                Some(price - self.config.take_profit_ticks * self.config.tick_size),
            ),
            _ => (None, None),
        };
        
        Signal {
            signal_type,
            price,
            index: i,
            stop_loss,
            take_profit,
        }
    }
    
    /// Check for exit signals
    fn check_exit(
        &self,
        i: usize,
        price: f64,
        _entry_price: f64,
        position_side: SignalType,
        indicators: &IndicatorArrays,
    ) -> Signal {
        // Check if moving away from POI
        let away_from_poi = self.is_away_from_poi(i, price, indicators);
        
        if away_from_poi {
            let signal_type = match position_side {
                SignalType::Long => SignalType::ExitLong,
                SignalType::Short => SignalType::ExitShort,
                _ => SignalType::None,
            };
            
            return Signal {
                signal_type,
                price,
                index: i,
                stop_loss: None,
                take_profit: None,
            };
        }
        
        Signal {
            signal_type: SignalType::None,
            price,
            index: i,
            stop_loss: None,
            take_profit: None,
        }
    }

    /// Check if price is at a POI
    fn is_at_poi(&self, i: usize, price: f64, indicators: &IndicatorArrays) -> bool {
        let tolerance = self.config.poi_tolerance * self.config.tick_size;

        // Check Volume Profile levels
        if (price - indicators.vp_vah[i]).abs() <= tolerance {
            return true;
        }
        if (price - indicators.vp_val[i]).abs() <= tolerance {
            return true;
        }
        if (price - indicators.vp_poc[i]).abs() <= tolerance {
            return true;
        }

        // Check VWAP levels
        if (price - indicators.vwap[i]).abs() <= tolerance {
            return true;
        }
        if (price - indicators.vwap_upper_1std[i]).abs() <= tolerance {
            return true;
        }
        if (price - indicators.vwap_lower_1std[i]).abs() <= tolerance {
            return true;
        }

        // Check Initial Balance levels
        if (price - indicators.ib_high[i]).abs() <= tolerance {
            return true;
        }
        if (price - indicators.ib_low[i]).abs() <= tolerance {
            return true;
        }

        false
    }

    /// Check if price has moved away from POI
    fn is_away_from_poi(&self, i: usize, price: f64, indicators: &IndicatorArrays) -> bool {
        let tolerance = self.config.poi_tolerance * self.config.tick_size * 2.0; // 2x tolerance

        // If we're far from all POIs, we've moved away
        let near_vp = (price - indicators.vp_vah[i]).abs() <= tolerance
            || (price - indicators.vp_val[i]).abs() <= tolerance
            || (price - indicators.vp_poc[i]).abs() <= tolerance;

        let near_vwap = (price - indicators.vwap[i]).abs() <= tolerance
            || (price - indicators.vwap_upper_1std[i]).abs() <= tolerance
            || (price - indicators.vwap_lower_1std[i]).abs() <= tolerance;

        let near_ib = (price - indicators.ib_high[i]).abs() <= tolerance
            || (price - indicators.ib_low[i]).abs() <= tolerance;

        !near_vp && !near_vwap && !near_ib
    }

    /// Get orderflow bias (-1 = bearish, 0 = neutral, 1 = bullish)
    fn get_orderflow_bias(&self, i: usize, indicators: &IndicatorArrays) -> i8 {
        let mut bullish_signals = 0;
        let mut bearish_signals = 0;

        // Cumulative Delta
        let delta = indicators.cumulative_delta[i];
        if delta > 50.0 {
            bullish_signals += 1;
        } else if delta < -50.0 {
            bearish_signals += 1;
        }

        // Stacked Imbalances
        if indicators.has_bullish_imbalance[i] {
            bullish_signals += 2;
        }
        if indicators.has_bearish_imbalance[i] {
            bearish_signals += 2;
        }

        // Footprint Delta
        let footprint_delta = indicators.footprint_delta[i];
        if footprint_delta > 100.0 {
            bullish_signals += 1;
        } else if footprint_delta < -100.0 {
            bearish_signals += 1;
        }

        if bullish_signals > bearish_signals {
            1
        } else if bearish_signals > bullish_signals {
            -1
        } else {
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signal_generation() {
        // Basic test to ensure signal generation compiles
        let config = SignalConfig {
            poi_tolerance: 3.0,
            tick_size: 0.01,
            take_profit_ticks: 0.30,
            stop_loss_ticks: 0.35,
            trailing_stop_ticks: 0.20,
        };

        let generator = SignalGenerator::new(config);
        assert!(generator.config.poi_tolerance == 3.0);
    }
}
