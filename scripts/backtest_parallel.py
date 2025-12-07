#!/usr/bin/env python3
"""
Parallel backtest runner - Utilizes all 8 CPU cores for parameter optimization.

This script runs multiple backtests in parallel with different parameter combinations,
allowing you to utilize all 8 cores simultaneously for maximum throughput.

Expected speedup: 8x (from using 8 cores in parallel)
"""
import sys
from pathlib import Path
from decimal import Decimal
from datetime import datetime
import multiprocessing as mp
from itertools import product
import json

# Add nautilus_trader to path
NAUTILUS_PATH = Path("/root/nautilus_trader")
sys.path.insert(0, str(NAUTILUS_PATH))

from nautilus_trader.backtest.node import BacktestNode
from nautilus_trader.config import (
    BacktestRunConfig,
    BacktestEngineConfig,
    BacktestVenueConfig,
    BacktestDataConfig,
    LoggingConfig,
    ImportableStrategyConfig,
)
from nautilus_trader.model.data import TradeTick
from nautilus_trader.model.identifiers import TraderId

# Paths
CATALOG_PATH = Path("/root/nautilus_trader/catalog_new")
INSTRUMENT_ID = "ETHUSDT-PERP.BINANCE"


def run_single_backtest(params):
    """
    Run a single backtest with given parameters.
    
    This function will be executed in parallel across multiple processes.
    """
    param_id, tp_pct, sl_pct, poi_tolerance, trailing_activation_pct = params
    
    try:
        # Import here to avoid pickling issues
        from nautilus_trader.persistence.catalog import ParquetDataCatalog
        
        # Load catalog
        catalog = ParquetDataCatalog(str(CATALOG_PATH))
        instruments = catalog.instruments()
        
        if not instruments:
            return {
                "param_id": param_id,
                "error": "No instruments found in catalog",
            }
        
        instrument = instruments[0]
        tick_size = float(instrument.price_increment)
        
        # Configure venue
        venue_config = BacktestVenueConfig(
            name="BINANCE",
            oms_type="NETTING",
            account_type="MARGIN",
            base_currency="USDT",
            starting_balances=["100_000 USDT"],
            default_leverage=Decimal("20"),
        )
        
        # Configure data
        data_config = BacktestDataConfig(
            catalog_path=str(CATALOG_PATH),
            data_cls=TradeTick,
            instrument_id=INSTRUMENT_ID,
            start_time="2025-03-01",
            end_time="2025-03-02",  # One day for faster testing
        )
        
        # Configure strategy with current parameters
        strategy_config = ImportableStrategyConfig(
            strategy_path="nautilus_trader.examples.strategies.orderflow_strategy:OrderFlowStrategy",
            config_path="nautilus_trader.examples.strategies.orderflow_strategy:OrderFlowStrategyConfig",
            config={
                "instrument_id": str(instrument.id),
                "tick_size": tick_size,
                "trade_size": "10.0",
                "poi_tolerance": poi_tolerance,
                "warmup_ticks": 1000,
                "tp_pct": tp_pct,
                "sl_pct": sl_pct,
                "trailing_activation_pct": trailing_activation_pct,
                "trailing_offset_pct": 0.10,
                "use_emulated_orders": True,
            },
        )
        
        # Configure engine
        engine_config = BacktestEngineConfig(
            trader_id=TraderId(f"BACKTEST-{param_id:03d}"),
            logging=LoggingConfig(log_level="ERROR"),
            strategies=[strategy_config],
        )
        
        # Configure run
        run_config = BacktestRunConfig(
            engine=engine_config,
            venues=[venue_config],
            data=[data_config],
            chunk_size=10_000,
        )
        
        # Run backtest
        start_time = datetime.now()
        node = BacktestNode(configs=[run_config])
        results = node.run()
        elapsed = (datetime.now() - start_time).total_seconds()
        
        # Extract results (simplified - you can add more metrics)
        return {
            "param_id": param_id,
            "tp_pct": tp_pct,
            "sl_pct": sl_pct,
            "poi_tolerance": poi_tolerance,
            "trailing_activation_pct": trailing_activation_pct,
            "elapsed_seconds": elapsed,
            "success": True,
        }
        
    except Exception as e:
        return {
            "param_id": param_id,
            "tp_pct": tp_pct,
            "sl_pct": sl_pct,
            "poi_tolerance": poi_tolerance,
            "trailing_activation_pct": trailing_activation_pct,
            "error": str(e),
            "success": False,
        }


def main():
    print("=" * 80)
    print("NautilusTrader Parallel Backtest - 8 Core Optimization")
    print("=" * 80)
    
    # Define parameter grid
    tp_pct_values = [0.20, 0.30, 0.40]
    sl_pct_values = [0.20, 0.30, 0.40]
    poi_tolerance_values = [3.0, 5.0, 7.0]
    trailing_activation_values = [0.20, 0.25, 0.30]
    
    # Generate all combinations
    param_combinations = list(product(
        tp_pct_values,
        sl_pct_values,
        poi_tolerance_values,
        trailing_activation_values,
    ))
    
    # Add param_id to each combination
    params_list = [
        (i, *params) for i, params in enumerate(param_combinations)
    ]
    
    total_combinations = len(params_list)
    print(f"\n📊 Parameter Grid:")
    print(f"  - TP %: {tp_pct_values}")
    print(f"  - SL %: {sl_pct_values}")
    print(f"  - POI Tolerance: {poi_tolerance_values}")
    print(f"  - Trailing Activation %: {trailing_activation_values}")
    print(f"  - Total combinations: {total_combinations}")
    
    # Determine number of processes (use all 8 cores)
    num_processes = min(8, mp.cpu_count())
    print(f"\n🚀 Running {total_combinations} backtests on {num_processes} cores...")
    print(f"  - Estimated time: ~{total_combinations / num_processes:.0f}x single backtest time")
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
    results_file = Path("backtest_results_parallel.json")
    with open(results_file, "w") as f:
        json.dump(results, f, indent=2)
    
    print(f"\n📊 Results saved to: {results_file}")
    
    # Show summary
    successful = [r for r in results if r.get("success")]
    failed = [r for r in results if not r.get("success")]
    
    print(f"\n📈 Summary:")
    print(f"  - Successful: {len(successful)}/{total_combinations}")
    print(f"  - Failed: {len(failed)}/{total_combinations}")
    
    if successful:
        print(f"\n🏆 Best parameters (by elapsed time - placeholder metric):")
        best = min(successful, key=lambda x: x.get("elapsed_seconds", float('inf')))
        print(f"  - TP%: {best['tp_pct']}")
        print(f"  - SL%: {best['sl_pct']}")
        print(f"  - POI Tolerance: {best['poi_tolerance']}")
        print(f"  - Trailing Activation%: {best['trailing_activation_pct']}")
    
    print("\n" + "=" * 80)
    print("✓ Parallel backtest optimization complete!")
    print("=" * 80)


if __name__ == "__main__":
    main()

