#!/usr/bin/env python3
"""Profile a single backtest to find performance bottlenecks."""

import pandas as pd
from pathlib import Path
from decimal import Decimal
from datetime import datetime
from dateutil import parser as date_parser

from nautilus_trader.backtest.engine import BacktestEngine, BacktestEngineConfig
from nautilus_trader.config import LoggingConfig
from nautilus_trader.model.data import TradeTick
from nautilus_trader.model.enums import AccountType, OmsType, AggressorSide
from nautilus_trader.model.identifiers import InstrumentId, TradeId, Venue
from nautilus_trader.model.objects import Money, Price, Quantity, Currency
from nautilus_trader.test_kit.providers import TestInstrumentProvider

from nautilus_trader.examples.strategies.orderflow_strategy import (
    OrderFlowStrategy,
    OrderFlowStrategyConfig,
)

# Constants
CSV_FILE = "/root/ethusdt_sample.csv"
INSTRUMENT_ID = "ETHUSDT-PERP.BINANCE"
USDT = Currency.from_str("USDT")


def load_csv_data(csv_path, max_rows=None):
    """Load trade data from CSV file."""
    print(f"Loading CSV: {csv_path}")
    df = pd.read_csv(csv_path, nrows=max_rows)
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

        aggressor_side = AggressorSide.SELLER if row['buyer_maker'] == True else AggressorSide.BUYER

        # Parse timestamp string to nanoseconds
        dt = date_parser.parse(row['timestamp'])
        ts_event = int(dt.timestamp() * 1_000_000_000)

        tick = TradeTick(
            instrument_id=instrument_id_obj,
            price=price,
            size=size,
            aggressor_side=aggressor_side,
            trade_id=TradeId(str(int(row['trade_id']))),
            ts_event=ts_event,
            ts_init=ts_event,
        )
        ticks.append(tick)

        if (idx + 1) % 50000 == 0:
            print(f"  Converted {idx + 1:,} ticks...")

    print(f"✓ Converted {len(ticks):,} ticks")
    return ticks


def run_single_backtest():
    """Run a single backtest with profiling."""
    
    # Load data (use smaller subset for faster profiling)
    print("\n" + "="*80)
    print("Profiling Single Backtest")
    print("="*80 + "\n")
    
    df = load_csv_data(CSV_FILE, max_rows=100_000)  # 100K ticks for faster profiling

    # Create instrument first to get precision
    instrument = TestInstrumentProvider.ethusdt_perp_binance()
    tick_size = float(instrument.price_increment)
    price_precision = instrument.price_precision
    size_precision = instrument.size_precision

    ticks = csv_to_trade_ticks(df, INSTRUMENT_ID, price_precision=price_precision, size_precision=size_precision)
    
    # Create backtest engine
    print("\nCreating backtest engine...")
    config = BacktestEngineConfig(
        logging=LoggingConfig(log_level="ERROR"),
    )
    engine = BacktestEngine(config=config)
    
    # Add venue
    engine.add_venue(
        venue=Venue("BINANCE"),
        oms_type=OmsType.NETTING,
        account_type=AccountType.MARGIN,
        base_currency=None,
        starting_balances=[Money(100_000, USDT)],
    )
    
    # Add instrument
    engine.add_instrument(instrument)
    
    # Create strategy
    print("Creating strategy...")
    strategy_config = OrderFlowStrategyConfig(
        instrument_id=instrument.id,
        tick_size=tick_size,
        trade_size=Decimal("10.0"),
        poi_tolerance=5.0,
        warmup_ticks=1000,
        tp_pct=0.3,
        sl_pct=0.3,
        trailing_activation_pct=0.25,
        trailing_offset_pct=0.10,
        use_emulated_orders=True,
    )
    
    strategy = OrderFlowStrategy(config=strategy_config)
    engine.add_strategy(strategy)
    
    # Add data
    print("Adding data...")
    engine.add_data(ticks)
    
    # Run backtest
    print(f"\nRunning backtest with {len(ticks):,} ticks...")
    start_time = datetime.now()
    engine.run()
    elapsed = (datetime.now() - start_time).total_seconds()
    
    print(f"\n{'='*80}")
    print(f"✓ Backtest completed in {elapsed:.2f} seconds")
    print(f"✓ Processing speed: {len(ticks)/elapsed:,.0f} ticks/second")
    print(f"{'='*80}\n")


if __name__ == "__main__":
    run_single_backtest()

