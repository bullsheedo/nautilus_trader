#!/usr/bin/env python3
"""
Run multiple vectorized backtests to simulate parameter optimization.
This demonstrates the speedup for running 81 backtests.
"""

import time
import pandas as pd
import itertools
from pathlib import Path

from nautilus_trader.core.nautilus_pyo3.backtest import VectorizedBacktest, BacktestConfig
from nautilus_trader.model.data import TradeTick
from nautilus_trader.model.identifiers import InstrumentId, TradeId
from nautilus_trader.model.objects import Price, Quantity
from nautilus_trader.model.enums import AggressorSide
from nautilus_trader.test_kit.providers import TestInstrumentProvider


def load_csv_data(csv_path: str, max_rows: int = None) -> pd.DataFrame:
    """Load CSV data."""
    df = pd.read_csv(csv_path, nrows=max_rows)
    return df


def csv_to_trade_ticks(df: pd.DataFrame, instrument_id_str: str, price_precision: int, size_precision: int) -> list[TradeTick]:
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


def main():
    print("=" * 80)
    print("Vectorized Backtest Optimization Suite")
    print("=" * 80)
    print()
    
    # Configuration
    CSV_FILE = "/root/ethusdt_sample.csv"
    INSTRUMENT_ID = "ETHUSDT-PERP.BINANCE"
    NUM_TICKS = 100_000  # Use 100K for faster testing
    
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
    
    print(f"✓ Loaded {len(ticks):,} ticks")
    print()
    
    # Define parameter grid (3x3x3 = 27 combinations, we'll run 27 to simulate 81)
    poi_tolerances = [2.0, 3.0, 4.0]
    take_profit_ticks = [0.25, 0.30, 0.35]
    stop_loss_ticks = [0.30, 0.35, 0.40]
    
    param_combinations = list(itertools.product(poi_tolerances, take_profit_ticks, stop_loss_ticks))
    
    print(f"Running {len(param_combinations)} backtests...")
    print(f"Parameters: POI tolerance x TP ticks x SL ticks")
    print()
    
    results = []
    start_time = time.time()
    
    for i, (poi_tol, tp_ticks, sl_ticks) in enumerate(param_combinations, 1):
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
            price_range_min=min_price - 100.0,
            price_range_max=max_price + 100.0,
        )
        
        backtest = VectorizedBacktest(config)
        result = backtest.run(ticks)
        
        results.append({
            'poi_tolerance': poi_tol,
            'take_profit': tp_ticks,
            'stop_loss': sl_ticks,
            'total_trades': result.stats.total_trades,
            'win_rate': result.stats.win_rate,
            'total_pnl': result.stats.total_pnl,
            'profit_factor': result.stats.profit_factor,
            'max_drawdown': result.stats.max_drawdown,
        })
        
        if i % 9 == 0:
            elapsed = time.time() - start_time
            print(f"  Completed {i}/{len(param_combinations)} backtests in {elapsed:.1f}s")
    
    total_time = time.time() - start_time
    
    print()
    print("=" * 80)
    print("Results")
    print("=" * 80)
    print()
    print(f"Total backtests: {len(param_combinations)}")
    print(f"Total time: {total_time:.2f}s ({total_time/60:.2f} minutes)")
    print(f"Average time per backtest: {total_time/len(param_combinations):.2f}s")
    print(f"Throughput: {len(param_combinations) * NUM_TICKS / total_time:,.0f} ticks/second")
    print()
    print(f"Estimated time for 81 backtests on 500K ticks:")
    print(f"  {(total_time / len(param_combinations)) * 81 * (500_000 / NUM_TICKS) / 60:.1f} minutes")
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


if __name__ == "__main__":
    main()

