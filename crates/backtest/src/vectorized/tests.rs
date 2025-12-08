//! Unit tests for vectorized backtesting engine

#[cfg(test)]
mod tests {
    use crate::vectorized::{
        engine::{BacktestConfig, VectorizedBacktest},
        positions::Position,
        statistics::PerformanceStats,
    };
    use nautilus_indicators::orderflow::vectorized::TickArrays;

    #[test]
    fn test_backtest_config_creation() {
        let config = BacktestConfig {
            vwap_window: 1000,
            volume_profile_window: 1000,
            footprint_window: 100,
            ib_period_minutes: 60,
            imbalance_min_stack: 3,
            imbalance_ratio: 1.5,
            poi_tolerance: 3.0,
            tick_size: 0.01,
            take_profit_ticks: 0.30,
            stop_loss_ticks: 0.35,
            trailing_stop_ticks: 0.20,
            price_range: (2000.0, 2500.0),
        };

        assert_eq!(config.vwap_window, 1000);
        assert_eq!(config.poi_tolerance, 3.0);
        assert_eq!(config.tick_size, 0.01);
    }

    #[test]
    fn test_vectorized_backtest_empty_data() {
        let config = BacktestConfig {
            vwap_window: 1000,
            volume_profile_window: 1000,
            footprint_window: 100,
            ib_period_minutes: 60,
            imbalance_min_stack: 3,
            imbalance_ratio: 1.5,
            poi_tolerance: 3.0,
            tick_size: 0.01,
            take_profit_ticks: 0.30,
            stop_loss_ticks: 0.35,
            trailing_stop_ticks: 0.20,
            price_range: (2000.0, 2500.0),
        };

        let backtest = VectorizedBacktest::new(config);
        let ticks = TickArrays {
            prices: vec![],
            quantities: vec![],
            is_buyer: vec![],
            timestamps: vec![],
            len: 0,
        };

        let result = backtest.run(&ticks);

        assert_eq!(result.stats.total_trades, 0);
        assert_eq!(result.stats.win_rate, 0.0);
        assert_eq!(result.stats.total_pnl, 0.0);
    }

    #[test]
    fn test_vectorized_backtest_simple_data() {
        let config = BacktestConfig {
            vwap_window: 10,
            volume_profile_window: 10,
            footprint_window: 5,
            ib_period_minutes: 60,
            imbalance_min_stack: 3,
            imbalance_ratio: 1.5,
            poi_tolerance: 3.0,
            tick_size: 0.01,
            take_profit_ticks: 0.30,
            stop_loss_ticks: 0.35,
            trailing_stop_ticks: 0.20,
            price_range: (2200.0, 2300.0),
        };

        let backtest = VectorizedBacktest::new(config);

        // Create simple oscillating price data
        let mut prices = vec![];
        let mut quantities = vec![];
        let mut is_buyer = vec![];
        let mut timestamps = vec![];

        for i in 0..100 {
            let price = 2250.0 + (i as f64 % 10.0) - 5.0;
            prices.push(price);
            quantities.push(1.0);
            is_buyer.push(i % 2 == 0);
            timestamps.push(i as u64 * 1_000_000_000);
        }

        let ticks = TickArrays {
            len: prices.len(),
            prices,
            quantities,
            is_buyer,
            timestamps,
        };

        let result = backtest.run(&ticks);

        // Should complete without errors
        assert!(result.elapsed_seconds > 0.0);
        assert!(result.ticks_per_second > 0.0);
    }

    #[test]
    fn test_performance_stats_no_trades() {
        let positions: Vec<Position> = vec![];
        let stats = PerformanceStats::calculate(&positions);

        assert_eq!(stats.total_trades, 0);
        assert_eq!(stats.winning_trades, 0);
        assert_eq!(stats.losing_trades, 0);
        assert_eq!(stats.win_rate, 0.0);
        assert_eq!(stats.total_pnl, 0.0);
        assert_eq!(stats.profit_factor, 0.0);
    }

    #[test]
    fn test_performance_stats_all_wins() {
        use crate::vectorized::positions::PositionSide;

        let positions = vec![
            Position {
                side: PositionSide::Long,
                entry_price: 2250.0,
                entry_index: 0,
                exit_price: Some(2260.0),
                exit_index: Some(10),
                stop_loss: None,
                take_profit: None,
                pnl: 10.0,
                pnl_pct: 0.44,
            },
            Position {
                side: PositionSide::Long,
                entry_price: 2260.0,
                entry_index: 11,
                exit_price: Some(2270.0),
                exit_index: Some(20),
                stop_loss: None,
                take_profit: None,
                pnl: 10.0,
                pnl_pct: 0.44,
            },
        ];

        let stats = PerformanceStats::calculate(&positions);

        assert_eq!(stats.total_trades, 2);
        assert_eq!(stats.winning_trades, 2);
        assert_eq!(stats.losing_trades, 0);
        assert_eq!(stats.win_rate, 100.0);
        assert_eq!(stats.total_pnl, 20.0);
    }
}
