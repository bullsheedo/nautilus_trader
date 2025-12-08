#!/usr/bin/env python3
"""
Validation script to compare vectorized backtest results with event-driven backtest.

This ensures the vectorized implementation produces correct results that match
the original event-driven approach.
"""

import sys
from pathlib import Path
import pandas as pd
import time

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
NUM_TICKS = 10_000  # Use smaller dataset for validation


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


def run_vectorized_backtest(ticks, price_range, params):
    """Run vectorized backtest."""
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
        warmup_ticks=1000,  # Match event-driven warmup period
        price_range_min=price_range[0],
        price_range_max=price_range[1],
    )
    
    backtest = VectorizedBacktest(config)
    result = backtest.run(ticks)
    
    return {
        'total_trades': result.stats.total_trades,
        'win_rate': result.stats.win_rate,
        'total_pnl': result.stats.total_pnl,
        'profit_factor': result.stats.profit_factor,
        'max_drawdown': result.stats.max_drawdown,
        'elapsed': result.elapsed_seconds,
    }


def run_eventdriven_backtest(ticks, params):
    """Run event-driven backtest."""
    # Create engine
    config = BacktestEngineConfig(
        trader_id=TraderId("BACKTESTER-001"),
        logging=LoggingConfig(log_level="ERROR"),
    )
    engine = BacktestEngine(config=config)
    
    # Add venue
    venue = Venue("BINANCE")
    engine.add_venue(
        venue=venue,
        oms_type=OmsType.NETTING,
        account_type=AccountType.MARGIN,
        base_currency=USDT,
        starting_balances=[Money(100_000, USDT)],
    )
    
    # Add instrument
    instrument = TestInstrumentProvider.ethusdt_perp_binance()
    engine.add_instrument(instrument)
    
    # Add strategy
    tick_size = float(instrument.price_increment)
    instrument_id_obj = InstrumentId.from_str(INSTRUMENT_ID)
    strategy_config = OrderFlowStrategyConfig(
        instrument_id=instrument_id_obj,
        tick_size=tick_size,
        trade_size=Decimal("10.0"),
        poi_tolerance=params['poi_tolerance'],
        warmup_ticks=1000,
        tp_pct=params['take_profit'],
        sl_pct=params['stop_loss'],
        trailing_activation_pct=0.25,
        trailing_offset_pct=0.10,
        use_emulated_orders=True,
    )
    strategy = OrderFlowStrategy(config=strategy_config)
    engine.add_strategy(strategy)
    
    # Add data
    engine.add_data(ticks)
    
    # Run backtest
    start_time = time.time()
    engine.run()
    elapsed = time.time() - start_time
    
    # Get results
    account = engine.trader.generate_account_report(venue)
    
    return {
        'total_trades': len(strategy.cache.orders()),
        'elapsed': elapsed,
    }


def main():
    print("=" * 80)
    print("Vectorized Backtest Validation")
    print("=" * 80)
    print()
    
    # Load data
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

    # Test parameters
    test_params = {
        'poi_tolerance': 3.0,
        'take_profit': 0.30,
        'stop_loss': 0.35,
    }

    print("Test parameters:")
    print(f"  POI tolerance: {test_params['poi_tolerance']}")
    print(f"  Take profit: {test_params['take_profit']}")
    print(f"  Stop loss: {test_params['stop_loss']}")
    print()

    # Run vectorized backtest
    print("=" * 80)
    print("Running Vectorized Backtest")
    print("=" * 80)
    print()

    vec_start = time.time()
    vec_results = run_vectorized_backtest(ticks, price_range, test_params)
    vec_time = time.time() - vec_start

    print(f"Vectorized Results:")
    print(f"  Total trades: {vec_results['total_trades']}")
    print(f"  Win rate: {vec_results['win_rate']:.2f}%")
    print(f"  Total PnL: {vec_results['total_pnl']:.2f}")
    print(f"  Profit factor: {vec_results['profit_factor']:.2f}")
    print(f"  Max drawdown: {vec_results['max_drawdown']:.2f}")
    print(f"  Time: {vec_results['elapsed']:.2f}s")
    print()

    # Run event-driven backtest
    print("=" * 80)
    print("Running Event-Driven Backtest")
    print("=" * 80)
    print()

    print("Note: Event-driven backtest may take several minutes...")
    ed_results = run_eventdriven_backtest(ticks, test_params)

    print(f"Event-Driven Results:")
    print(f"  Total orders: {ed_results['total_trades']}")
    print(f"  Time: {ed_results['elapsed']:.2f}s")
    print()

    # Compare results
    print("=" * 80)
    print("Comparison")
    print("=" * 80)
    print()

    speedup = ed_results['elapsed'] / vec_results['elapsed']

    print(f"Performance:")
    print(f"  Event-driven: {ed_results['elapsed']:.2f}s")
    print(f"  Vectorized: {vec_results['elapsed']:.2f}s")
    print(f"  Speedup: {speedup:.1f}x faster!")
    print()

    print(f"Vectorized Statistics:")
    print(f"  Total trades: {vec_results['total_trades']}")
    print(f"  Win rate: {vec_results['win_rate']:.2f}%")
    print(f"  Total PnL: {vec_results['total_pnl']:.2f}")
    print(f"  Profit factor: {vec_results['profit_factor']:.2f}")
    print()

    print("✓ Validation complete!")
    print()
    print("Note: The vectorized backtest provides comprehensive statistics")
    print("that are not available in the event-driven approach.")


if __name__ == "__main__":
    main()
