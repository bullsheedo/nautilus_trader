#!/usr/bin/env python3
"""
Debug script to understand differences between vectorized and event-driven strategies.
Prints detailed information about POI detection and signal generation.
"""

import sys
from pathlib import Path
import pandas as pd

# Add nautilus_trader to path
NAUTILUS_PATH = Path("/root/nautilus_trader")
sys.path.insert(0, str(NAUTILUS_PATH))

from decimal import Decimal
from nautilus_trader.core.nautilus_pyo3.backtest import VectorizedBacktest, BacktestConfig
from nautilus_trader.backtest.engine import BacktestEngine, BacktestEngineConfig
from nautilus_trader.config import LoggingConfig
from nautilus_trader.model.data import TradeTick
from nautilus_trader.model.identifiers import InstrumentId, TradeId, TraderId, Venue
from nautilus_trader.model.objects import Price, Quantity, Money
from nautilus_trader.model.enums import AggressorSide, OmsType, AccountType
from nautilus_trader.model.currencies import USDT
from nautilus_trader.test_kit.providers import TestInstrumentProvider
from nautilus_trader.examples.strategies.orderflow_strategy import (
    OrderFlowStrategy,
    OrderFlowStrategyConfig,
)

# Configuration
CSV_FILE = Path("/root/ethusdt_sample.csv")
INSTRUMENT_ID = "ETHUSDT-PERP.BINANCE"
NUM_TICKS = 10_000


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


def main():
    print("=" * 80)
    print("Strategy Differences Debug")
    print("=" * 80)
    print()
    
    # Load data
    print(f"Loading {NUM_TICKS:,} ticks from CSV...")
    df = load_csv_data(CSV_FILE, NUM_TICKS)
    print(f"✓ Loaded {len(df):,} ticks")
    print(f"  Price range: {df['price'].min():.2f} - {df['price'].max():.2f}")
    print()
    
    # Convert to TradeTick objects
    ticks = csv_to_trade_ticks(df, INSTRUMENT_ID, price_precision=2, size_precision=8)
    price_range = (df['price'].min(), df['price'].max())
    
    # Test parameters
    params = {
        'poi_tolerance': 3.0,
        'take_profit': 0.30,
        'stop_loss': 0.35,
    }
    
    print("Test parameters:")
    print(f"  POI tolerance: {params['poi_tolerance']}")
    print(f"  Take profit: {params['take_profit']}")
    print(f"  Stop loss: {params['stop_loss']}")
    print()
    
    # Run vectorized backtest
    print("=" * 80)
    print("Running Vectorized Backtest")
    print("=" * 80)
    print()
    
    config = BacktestConfig(
        vwap_window=1000,
        volume_profile_window=1000,
        footprint_window=100,
        ib_period_minutes=60,
        imbalance_min_stack=3,
        imbalance_ratio=1.5,
        poi_tolerance=params['poi_tolerance'],
        tick_size=0.01,
        take_profit_ticks=params['take_profit'],
        stop_loss_ticks=params['stop_loss'],
        trailing_stop_ticks=0.20,
        warmup_ticks=1000,
        price_range_min=price_range[0],
        price_range_max=price_range[1],
    )
    
    backtest = VectorizedBacktest(config)
    result = backtest.run(ticks)
    
    print(f"Vectorized Results:")
    print(f"  Total trades: {result.stats.total_trades}")
    print(f"  Win rate: {result.stats.win_rate:.2f}%")
    print(f"  Total PnL: {result.stats.total_pnl:.2f}")
    print()
    
    print("✓ Debug complete!")


if __name__ == "__main__":
    main()

