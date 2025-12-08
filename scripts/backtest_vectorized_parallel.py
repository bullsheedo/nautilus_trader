#!/usr/bin/env python3
"""
Parallel vectorized backtest runner - Combines vectorized engine (20x) with multiprocessing (8x).

This script runs multiple vectorized backtests in parallel across all CPU cores,
achieving ~160x total speedup over the original event-driven approach.

Expected performance:
- Single vectorized backtest: 20x faster
- 8 cores in parallel: 8x faster
- Total speedup: ~160x faster
- 81 backtests: ~2 minutes (vs 5.5 hours originally)
"""

import sys
from pathlib import Path
import multiprocessing as mp
from itertools import product
import json
import pandas as pd
import time

# Add nautilus_trader to path
NAUTILUS_PATH = Path("/root/nautilus_trader")
sys.path.insert(0, str(NAUTILUS_PATH))

from nautilus_trader.core.nautilus_pyo3.backtest import VectorizedBacktest, BacktestConfig
from nautilus_trader.model.data import TradeTick
from nautilus_trader.model.identifiers import InstrumentId, TradeId
from nautilus_trader.model.objects import Price, Quantity
from nautilus_trader.model.enums import AggressorSide
from nautilus_trader.test_kit.providers import TestInstrumentProvider

# Configuration
CSV_FILE = Path("/root/ethusdt_sample.csv")
INSTRUMENT_ID = "ETHUSDT-PERP.BINANCE"
NUM_TICKS = 500_000  # Use full dataset


def load_csv_data(csv_path, max_rows=None):
    """Load trade data from CSV file."""
    df = pd.read_csv(csv_path, nrows=max_rows)
    return df


def csv_to_trade_ticks(df, instrument_id_str, price_precision, size_precision):
    """Convert CSV rows to TradeTick objects."""
    instrument_id = InstrumentId.from_str(instrument_id_str)
    ticks = []
    
    for idx, row in df.iterrows():
        aggressor_side = AggressorSide.BUYER if row['buyer_maker'] == False else AggressorSide.SELLER
        trade_id = TradeId(str(row['trade_id']))
        ts_ms = pd.Timestamp(row['timestamp']).value // 1_000_000
        ts_nanos = ts_ms * 1_000_000
        
        tick = TradeTick(
            instrument_id=instrument_id,
            price=Price(float(row['price']), precision=price_precision),
            size=Quantity(float(row['quantity']), precision=size_precision),
            aggressor_side=aggressor_side,
            trade_id=trade_id,
            ts_event=ts_nanos,
            ts_init=ts_nanos,
        )
        ticks.append(tick)
    
    return ticks


def run_single_backtest(args):
    """Run a single vectorized backtest with given parameters."""
    params, ticks, price_range = args
    
    poi_tol, tp_ticks, sl_ticks = params
    
    config = BacktestConfig(
        vwap_window=1000,
        volume_profile_window=1000,
        footprint_window=100,
        ib_period_minutes=60,
        imbalance_min_stack=3,
        imbalance_ratio=1.5,
        poi_tolerance=poi_tol,
        tick_size=0.01,
        take_profit_ticks=tp_ticks,
        stop_loss_ticks=sl_ticks,
        trailing_stop_ticks=0.20,
        price_range_min=price_range[0],
        price_range_max=price_range[1],
    )
    
    backtest = VectorizedBacktest(config)
    result = backtest.run(ticks)
    
    return {
        'poi_tolerance': poi_tol,
        'take_profit': tp_ticks,
        'stop_loss': sl_ticks,
        'total_trades': result.stats.total_trades,
        'winning_trades': result.stats.winning_trades,
        'losing_trades': result.stats.losing_trades,
        'win_rate': result.stats.win_rate,
        'total_pnl': result.stats.total_pnl,
        'avg_win': result.stats.avg_win,
        'avg_loss': result.stats.avg_loss,
        'profit_factor': result.stats.profit_factor,
        'sharpe_ratio': result.stats.sharpe_ratio,
        'max_drawdown': result.stats.max_drawdown,
        'elapsed_seconds': result.elapsed_seconds,
        'ticks_per_second': result.ticks_per_second,
    }


def main():
    print("=" * 80)
    print("Parallel Vectorized Backtest Optimization Suite")
    print("=" * 80)
    print()
    
    # Load data once
    print(f"Loading {NUM_TICKS:,} ticks from CSV...")
    df = load_csv_data(CSV_FILE, max_rows=NUM_TICKS)
    
    instrument = TestInstrumentProvider.ethusdt_perp_binance()
    ticks = csv_to_trade_ticks(df, INSTRUMENT_ID, 
                                price_precision=instrument.price_precision,
                                size_precision=instrument.size_precision)
    
    # Get price range
    prices = [float(row['price']) for _, row in df.iterrows()]
    min_price = min(prices)
    max_price = max(prices)
    price_range = (min_price - 100.0, max_price + 100.0)
    
    print(f"✓ Loaded {len(ticks):,} ticks")
    print(f"  Price range: {min_price:.2f} - {max_price:.2f}")
    print()
    
    # Define parameter grid (3x3x3 = 27 combinations for testing)
    # For full optimization, use the full grid
    poi_tolerances = [2.0, 3.0, 4.0]
    take_profit_ticks = [0.25, 0.30, 0.35]
    stop_loss_ticks = [0.30, 0.35, 0.40]
    
    param_combinations = list(product(poi_tolerances, take_profit_ticks, stop_loss_ticks))

    num_cores = mp.cpu_count()
    print(f"Running {len(param_combinations)} backtests across {num_cores} CPU cores...")
    print(f"Parameters: POI tolerance x TP ticks x SL ticks")
    print()

    # Prepare arguments for parallel processing
    args_list = [(params, ticks, price_range) for params in param_combinations]

    # Run backtests in parallel
    start_time = time.time()

    with mp.Pool(processes=num_cores) as pool:
        results = pool.map(run_single_backtest, args_list)

    total_time = time.time() - start_time

    print()
    print("=" * 80)
    print("Results")
    print("=" * 80)
    print()
    print(f"Total backtests: {len(param_combinations)}")
    print(f"Total time: {total_time:.2f}s ({total_time/60:.2f} minutes)")
    print(f"Average time per backtest: {total_time/len(param_combinations):.2f}s")
    print(f"Total throughput: {len(param_combinations) * NUM_TICKS / total_time:,.0f} ticks/second")
    print()

    # Calculate speedup
    original_time_per_backtest = 125  # seconds for 100K ticks event-driven
    original_time_scaled = original_time_per_backtest * (NUM_TICKS / 100_000)
    original_total_time = original_time_scaled * len(param_combinations)
    speedup = original_total_time / total_time

    print(f"Performance comparison:")
    print(f"  Original (event-driven, sequential): {original_total_time/3600:.2f} hours")
    print(f"  Vectorized + Parallel: {total_time/60:.2f} minutes")
    print(f"  Total speedup: {speedup:.1f}x faster!")
    print()

    # Estimate for 81 backtests
    time_per_backtest = total_time / len(param_combinations)
    estimated_81 = time_per_backtest * 81
    print(f"Estimated time for 81 backtests on {NUM_TICKS:,} ticks:")
    print(f"  {estimated_81/60:.1f} minutes ({estimated_81:.1f} seconds)")
    print()

    # Show best result
    best = max(results, key=lambda x: x['total_pnl'])
    print("Best configuration:")
    print(f"  POI tolerance: {best['poi_tolerance']}")
    print(f"  Take profit: {best['take_profit']}")
    print(f"  Stop loss: {best['stop_loss']}")
    print(f"  Total PnL: {best['total_pnl']:.2f}")
    print(f"  Win rate: {best['win_rate']:.2f}%")
    print(f"  Trades: {best['total_trades']}")
    print(f"  Profit factor: {best['profit_factor']:.2f}")
    print()

    # Save results to JSON
    output_file = Path("backtest_results_vectorized_parallel.json")
    with open(output_file, 'w') as f:
        json.dump(results, f, indent=2)

    print(f"✓ Results saved to {output_file}")
    print()
    print("SUCCESS! 🚀")


if __name__ == "__main__":
    main()

