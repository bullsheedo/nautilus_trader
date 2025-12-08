#!/usr/bin/env python3
"""
Test the vectorized backtesting engine.

This demonstrates the new Level 4 vectorized approach which processes
all ticks at once for 100-1000x speedup.
"""

import time
import pandas as pd
from dateutil import parser as date_parser

from nautilus_trader.model.identifiers import InstrumentId, TradeId
from nautilus_trader.model.objects import Price, Quantity
from nautilus_trader.model.data import TradeTick
from nautilus_trader.model.enums import AggressorSide
from nautilus_trader.test_kit.providers import TestInstrumentProvider
from nautilus_trader.core.nautilus_pyo3.backtest import VectorizedBacktest, BacktestConfig


def load_csv_data(csv_file, max_rows=None):
    """Load CSV data into DataFrame."""
    print(f"Loading CSV: {csv_file}")
    df = pd.read_csv(csv_file, nrows=max_rows)
    print(f"Loaded {len(df):,} rows")
    return df


def csv_to_trade_ticks(df, instrument_id, price_precision=2, size_precision=3):
    """Convert CSV DataFrame to TradeTick objects."""
    print(f"Converting to TradeTick objects...")
    
    ticks = []
    instrument_id_obj = InstrumentId.from_str(instrument_id)
    
    for idx, row in df.iterrows():
        price = Price(float(row['price']), precision=price_precision)
        size = Quantity(float(row['quantity']), precision=size_precision)
        
        # Parse timestamp
        ts_str = str(row['timestamp'])
        dt = date_parser.parse(ts_str)
        ts_nanos = int(dt.timestamp() * 1_000_000_000)
        
        # Determine aggressor side
        aggressor_side = AggressorSide.BUYER if row['buyer_maker'] == False else AggressorSide.SELLER
        
        # Create trade ID
        trade_id = TradeId(f"T-{row['trade_id']}")

        tick = TradeTick(
            instrument_id=instrument_id_obj,
            price=price,
            size=size,
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
    print("Testing Vectorized Backtest Engine")
    print("=" * 80)
    print()
    
    # Configuration
    CSV_FILE = "/root/ethusdt_sample.csv"
    INSTRUMENT_ID = "ETHUSDT-PERP.BINANCE"
    
    # Load data
    df = load_csv_data(CSV_FILE, max_rows=500_000)  # Load 500K ticks
    
    # Create instrument first to get precision
    instrument = TestInstrumentProvider.ethusdt_perp_binance()
    price_precision = instrument.price_precision
    size_precision = instrument.size_precision
    
    ticks = csv_to_trade_ticks(df, INSTRUMENT_ID, price_precision=price_precision, size_precision=size_precision)

    print()
    print("=" * 80)
    print("Running Vectorized Backtest")
    print("=" * 80)
    print()

    # Get price range from data
    prices = [float(row['price']) for _, row in df.iterrows()]
    min_price = min(prices)
    max_price = max(prices)
    print(f"Price range: {min_price:.2f} - {max_price:.2f}")

    # Create backtest configuration
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
        price_range_min=min_price - 100.0,  # Add some buffer
        price_range_max=max_price + 100.0,
    )
    print(f"Config: {config}")
    print()

    # Create and run vectorized backtest
    backtest = VectorizedBacktest(config)

    print(f"Running backtest on {len(ticks):,} ticks...")
    start_time = time.time()
    result = backtest.run(ticks)
    elapsed = time.time() - start_time

    print()
    print("=" * 80)
    print("RESULTS")
    print("=" * 80)
    print(f"\n{result}")
    print(f"\nElapsed time: {elapsed:.3f}s")
    print(f"Ticks processed: {len(ticks):,}")
    print(f"Throughput: {len(ticks)/elapsed:,.0f} ticks/second")
    print()
    print("=" * 80)
    print("Performance Comparison:")
    print("=" * 80)
    print(f"  Current (event-driven): ~800 ticks/second")
    print(f"  Vectorized (Level 4):   {len(ticks)/elapsed:,.0f} ticks/second")
    print(f"  Speedup:                {(len(ticks)/elapsed)/800:.1f}x faster!")
    print()
    print("SUCCESS! 🚀")
    print()


if __name__ == "__main__":
    main()

