#!/usr/bin/env python3
"""
Massive Parameter Sweep: 640 configurations on 500K ticks
Overnight test to find the absolute best parameters
"""

import sys
from pathlib import Path
import pandas as pd
import json
from datetime import datetime
import time
import itertools
from concurrent.futures import ProcessPoolExecutor, as_completed

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
NUM_TICKS = 500_000
OUTPUT_FILE = Path("/root/nautilus_trader/massive_sweep_results.json")
LOG_FILE = Path("/root/nautilus_trader/massive_sweep.log")

# Parameter grid - 640 combinations!
PARAM_GRID = {
    'poi_tolerance': [1.5, 2.0, 2.5, 3.0, 3.5, 4.0, 4.5, 5.0, 5.5, 6.0],  # 10 values
    'take_profit': [0.15, 0.20, 0.25, 0.30, 0.35, 0.40, 0.45, 0.50],      # 8 values
    'stop_loss': [0.15, 0.20, 0.25, 0.30, 0.35, 0.40, 0.45, 0.50],        # 8 values
}


def log(message):
    """Log message to both console and file."""
    timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
    log_msg = f"[{timestamp}] {message}"
    print(log_msg)
    with open(LOG_FILE, "a") as f:
        f.write(log_msg + "\n")


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


def run_single_config(params):
    """Run backtest for a single parameter configuration."""
    config_id, poi_tol, tp, sl, ticks, price_range = params

    try:
        config = BacktestConfig(
            vwap_window=1000,
            volume_profile_window=1000,
            footprint_window=100,
            ib_period_minutes=60,
            imbalance_min_stack=3,
            imbalance_ratio=1.5,
            poi_tolerance=poi_tol,
            tick_size=0.01,
            take_profit_ticks=tp,
            stop_loss_ticks=sl,
            trailing_stop_ticks=0.20,
            warmup_ticks=1000,
            price_range_min=price_range[0],
            price_range_max=price_range[1],
        )

        backtest = VectorizedBacktest(config)
        result = backtest.run(ticks)

        return {
            'config_id': config_id,
            'poi_tolerance': poi_tol,
            'take_profit': tp,
            'stop_loss': sl,
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
        }
    except Exception as e:
        return {
            'config_id': config_id,
            'poi_tolerance': poi_tol,
            'take_profit': tp,
            'stop_loss': sl,
            'error': str(e),
        }


def main():
    """Run massive parameter sweep."""
    log("🔥 MASSIVE PARAMETER SWEEP STARTED 🔥")
    log(f"Start time: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
    log("")
    
    overall_start = time.time()
    
    # Load data once
    log(f"Loading {NUM_TICKS:,} ticks from CSV...")
    df = load_csv_data(CSV_FILE, max_rows=NUM_TICKS)
    
    instrument = TestInstrumentProvider.ethusdt_perp_binance()
    ticks = csv_to_trade_ticks(df, INSTRUMENT_ID, 
                                price_precision=instrument.price_precision,
                                size_precision=instrument.size_precision)
    
    # Get price range
    prices = [float(tick.price) for tick in ticks]
    price_range = (min(prices), max(prices))

    log(f"✓ Loaded {len(ticks):,} ticks")
    log(f"  Price range: ${price_range[0]:.2f} - ${price_range[1]:.2f}")
    log("")

    # Generate all parameter combinations
    combinations = list(itertools.product(
        PARAM_GRID['poi_tolerance'],
        PARAM_GRID['take_profit'],
        PARAM_GRID['stop_loss']
    ))

    total_configs = len(combinations)
    log(f"📊 Testing {total_configs} parameter combinations")
    log(f"  POI tolerance: {PARAM_GRID['poi_tolerance']}")
    log(f"  Take profit: {PARAM_GRID['take_profit']}")
    log(f"  Stop loss: {PARAM_GRID['stop_loss']}")
    log("")

    # Prepare tasks
    tasks = [
        (i+1, poi, tp, sl, ticks, price_range)
        for i, (poi, tp, sl) in enumerate(combinations)
    ]
    
    # Run in parallel
    results = []
    completed = 0
    
    log(f"🚀 Starting parallel execution with {8} workers...")
    log("")
    
    with ProcessPoolExecutor(max_workers=8) as executor:
        futures = {executor.submit(run_single_config, task): task for task in tasks}
        
        for future in as_completed(futures):
            result = future.result()
            results.append(result)
            completed += 1
            
            if completed % 50 == 0 or completed == total_configs:
                elapsed = time.time() - overall_start
                eta = (elapsed / completed) * (total_configs - completed) if completed > 0 else 0
                log(f"Progress: {completed}/{total_configs} ({100*completed/total_configs:.1f}%) | "
                    f"Elapsed: {elapsed/60:.1f}m | ETA: {eta/60:.1f}m")
    
    # Save results
    log("")
    log("💾 Saving results...")
    
    with open(OUTPUT_FILE, "w") as f:
        json.dump(results, f, indent=2)
    
    # Find best configurations
    valid_results = [r for r in results if 'error' not in r and r['total_trades'] > 0]
    
    if valid_results:
        best_pnl = max(valid_results, key=lambda x: x['total_pnl'])
        best_pf = max(valid_results, key=lambda x: x.get('profit_factor', 0))
        best_wr = max(valid_results, key=lambda x: x['win_rate'])
        
        log("")
        log("="*80)
        log("🏆 TOP CONFIGURATIONS")
        log("="*80)
        log("")
        log("Best by Total PnL:")
        log(f"  POI={best_pnl['poi_tolerance']}, TP={best_pnl['take_profit']}, SL={best_pnl['stop_loss']}")
        log(f"  PnL: ${best_pnl['total_pnl']:.2f}, Trades: {best_pnl['total_trades']}, Win Rate: {best_pnl['win_rate']:.2f}%")
        log("")
        log("Best by Profit Factor:")
        log(f"  POI={best_pf['poi_tolerance']}, TP={best_pf['take_profit']}, SL={best_pf['stop_loss']}")
        log(f"  PF: {best_pf['profit_factor']:.2f}, PnL: ${best_pf['total_pnl']:.2f}, Trades: {best_pf['total_trades']}")
        log("")
        log("Best by Win Rate:")
        log(f"  POI={best_wr['poi_tolerance']}, TP={best_wr['take_profit']}, SL={best_wr['stop_loss']}")
        log(f"  Win Rate: {best_wr['win_rate']:.2f}%, PnL: ${best_wr['total_pnl']:.2f}, Trades: {best_wr['total_trades']}")
    
    total_elapsed = time.time() - overall_start
    log("")
    log("="*80)
    log("🎉 MASSIVE PARAMETER SWEEP COMPLETED 🎉")
    log(f"Total runtime: {total_elapsed/3600:.2f} hours ({total_elapsed/60:.1f} minutes)")
    log(f"Configurations tested: {total_configs}")
    log(f"Results saved to: {OUTPUT_FILE}")
    log(f"Log saved to: {LOG_FILE}")
    log("="*80)


if __name__ == "__main__":
    main()

