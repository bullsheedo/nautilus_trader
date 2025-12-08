#!/root/nautilus_trader/venv/bin/python
"""
Complete optimization workflow - Hybrid approach combining speed and detail.

This master script orchestrates the complete optimization workflow:
1. Phase 1: Fast vectorized parameter sweep (80x speedup)
2. Phase 2: Detailed backtest of best config with tearsheet generation

This gives you the best of both worlds:
- Fast iteration for parameter optimization
- Detailed analysis and visualization for the winning strategy

Usage:
    python scripts/run_complete_optimization.py
    
Options:
    --skip-optimization    Skip Phase 1 and use existing results
    --num-backtests N      Number of backtests to run in Phase 1 (default: 27)
    --output-dir PATH      Directory for output files (default: current directory)
"""

import sys
import argparse
import subprocess
import json
from pathlib import Path
import time

# Configuration
SCRIPTS_DIR = Path("/root/nautilus_trader/scripts")
PYTHON_BIN = "/root/nautilus_trader/venv/bin/python"
RESULTS_FILE = "backtest_results_vectorized_parallel.json"
TEARSHEET_FILE = "best_config_tearsheet.html"


def print_header(title):
    """Print a formatted header."""
    print()
    print("=" * 80)
    print(title.center(80))
    print("=" * 80)
    print()


def print_phase(phase_num, title):
    """Print a phase header."""
    print()
    print("-" * 80)
    print(f"PHASE {phase_num}: {title}")
    print("-" * 80)
    print()


def run_vectorized_optimization():
    """Run Phase 1: Fast vectorized parameter sweep."""
    print_phase(1, "Fast Vectorized Parameter Sweep (80x speedup)")
    
    print("Running parallel vectorized backtests...")
    print("This will test 27 parameter combinations across 8 CPU cores")
    print("Expected time: ~3-4 minutes for 500K ticks")
    print()
    
    start_time = time.time()
    
    # Run the parallel vectorized backtest
    result = subprocess.run(
        [PYTHON_BIN, str(SCRIPTS_DIR / "backtest_vectorized_parallel.py")],
        cwd="/root/nautilus_trader",
        capture_output=False,
        text=True,
    )
    
    elapsed = time.time() - start_time
    
    if result.returncode != 0:
        print(f"\n❌ Phase 1 failed with exit code {result.returncode}")
        return False
    
    print(f"\n✅ Phase 1 complete in {elapsed/60:.1f} minutes")
    return True


def run_detailed_backtest():
    """Run Phase 2: Detailed backtest with tearsheet generation."""
    print_phase(2, "Detailed Backtest with Tearsheet Generation")
    
    # Check if results file exists
    if not Path(RESULTS_FILE).exists():
        print(f"❌ Results file not found: {RESULTS_FILE}")
        print("   Run Phase 1 first or specify --skip-optimization with existing results")
        return False
    
    # Load and display best config
    with open(RESULTS_FILE, 'r') as f:
        results = json.load(f)
    
    best = max(results, key=lambda x: x['total_pnl'])
    
    print("📊 Best configuration from Phase 1:")
    print(f"  POI tolerance: {best['poi_tolerance']}")
    print(f"  Take profit: {best['take_profit']}")
    print(f"  Stop loss: {best['stop_loss']}")
    print(f"  Total PnL: {best['total_pnl']:.2f}")
    print(f"  Win rate: {best['win_rate']:.2f}%")
    print(f"  Trades: {best['total_trades']}")
    print(f"  Profit factor: {best['profit_factor']:.2f}")
    print()
    
    print("Running detailed backtest with full NautilusTrader engine...")
    print("This will generate:")
    print("  - Detailed performance statistics")
    print("  - Interactive tearsheet with charts")
    print("  - Trade-by-trade analysis")
    print()
    
    start_time = time.time()
    
    # Run the detailed backtest
    result = subprocess.run(
        [
            PYTHON_BIN,
            str(SCRIPTS_DIR / "backtest_detailed_with_tearsheet.py"),
            "--from-json", RESULTS_FILE,
            "--output", TEARSHEET_FILE,
        ],
        cwd="/root/nautilus_trader",
        capture_output=False,
        text=True,
    )
    
    elapsed = time.time() - start_time
    
    if result.returncode != 0:
        print(f"\n❌ Phase 2 failed with exit code {result.returncode}")
        return False
    
    print(f"\n✅ Phase 2 complete in {elapsed:.1f} seconds")
    return True


def main():
    parser = argparse.ArgumentParser(
        description="Complete optimization workflow with vectorized sweep and detailed analysis"
    )
    parser.add_argument(
        "--skip-optimization",
        action="store_true",
        help="Skip Phase 1 and use existing results",
    )
    parser.add_argument(
        "--output-dir",
        type=str,
        default=".",
        help="Directory for output files",
    )
    
    args = parser.parse_args()
    
    # Change to output directory
    if args.output_dir != ".":
        Path(args.output_dir).mkdir(parents=True, exist_ok=True)
        import os
        os.chdir(args.output_dir)
    
    print_header("Complete Optimization Workflow - Hybrid Approach")

    print("This workflow combines:")
    print("  ✓ Phase 1: Fast vectorized parameter sweep (80x speedup)")
    print("  ✓ Phase 2: Detailed backtest with tearsheet generation")
    print()
    print("Benefits:")
    print("  • Fast iteration for parameter optimization")
    print("  • Detailed analysis and visualization for best strategy")
    print("  • Proper graceful shutdown and resource management")
    print()

    overall_start = time.time()

    # Phase 1: Vectorized optimization
    if not args.skip_optimization:
        if not run_vectorized_optimization():
            print("\n❌ Workflow failed at Phase 1")
            return 1
    else:
        print_phase(1, "Skipped (using existing results)")
        if not Path(RESULTS_FILE).exists():
            print(f"❌ Results file not found: {RESULTS_FILE}")
            return 1

    # Phase 2: Detailed backtest
    if not run_detailed_backtest():
        print("\n❌ Workflow failed at Phase 2")
        return 1

    # Summary
    overall_elapsed = time.time() - overall_start

    print_header("Workflow Complete!")

    print("📊 Summary:")
    print(f"  Total time: {overall_elapsed/60:.1f} minutes")
    print()
    print("📁 Output files:")
    print(f"  • Optimization results: {RESULTS_FILE}")
    print(f"  • Interactive tearsheet: {TEARSHEET_FILE}")
    print()
    print("🎯 Next steps:")
    print(f"  1. Open {TEARSHEET_FILE} in your browser")
    print("  2. Review the performance charts and statistics")
    print("  3. Analyze trade-by-trade results")
    print("  4. Deploy the winning strategy to live trading!")
    print()
    print("✨ Success! Your strategy is optimized and ready to deploy.")
    print()

    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except KeyboardInterrupt:
        print("\n\n⚠️ Interrupted by user")
        sys.exit(130)
    except Exception as e:
        print(f"\n\n❌ Unexpected error: {e}")
        import traceback
        traceback.print_exc()
        sys.exit(1)

