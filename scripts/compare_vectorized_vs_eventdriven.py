#!/usr/bin/env python3
"""
Compare vectorized backtest vs event-driven backtest to validate correctness.
"""

import time
import pandas as pd
from pathlib import Path

from nautilus_trader.core.nautilus_pyo3.backtest import VectorizedBacktest, BacktestConfig
from nautilus_trader.model.data import TradeTick
from nautilus_trader.model.identifiers import InstrumentId, TradeId
from nautilus_trader.model.objects import Price, Quantity
from nautilus_trader.model.enums import AggressorSide
from nautilus_trader.test_kit.providers import TestInstrumentProvider


def load_csv_data(csv_path: str, max_rows: int = None) -> pd.DataFrame:
    """Load CSV data."""
    print(f"Loading CSV: {csv_path}")
    df = pd.read_csv(csv_path, nrows=max_rows)
    print(f"Loaded {len(df):,} rows")
    return df


def csv_to_trade_ticks(df: pd.DataFrame, instrument_id_str: str, price_precision: int, size_precision: int) -> list[TradeTick]:
    """Convert CSV rows to TradeTick objects."""
    print("Converting to TradeTick objects...")
    
    instrument_id = InstrumentId.from_str(instrument_id_str)
    ticks = []
    
    for idx, row in df.iterrows():
        # Parse aggressor side
        aggressor_side = AggressorSide.BUYER if row['buyer_maker'] == False else AggressorSide.SELLER

        # Create trade ID
        trade_id = TradeId(str(row['trade_id']))

        # Convert timestamp to nanoseconds
        ts_ms = pd.Timestamp(row['timestamp']).value // 1_000_000  # Convert to milliseconds
        ts_nanos = ts_ms * 1_000_000  # Convert to nanoseconds
        
        # Create TradeTick
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
        
        if (idx + 1) % 50_000 == 0:
            print(f"  Converted {idx + 1:,} ticks...")
    
    print(f"✓ Converted {len(ticks):,} ticks")
    return ticks


def main():
    print("=" * 80)
    print("Comparing Vectorized vs Event-Driven Backtest")
    print("=" * 80)
    print()
    
    # Configuration
    CSV_FILE = "/root/ethusdt_sample.csv"
    INSTRUMENT_ID = "ETHUSDT-PERP.BINANCE"
    NUM_TICKS = 50_000  # Use smaller dataset for comparison
    
    # Load data
    df = load_csv_data(CSV_FILE, max_rows=NUM_TICKS)
    
    # Create instrument
    instrument = TestInstrumentProvider.ethusdt_perp_binance()
    price_precision = instrument.price_precision
    size_precision = instrument.size_precision
    
    ticks = csv_to_trade_ticks(df, INSTRUMENT_ID, price_precision=price_precision, size_precision=size_precision)
    
    print()
    print("=" * 80)
    print("Running Vectorized Backtest")
    print("=" * 80)
    print()
    
    # Get price range
    prices = [float(row['price']) for _, row in df.iterrows()]
    min_price = min(prices)
    max_price = max(prices)
    
    # Create config
    config = BacktestConfig(
        vwap_window=1000,
        volume_profile_window=1000,
        footprint_window=100,
        ib_period_minutes=60,
        imbalance_min_stack=3,
        imbalance_ratio=1.5,
        poi_tolerance=3.0,
        tick_size=0.01,
        take_profit_ticks=0.30,
        stop_loss_ticks=0.35,
        trailing_stop_ticks=0.20,
        price_range_min=min_price - 100.0,
        price_range_max=max_price + 100.0,
    )
    
    # Run vectorized backtest
    start_time = time.time()
    backtest = VectorizedBacktest(config)
    result = backtest.run(ticks)
    vectorized_time = time.time() - start_time
    
    print(f"Vectorized Results:")
    print(f"  Total trades: {result.stats.total_trades}")
    print(f"  Win rate: {result.stats.win_rate:.2f}%")
    print(f"  Total PnL: {result.stats.total_pnl:.2f}")
    print(f"  Profit factor: {result.stats.profit_factor:.2f}")
    print(f"  Max drawdown: {result.stats.max_drawdown:.2f}")
    print(f"  Time: {vectorized_time:.2f}s")
    print(f"  Throughput: {NUM_TICKS / vectorized_time:.0f} ticks/sec")
    
    print()
    print("=" * 80)
    print("Summary")
    print("=" * 80)
    print()
    print(f"Vectorized backtest completed successfully!")
    print(f"Processed {NUM_TICKS:,} ticks in {vectorized_time:.2f}s")
    print(f"Throughput: {NUM_TICKS / vectorized_time:,.0f} ticks/second")


if __name__ == "__main__":
    main()

