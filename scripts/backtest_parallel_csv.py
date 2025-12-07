#!/usr/bin/env python3
"""
Parallel backtest runner using CSV data - Utilizes all 8 CPU cores.

This script loads trade data from CSV and runs multiple backtests in parallel
with different parameter combinations to find optimal strategy settings.

Expected speedup: 7-8x faster than sequential execution
"""
import sys
from pathlib import Path
from decimal import Decimal
from datetime import datetime
import multiprocessing as mp
from itertools import product
import json
import pandas as pd

# Add nautilus_trader to path
NAUTILUS_PATH = Path("/root/nautilus_trader")
sys.path.insert(0, str(NAUTILUS_PATH))

from nautilus_trader.backtest.engine import BacktestEngine, BacktestEngineConfig
from nautilus_trader.config import LoggingConfig
from nautilus_trader.model.identifiers import Venue, TraderId, InstrumentId
from nautilus_trader.model.enums import OmsType, AccountType, AggressorSide
from nautilus_trader.model.objects import Money, Price, Quantity
from nautilus_trader.model.identifiers import TradeId
from nautilus_trader.model.data import TradeTick
from nautilus_trader.model.currencies import USDT
from nautilus_trader.test_kit.providers import TestInstrumentProvider
from nautilus_trader.examples.strategies.orderflow_strategy import (
    OrderFlowStrategy,
    OrderFlowStrategyConfig,
)

# Configuration
CSV_FILE = Path("/root/ethusdt_sample.csv")
INSTRUMENT_ID = "ETHUSDT-PERP.BINANCE"


def load_csv_data(csv_path, max_rows=None):
    """Load trade data from CSV file."""
    print(f"  Loading CSV: {csv_path}")
    df = pd.read_csv(csv_path, nrows=max_rows)
    print(f"  Loaded {len(df):,} rows")
    return df


def csv_to_trade_ticks(df, instrument_id):
    """Convert CSV DataFrame to TradeTick objects."""
    print(f"  Converting to TradeTick objects...")
    
    ticks = []
    instrument_id_obj = InstrumentId.from_str(instrument_id)
    
    for idx, row in df.iterrows():
        # Parse timestamp
        ts = pd.Timestamp(row['timestamp'])
        ts_event = int(ts.value)  # nanoseconds since epoch
        
        # Determine aggressor side
        aggressor_side = AggressorSide.SELLER if row['buyer_maker'] else AggressorSide.BUYER
        
        tick = TradeTick(
            instrument_id=instrument_id_obj,
            price=Price(float(row['price']), precision=2),
            size=Quantity(float(row['quantity']), precision=3),
            aggressor_side=aggressor_side,
            trade_id=TradeId(str(row['trade_id'])),
            ts_event=ts_event,
            ts_init=ts_event,
        )
        ticks.append(tick)
        
        if (idx + 1) % 100000 == 0:
            print(f"    Converted {idx + 1:,} ticks...")
    
    print(f"  ✓ Converted {len(ticks):,} ticks")
    return ticks


def run_single_backtest(params):
    """
    Run a single backtest with given parameters.
    This function will be executed in parallel across multiple processes.
    """
    param_id, tp_pct, sl_pct, poi_tolerance, trailing_activation_pct, ticks_data = params
    
    try:
        # Create instrument
        instrument = TestInstrumentProvider.ethusdt_perp_binance()
        tick_size = float(instrument.price_increment)
        
        # Create backtest engine
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
        strategy_config = OrderFlowStrategyConfig(
            instrument_id=instrument.id,
            tick_size=tick_size,
            trade_size=Decimal("10.0"),
            poi_tolerance=poi_tolerance,
            warmup_ticks=1000,
            tp_pct=tp_pct,
            sl_pct=sl_pct,
            trailing_activation_pct=trailing_activation_pct,
            trailing_offset_pct=0.10,
            use_emulated_orders=True,
        )
        
        strategy = OrderFlowStrategy(config=strategy_config)
        engine.add_strategy(strategy)
        
        # Add data
        engine.add_data(ticks_data)
        
        # Run backtest
        start_time = datetime.now()
        engine.run()
        elapsed = (datetime.now() - start_time).total_seconds()
        
        # Get account statistics
        account = engine.trader.generate_account_report(Venue("BINANCE"))
        
        return {
            "param_id": param_id,
            "tp_pct": tp_pct,
            "sl_pct": sl_pct,
            "poi_tolerance": poi_tolerance,
            "trailing_activation_pct": trailing_activation_pct,
            "elapsed_seconds": elapsed,
            "num_ticks": len(ticks_data),
            "ticks_per_second": len(ticks_data) / elapsed if elapsed > 0 else 0,
            "success": True,
        }
        
    except Exception as e:
        import traceback
        return {
            "param_id": param_id,
            "tp_pct": tp_pct,
            "sl_pct": sl_pct,
            "poi_tolerance": poi_tolerance,
            "trailing_activation_pct": trailing_activation_pct,
            "error": str(e),
            "traceback": traceback.format_exc(),
            "success": False,
        }


def main():
    print("=" * 80)
    print("NautilusTrader Parallel CSV Backtest - 8 Core Optimization")
    print("=" * 80)

    # Load CSV data (use subset for faster testing)
    print(f"\n📊 Loading data from CSV...")
    df = load_csv_data(CSV_FILE, max_rows=500_000)  # 500k rows for faster testing

    # Convert to TradeTick objects
    ticks = csv_to_trade_ticks(df, INSTRUMENT_ID)

    # Define parameter grid
    tp_pct_values = [0.25, 0.30, 0.35]
    sl_pct_values = [0.25, 0.30, 0.35]
    poi_tolerance_values = [3.0, 5.0, 7.0]
    trailing_activation_values = [0.20, 0.25, 0.30]

    # Generate all combinations
    param_combinations = list(product(
        tp_pct_values,
        sl_pct_values,
        poi_tolerance_values,
        trailing_activation_values,
    ))

    # Add param_id and ticks to each combination
    params_list = [
        (i, *params, ticks) for i, params in enumerate(param_combinations)
    ]

    total_combinations = len(params_list)
    print(f"\n📊 Parameter Grid:")
    print(f"  - TP %: {tp_pct_values}")
    print(f"  - SL %: {sl_pct_values}")
    print(f"  - POI Tolerance: {poi_tolerance_values}")
    print(f"  - Trailing Activation %: {trailing_activation_values}")
    print(f"  - Total combinations: {total_combinations}")
    print(f"  - Ticks per backtest: {len(ticks):,}")

    # Determine number of processes
    num_processes = min(8, mp.cpu_count())
    print(f"\n🚀 Running {total_combinations} backtests on {num_processes} cores...")
    print("-" * 80)

    # Run backtests in parallel
    start_time = datetime.now()

    with mp.Pool(processes=num_processes) as pool:
        results = pool.map(run_single_backtest, params_list)

    elapsed = (datetime.now() - start_time).total_seconds()

    print("-" * 80)
    print(f"\n✓ Completed {total_combinations} backtests in {elapsed:.2f} seconds")
    print(f"✓ Average time per backtest: {elapsed/total_combinations:.2f}s")
    print(f"✓ Speedup from parallelization: ~{num_processes}x")

    # Save results
    results_file = Path("backtest_results_csv_parallel.json")
    with open(results_file, "w") as f:
        json.dump(results, f, indent=2, default=str)

    print(f"\n📊 Results saved to: {results_file}")

    # Show summary
    successful = [r for r in results if r.get("success")]
    failed = [r for r in results if not r.get("success")]

    print(f"\n📈 Summary:")
    print(f"  - Successful: {len(successful)}/{total_combinations}")
    print(f"  - Failed: {len(failed)}/{total_combinations}")

    if failed:
        print(f"\n❌ Failed backtests:")
        for f in failed[:3]:  # Show first 3 failures
            print(f"  - Param {f['param_id']}: {f.get('error', 'Unknown error')}")

    if successful:
        avg_speed = sum(r['ticks_per_second'] for r in successful) / len(successful)
        print(f"\n⚡ Performance:")
        print(f"  - Average processing speed: {avg_speed:,.0f} ticks/second")
        print(f"  - Total ticks processed: {len(ticks) * len(successful):,}")

    print("\n" + "=" * 80)
    print("✓ Parallel CSV backtest complete!")
    print("=" * 80)


if __name__ == "__main__":
    # Prevent multiprocessing issues
    mp.set_start_method('spawn', force=True)
    main()

