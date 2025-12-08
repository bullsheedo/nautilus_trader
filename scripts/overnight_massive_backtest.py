#!/usr/bin/env python3
"""
Overnight Massive Backtest: 10M+ ticks from Parquet catalog
Uses NautilusTrader's ParquetDataCatalog to load data efficiently
"""

import sys
from pathlib import Path
import pandas as pd
import json
from datetime import datetime
import time

# Add nautilus_trader to path
NAUTILUS_PATH = Path("/root/nautilus_trader")
sys.path.insert(0, str(NAUTILUS_PATH))

from nautilus_trader.persistence.catalog import ParquetDataCatalog
from nautilus_trader.model.data import TradeTick
from nautilus_trader.model.identifiers import InstrumentId
from nautilus_trader.test_kit.providers import TestInstrumentProvider

# Import vectorized backtest
from nautilus_backtest import BacktestConfig, run_vectorized_backtest

# Paths
CATALOG_PATH = Path("/root/nautilus_trader/catalog")
RESULTS_JSON = Path("/root/nautilus_trader/backtest_results_vectorized_parallel.json")
OUTPUT_DIR = Path("/root/nautilus_trader/overnight_results")
LOG_FILE = Path("/root/nautilus_trader/overnight_massive_backtest.log")

OUTPUT_DIR.mkdir(exist_ok=True)


def log(message):
    """Log message to both console and file."""
    timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
    log_msg = f"[{timestamp}] {message}"
    print(log_msg)
    with open(LOG_FILE, "a") as f:
        f.write(log_msg + "\n")


def load_ticks_from_catalog(start_date, end_date, max_ticks=None):
    """Load TradeTicks from parquet catalog."""
    log(f"Loading ticks from catalog: {start_date} to {end_date}")
    log(f"  Max ticks: {max_ticks if max_ticks else 'unlimited'}")
    
    catalog = ParquetDataCatalog(str(CATALOG_PATH))
    
    # Load trade ticks
    ticks = catalog.trade_ticks(
        instrument_ids=["ETHUSDT-PERP.BINANCE"],
        start=pd.Timestamp(start_date, tz='UTC'),
        end=pd.Timestamp(end_date, tz='UTC'),
    )
    
    if max_ticks and len(ticks) > max_ticks:
        log(f"  Limiting to {max_ticks:,} ticks (loaded {len(ticks):,})")
        ticks = ticks[:max_ticks]
    
    log(f"✓ Loaded {len(ticks):,} ticks")
    return ticks


def run_vectorized_on_ticks(ticks, config_params, test_name):
    """Run vectorized backtest on loaded ticks."""
    log(f"\n{'='*80}")
    log(f"Running vectorized backtest: {test_name}")
    log(f"{'='*80}")
    log(f"Ticks: {len(ticks):,}")
    log(f"Config: POI={config_params['poi_tolerance']}, TP={config_params['take_profit']}, SL={config_params['stop_loss']}")
    
    start_time = time.time()
    
    # Extract prices and timestamps
    prices = [float(tick.price) for tick in ticks]
    timestamps = [tick.ts_event for tick in ticks]
    
    # Get price range
    price_min = min(prices)
    price_max = max(prices)
    
    log(f"Price range: ${price_min:.2f} - ${price_max:.2f}")
    
    # Create config
    config = BacktestConfig(
        vwap_window=1000,
        volume_profile_window=1000,
        footprint_window=100,
        ib_period_minutes=60,
        imbalance_min_stack=3,
        imbalance_ratio=1.5,
        poi_tolerance=config_params['poi_tolerance'],
        tick_size=0.01,
        take_profit_ticks=config_params['take_profit'],
        stop_loss_ticks=config_params['stop_loss'],
        trailing_stop_ticks=0.20,
        warmup_ticks=1000,
        price_range_min=price_min,
        price_range_max=price_max,
    )
    
    # Run backtest
    result = run_vectorized_backtest(prices, timestamps, config)
    
    elapsed = time.time() - start_time
    
    log(f"\n✅ Backtest complete!")
    log(f"  Runtime: {elapsed/60:.2f} minutes")
    log(f"  Total trades: {result.total_trades}")
    log(f"  Win rate: {result.win_rate:.2f}%")
    log(f"  Total PnL: ${result.total_pnl:.2f}")
    log(f"  Profit factor: {result.profit_factor:.2f}")
    log(f"  Max drawdown: ${result.max_drawdown:.2f}")
    
    return {
        "test_name": test_name,
        "num_ticks": len(ticks),
        "config": config_params,
        "results": {
            "total_trades": result.total_trades,
            "winning_trades": result.winning_trades,
            "losing_trades": result.losing_trades,
            "win_rate": result.win_rate,
            "total_pnl": result.total_pnl,
            "profit_factor": result.profit_factor,
            "max_drawdown": result.max_drawdown,
            "sharpe_ratio": result.sharpe_ratio,
        },
        "runtime_minutes": elapsed / 60,
    }


def main():
    """Run overnight massive backtest suite."""
    log("🌙 OVERNIGHT MASSIVE BACKTEST STARTED 🌙")
    log(f"Start time: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
    log("")
    
    overall_start = time.time()
    
    # Load best config from previous results
    with open(RESULTS_JSON) as f:
        data = json.load(f)
    best = max(data, key=lambda x: x.get('profit_factor', 0))
    
    config_params = {
        'poi_tolerance': best['poi_tolerance'],
        'take_profit': best['take_profit'],
        'stop_loss': best['stop_loss'],
    }
    
    log(f"Using best config: POI={config_params['poi_tolerance']}, TP={config_params['take_profit']}, SL={config_params['stop_loss']}")
    
    all_results = []
    
    # Test 1: 10M ticks (full dataset)
    ticks_10m = load_ticks_from_catalog('2025-03-01', '2025-05-11', max_ticks=10_000_000)
    result_10m = run_vectorized_on_ticks(ticks_10m, config_params, "10M_ticks_full")
    all_results.append(result_10m)
    
    # Save results
    output_file = OUTPUT_DIR / "massive_backtest_results.json"
    with open(output_file, "w") as f:
        json.dump({
            "test_date": datetime.now().isoformat(),
            "best_config": config_params,
            "results": all_results,
            "total_runtime_hours": (time.time() - overall_start) / 3600,
        }, f, indent=2)
    
    log(f"\n{'='*80}")
    log("🎉 OVERNIGHT MASSIVE BACKTEST COMPLETED 🎉")
    log(f"Total runtime: {(time.time() - overall_start)/3600:.2f} hours")
    log(f"Results saved to: {output_file}")
    log(f"{'='*80}")


if __name__ == "__main__":
    main()

