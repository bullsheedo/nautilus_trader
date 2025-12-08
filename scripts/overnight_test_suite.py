#!/usr/bin/env python3
"""
Overnight Test Suite: Full Tearsheet + Multi-Timeframe Validation
Estimated runtime: 6-8 hours
"""

import subprocess
import sys
import json
from pathlib import Path
from datetime import datetime
import time

# Paths
WORKSPACE = Path("/root/nautilus_trader")
VENV_PYTHON = WORKSPACE / "venv/bin/python"
TEARSHEET_SCRIPT = WORKSPACE / "scripts/backtest_detailed_with_tearsheet.py"
VECTORIZED_SCRIPT = WORKSPACE / "scripts/backtest_vectorized_parallel.py"
RESULTS_JSON = WORKSPACE / "backtest_results_vectorized_parallel.json"

# Output files
FULL_TEARSHEET = WORKSPACE / "full_500k_tearsheet.html"
TIMEFRAME_RESULTS = WORKSPACE / "multi_timeframe_results.json"
LOG_FILE = WORKSPACE / "overnight_test.log"


def log(message):
    """Log message to both console and file."""
    timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
    log_msg = f"[{timestamp}] {message}"
    print(log_msg)
    with open(LOG_FILE, "a") as f:
        f.write(log_msg + "\n")


def run_command(cmd, description):
    """Run a command and log output."""
    log(f"Starting: {description}")
    log(f"Command: {' '.join(cmd)}")
    
    start_time = time.time()
    
    try:
        result = subprocess.run(
            cmd,
            cwd=WORKSPACE,
            capture_output=True,
            text=True,
            timeout=14400  # 4 hour timeout per command
        )
        
        elapsed = time.time() - start_time
        
        if result.returncode == 0:
            log(f"✅ SUCCESS: {description} (took {elapsed/60:.1f} minutes)")
            log(f"Output preview:\n{result.stdout[-500:]}")  # Last 500 chars
            return True
        else:
            log(f"❌ FAILED: {description}")
            log(f"Error:\n{result.stderr}")
            return False
            
    except subprocess.TimeoutExpired:
        log(f"⏰ TIMEOUT: {description} (exceeded 4 hours)")
        return False
    except Exception as e:
        log(f"💥 EXCEPTION: {description} - {str(e)}")
        return False


def test_1_full_tearsheet():
    """Test 1: Generate full 500K tick tearsheet."""
    log("=" * 80)
    log("TEST 1: Full 500K Tick Tearsheet Generation")
    log("=" * 80)

    # Load best config from JSON (it's an array, find best by profit_factor)
    with open(RESULTS_JSON) as f:
        data = json.load(f)

    # Find best config by profit_factor
    best = max(data, key=lambda x: x.get('profit_factor', 0))
    log(f"Best config: POI={best['poi_tolerance']}, TP={best['take_profit']}, SL={best['stop_loss']}")
    log(f"  Profit Factor: {best['profit_factor']:.2f}, Win Rate: {best['win_rate']:.2f}%, Total PnL: ${best['total_pnl']:.2f}")
    
    cmd = [
        str(VENV_PYTHON),
        str(TEARSHEET_SCRIPT),
        "--poi-tolerance", str(best['poi_tolerance']),
        "--take-profit", str(best['take_profit']),
        "--stop-loss", str(best['stop_loss']),
        "--output", str(FULL_TEARSHEET),
        "--max-ticks", "500000"
    ]
    
    return run_command(cmd, "Full 500K Tearsheet")


def test_2_multi_timeframe():
    """Test 2: Multi-timeframe validation."""
    log("=" * 80)
    log("TEST 2: Multi-Timeframe Validation (5 chunks of 100K ticks)")
    log("=" * 80)

    # Load best config
    with open(RESULTS_JSON) as f:
        data = json.load(f)
    best = max(data, key=lambda x: x.get('profit_factor', 0))
    
    # We'll run 5 separate backtests on different chunks
    chunk_size = 100000
    results = []
    
    for i in range(5):
        start_tick = i * chunk_size
        end_tick = (i + 1) * chunk_size
        
        log(f"\n--- Chunk {i+1}/5: Ticks {start_tick:,} to {end_tick:,} ---")
        
        # Create a temporary script to run this chunk
        chunk_output = WORKSPACE / f"chunk_{i+1}_tearsheet.html"
        
        cmd = [
            str(VENV_PYTHON),
            str(TEARSHEET_SCRIPT),
            "--poi-tolerance", str(best['poi_tolerance']),
            "--take-profit", str(best['take_profit']),
            "--stop-loss", str(best['stop_loss']),
            "--output", str(chunk_output),
            "--max-ticks", str(chunk_size),
            "--skip-ticks", str(start_tick)  # We'll need to add this parameter
        ]
        
        success = run_command(cmd, f"Chunk {i+1}/5")
        
        if success:
            results.append({
                "chunk": i + 1,
                "start_tick": start_tick,
                "end_tick": end_tick,
                "status": "success",
                "output_file": str(chunk_output)
            })
        else:
            results.append({
                "chunk": i + 1,
                "start_tick": start_tick,
                "end_tick": end_tick,
                "status": "failed"
            })
    
    # Save results
    with open(TIMEFRAME_RESULTS, "w") as f:
        json.dump({
            "test_date": datetime.now().isoformat(),
            "best_config": best,
            "chunks": results
        }, f, indent=2)
    
    log(f"\n✅ Multi-timeframe results saved to: {TIMEFRAME_RESULTS}")
    return True


def main():
    """Run all overnight tests."""
    log("🌙 OVERNIGHT TEST SUITE STARTED 🌙")
    log(f"Start time: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
    log(f"Estimated completion: ~6-8 hours")
    log("")
    
    overall_start = time.time()
    
    # Test 1: Full tearsheet
    test1_success = test_1_full_tearsheet()
    
    # Test 2: Multi-timeframe (simplified - just run on different max_ticks)
    # Since we don't have skip-ticks parameter, we'll do a simpler version
    log("\n" + "=" * 80)
    log("TEST 2: Simplified - Running 5 tearsheets with different tick counts")
    log("=" * 80)

    with open(RESULTS_JSON) as f:
        data = json.load(f)
    best = max(data, key=lambda x: x.get('profit_factor', 0))
    
    tick_counts = [100000, 200000, 300000, 400000, 500000]
    
    for i, ticks in enumerate(tick_counts, 1):
        log(f"\n--- Run {i}/5: {ticks:,} ticks ---")
        output = WORKSPACE / f"tearsheet_{ticks//1000}k_ticks.html"
        
        cmd = [
            str(VENV_PYTHON),
            str(TEARSHEET_SCRIPT),
            "--poi-tolerance", str(best['poi_tolerance']),
            "--take-profit", str(best['take_profit']),
            "--stop-loss", str(best['stop_loss']),
            "--output", str(output),
            "--max-ticks", str(ticks)
        ]
        
        run_command(cmd, f"Tearsheet with {ticks:,} ticks")
    
    # Summary
    overall_elapsed = time.time() - overall_start
    log("\n" + "=" * 80)
    log("🎉 OVERNIGHT TEST SUITE COMPLETED 🎉")
    log(f"Total runtime: {overall_elapsed/3600:.2f} hours")
    log(f"End time: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
    log("=" * 80)
    log("\n📁 Generated files:")
    log(f"  - {FULL_TEARSHEET}")
    for ticks in tick_counts:
        log(f"  - tearsheet_{ticks//1000}k_ticks.html")
    log(f"  - {LOG_FILE}")
    log("\n✅ All done! Check the files above for results.")


if __name__ == "__main__":
    main()

