#!/usr/bin/env python3
"""
Demonstration of parallel backtesting concept using multiprocessing.

This script shows how to utilize all 8 cores by running multiple
backtests in parallel with different parameters.

This is a simplified demo that doesn't require data - it just shows
the parallelization concept and measures the speedup.
"""
import multiprocessing as mp
import time
import random
from datetime import datetime


def simulate_backtest(params):
    """
    Simulate a backtest with given parameters.
    
    In reality, this would run a full NautilusTrader backtest.
    For demo purposes, we just simulate some computation.
    """
    param_id, tp_pct, sl_pct, poi_tolerance = params
    
    # Simulate backtest computation (2-5 seconds of work)
    computation_time = random.uniform(2.0, 5.0)
    
    # Simulate processing ticks
    start = time.time()
    total = 0
    while time.time() - start < computation_time:
        # Simulate indicator calculations
        for _ in range(10000):
            total += random.random() * random.random()
    
    elapsed = time.time() - start
    
    # Simulate some results
    pnl = random.uniform(-1000, 5000)
    num_trades = random.randint(10, 100)
    win_rate = random.uniform(0.4, 0.7)
    
    return {
        "param_id": param_id,
        "tp_pct": tp_pct,
        "sl_pct": sl_pct,
        "poi_tolerance": poi_tolerance,
        "elapsed": elapsed,
        "pnl": pnl,
        "num_trades": num_trades,
        "win_rate": win_rate,
    }


def run_sequential(params_list):
    """Run backtests sequentially (1 core)."""
    print("\n🐌 Running SEQUENTIAL (1 core)...")
    start = time.time()
    results = [simulate_backtest(p) for p in params_list]
    elapsed = time.time() - start
    return results, elapsed


def run_parallel(params_list, num_processes=8):
    """Run backtests in parallel (8 cores)."""
    print(f"\n🚀 Running PARALLEL ({num_processes} cores)...")
    start = time.time()
    with mp.Pool(processes=num_processes) as pool:
        results = pool.map(simulate_backtest, params_list)
    elapsed = time.time() - start
    return results, elapsed


def main():
    print("=" * 80)
    print("Parallel Backtesting Demo - 8 Core Utilization")
    print("=" * 80)
    
    # Generate parameter combinations
    tp_values = [0.20, 0.25, 0.30, 0.35]
    sl_values = [0.20, 0.25, 0.30, 0.35]
    poi_values = [3.0, 5.0, 7.0, 10.0]
    
    params_list = []
    param_id = 0
    for tp in tp_values:
        for sl in sl_values:
            for poi in poi_values:
                params_list.append((param_id, tp, sl, poi))
                param_id += 1
    
    total_backtests = len(params_list)
    print(f"\n📊 Configuration:")
    print(f"  - Total parameter combinations: {total_backtests}")
    print(f"  - TP% values: {tp_values}")
    print(f"  - SL% values: {sl_values}")
    print(f"  - POI tolerance values: {poi_values}")
    print(f"  - Simulated backtest time: 2-5 seconds each")
    
    # Run sequential
    results_seq, time_seq = run_sequential(params_list)
    print(f"  ✓ Completed in {time_seq:.2f} seconds")
    print(f"  ✓ Average per backtest: {time_seq/total_backtests:.2f}s")
    
    # Run parallel
    num_cores = min(8, mp.cpu_count())
    results_par, time_par = run_parallel(params_list, num_cores)
    print(f"  ✓ Completed in {time_par:.2f} seconds")
    print(f"  ✓ Average per backtest: {time_par/total_backtests:.2f}s")
    
    # Calculate speedup
    speedup = time_seq / time_par
    efficiency = (speedup / num_cores) * 100
    
    print("\n" + "=" * 80)
    print("📈 Performance Results:")
    print("=" * 80)
    print(f"  Sequential time:  {time_seq:.2f}s")
    print(f"  Parallel time:    {time_par:.2f}s")
    print(f"  Speedup:          {speedup:.2f}x")
    print(f"  Efficiency:       {efficiency:.1f}%")
    print(f"  Cores used:       {num_cores}")
    
    # Show best parameters (by PnL)
    best = max(results_par, key=lambda x: x["pnl"])
    print(f"\n🏆 Best Parameters (by PnL):")
    print(f"  - TP%: {best['tp_pct']}")
    print(f"  - SL%: {best['sl_pct']}")
    print(f"  - POI Tolerance: {best['poi_tolerance']}")
    print(f"  - PnL: ${best['pnl']:.2f}")
    print(f"  - Win Rate: {best['win_rate']*100:.1f}%")
    print(f"  - Num Trades: {best['num_trades']}")
    
    print("\n" + "=" * 80)
    print("✓ Demo Complete!")
    print("=" * 80)
    print(f"\n💡 Key Takeaway:")
    print(f"   By using {num_cores} cores in parallel, you can run {speedup:.1f}x more")
    print(f"   backtests in the same time, enabling comprehensive parameter")
    print(f"   optimization that would be impractical on a single core.")
    print(f"\n🚀 Apply this to your real backtests with scripts/backtest_parallel.py")


if __name__ == "__main__":
    # Prevent multiprocessing issues on some systems
    mp.set_start_method('spawn', force=True)
    main()

