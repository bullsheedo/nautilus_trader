# 🚀 Hybrid Optimization Workflow Guide

## Overview

This guide explains the **hybrid optimization workflow** that combines:
1. **Fast vectorized parameter sweep** (80x speedup) for rapid iteration
2. **Detailed backtest with tearsheet generation** for comprehensive analysis

This approach gives you the best of both worlds: speed for optimization and detail for final analysis.

---

## 📊 Workflow Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    PHASE 1: OPTIMIZATION                     │
│                  (Vectorized - 80x faster)                   │
├─────────────────────────────────────────────────────────────┤
│  • Run 27-81 parameter combinations in parallel             │
│  • Process 500K ticks in ~3-4 minutes                       │
│  • Output: JSON with all results                            │
│  • Identify best configuration by PnL                       │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│                   PHASE 2: DETAILED ANALYSIS                 │
│              (Full Engine - Comprehensive)                   │
├─────────────────────────────────────────────────────────────┤
│  • Run best config through full BacktestEngine              │
│  • Generate interactive tearsheet with charts               │
│  • Detailed performance statistics                          │
│  • Trade-by-trade analysis                                  │
│  • Proper graceful shutdown                                 │
└─────────────────────────────────────────────────────────────┘
```

---

## 🎯 Quick Start

### Option 1: Complete Workflow (Recommended)

Run the entire workflow with one command:

```bash
python scripts/run_complete_optimization.py
```

This will:
1. Run vectorized optimization (Phase 1)
2. Automatically run detailed backtest on best config (Phase 2)
3. Generate tearsheet and save all results

### Option 2: Manual Step-by-Step

**Step 1: Run vectorized optimization**
```bash
python scripts/backtest_vectorized_parallel.py
```

**Step 2: Run detailed backtest on best config**
```bash
python scripts/backtest_detailed_with_tearsheet.py \
    --from-json backtest_results_vectorized_parallel.json
```

### Option 3: Skip Optimization (Use Existing Results)

If you already have optimization results:

```bash
python scripts/run_complete_optimization.py --skip-optimization
```

---

## 📁 Scripts Overview

### 1. `backtest_vectorized_parallel.py`
**Purpose:** Fast parameter sweep using vectorized engine + multiprocessing

**Features:**
- Runs 27 parameter combinations in parallel
- 80x speedup over event-driven approach
- Outputs JSON with all results
- Memory cleanup and graceful shutdown

**Output:** `backtest_results_vectorized_parallel.json`

**Usage:**
```bash
python scripts/backtest_vectorized_parallel.py
```

---

### 2. `backtest_detailed_with_tearsheet.py`
**Purpose:** Detailed backtest with full NautilusTrader engine

**Features:**
- Full BacktestEngine with all features
- Interactive tearsheet generation
- Detailed performance statistics
- Proper graceful shutdown
- Can load best config from JSON

**Output:** `tearsheet.html` (interactive HTML report)

**Usage:**
```bash
# From JSON results
python scripts/backtest_detailed_with_tearsheet.py \
    --from-json backtest_results_vectorized_parallel.json

# Manual parameters
python scripts/backtest_detailed_with_tearsheet.py \
    --poi-tolerance 3.0 \
    --take-profit 0.30 \
    --stop-loss 0.35 \
    --output my_tearsheet.html
```

---

### 3. `run_complete_optimization.py`
**Purpose:** Master script orchestrating the complete workflow

**Features:**
- Runs both phases automatically
- Progress tracking and timing
- Error handling and cleanup
- Summary report

**Usage:**
```bash
# Full workflow
python scripts/run_complete_optimization.py

# Skip optimization phase
python scripts/run_complete_optimization.py --skip-optimization

# Custom output directory
python scripts/run_complete_optimization.py --output-dir ./results
```

---

## 📊 Performance Comparison

| Approach | Time for 81 Backtests | Use Case |
|----------|----------------------|----------|
| **Event-driven (original)** | 5.5 hours | Baseline |
| **Vectorized (single core)** | ~40 minutes | Quick tests |
| **Vectorized + Parallel (8 cores)** | **~10 minutes** | **Parameter optimization** |
| **Full engine (detailed)** | ~30 minutes | **Final analysis** |

**Hybrid workflow total:** ~13 minutes (10 min optimization + 3 min detailed)

---

## 🎨 Tearsheet Features

The generated tearsheet includes:

### Performance Metrics
- Total PnL and returns
- Win rate and profit factor
- Sharpe ratio and Sortino ratio
- Maximum drawdown
- Average win/loss

### Interactive Charts
- Equity curve over time
- Drawdown chart
- Returns distribution
- Trade analysis
- Position sizing

### Trade Analysis
- Trade-by-trade breakdown
- Entry/exit prices
- Hold times
- PnL per trade

---

## 🔧 Graceful Shutdown

All scripts now include proper cleanup:

### Vectorized Scripts
- Memory cleanup with garbage collection
- Proper exception handling
- Keyboard interrupt handling

### Detailed Backtest
- Stop all engines (trader, data, risk, exec)
- Reset engine state
- Dispose of resources
- Flush logs

---

## 💡 Best Practices

### 1. Development Workflow
```bash
# Quick iteration during development
python scripts/backtest_vectorized_parallel.py

# Review results
cat backtest_results_vectorized_parallel.json | jq '.[] | select(.total_pnl > 0)'

# Detailed analysis of best config
python scripts/backtest_detailed_with_tearsheet.py --from-json backtest_results_vectorized_parallel.json
```

### 2. Production Workflow
```bash
# Complete workflow with all features
python scripts/run_complete_optimization.py
```

### 3. Re-analysis
```bash
# Skip optimization, just regenerate tearsheet
python scripts/run_complete_optimization.py --skip-optimization
```

---

## 🚦 Next Steps

After running the workflow:

1. **Open the tearsheet** in your browser
   ```bash
   open tearsheet.html  # macOS
   xdg-open tearsheet.html  # Linux
   ```

2. **Review the results**
   - Check equity curve for consistency
   - Verify drawdown is acceptable
   - Analyze win rate and profit factor

3. **Iterate if needed**
   - Adjust parameter ranges
   - Run optimization again
   - Compare tearsheets

4. **Deploy to live trading**
   - Use the winning configuration
   - Start with paper trading
   - Monitor performance

---

## ✅ Summary

The hybrid workflow provides:

✓ **Speed:** 80x faster parameter optimization  
✓ **Detail:** Comprehensive analysis with tearsheets  
✓ **Reliability:** Proper shutdown and resource management  
✓ **Flexibility:** Run phases separately or together  
✓ **Visualization:** Interactive charts and reports  

**Total time saved:** 5+ hours per optimization run!

