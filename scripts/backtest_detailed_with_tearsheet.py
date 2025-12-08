#!/root/nautilus_trader/venv/bin/python
"""
Detailed backtest runner with tearsheet generation.

This script takes a configuration (typically the best from vectorized optimization)
and runs it through the full NautilusTrader BacktestEngine to generate:
- Detailed performance statistics
- Interactive tearsheet with charts
- Proper graceful shutdown

Usage:
    python scripts/backtest_detailed_with_tearsheet.py --poi-tolerance 3.0 --take-profit 0.30 --stop-loss 0.35
    
Or load from vectorized results JSON:
    python scripts/backtest_detailed_with_tearsheet.py --from-json backtest_results_vectorized_parallel.json
"""

import sys
import argparse
import json
from pathlib import Path
from decimal import Decimal
import pandas as pd
from dateutil import parser as date_parser

# Add nautilus_trader to path
NAUTILUS_PATH = Path("/root/nautilus_trader")
sys.path.insert(0, str(NAUTILUS_PATH))

from nautilus_trader.backtest.engine import BacktestEngine, BacktestEngineConfig
from nautilus_trader.config import LoggingConfig
from nautilus_trader.model.currencies import USDT
from nautilus_trader.model.enums import AccountType, OmsType, AggressorSide
from nautilus_trader.model.identifiers import InstrumentId, Venue, TradeId
from nautilus_trader.model.objects import Money, Price, Quantity
from nautilus_trader.model.data import TradeTick
from nautilus_trader.test_kit.providers import TestInstrumentProvider
from nautilus_trader.examples.strategies.orderflow_strategy import (
    OrderFlowStrategy,
    OrderFlowStrategyConfig,
)

# Configuration
CSV_FILE = Path("/root/ethusdt_sample.csv")
INSTRUMENT_ID = "ETHUSDT-PERP.BINANCE"


def load_best_config_from_json(json_path):
    """Load the best configuration from vectorized results JSON."""
    with open(json_path, 'r') as f:
        results = json.load(f)
    
    # Find best result by total PnL
    best = max(results, key=lambda x: x['total_pnl'])
    
    print(f"📊 Best configuration from {json_path}:")
    print(f"  POI tolerance: {best['poi_tolerance']}")
    print(f"  Take profit: {best['take_profit']}")
    print(f"  Stop loss: {best['stop_loss']}")
    print(f"  Total PnL: {best['total_pnl']:.2f}")
    print(f"  Win rate: {best['win_rate']:.2f}%")
    print(f"  Trades: {best['total_trades']}")
    print()
    
    return {
        'poi_tolerance': best['poi_tolerance'],
        'take_profit': best['take_profit'],
        'stop_loss': best['stop_loss'],
    }


def csv_to_trade_ticks(df, instrument_id, price_precision=2, size_precision=3, max_rows=None):
    """Convert CSV DataFrame to TradeTick objects."""
    print(f"Converting to TradeTick objects...")

    if max_rows:
        df = df.head(max_rows)

    ticks = []
    instrument_id_obj = InstrumentId.from_str(instrument_id)

    for idx, row in df.iterrows():
        price = Price(float(row['price']), precision=price_precision)
        size = Quantity(float(row['quantity']), precision=size_precision)

        # Parse timestamp
        ts_str = str(row['timestamp'])
        dt = date_parser.parse(ts_str)
        ts_nanos = int(dt.timestamp() * 1_000_000_000)

        # Determine aggressor side
        aggressor_side = AggressorSide.BUYER if row['buyer_maker'] == False else AggressorSide.SELLER

        # Create trade ID
        trade_id = TradeId(f"T-{row['trade_id']}")

        tick = TradeTick(
            instrument_id=instrument_id_obj,
            price=price,
            size=size,
            aggressor_side=aggressor_side,
            trade_id=trade_id,
            ts_event=ts_nanos,
            ts_init=ts_nanos,
        )

        ticks.append(tick)

        if (idx + 1) % 50_000 == 0:
            print(f"  Converted {idx + 1:,} ticks...")

    print(f"✓ Converted {len(ticks):,} ticks")
    return ticks


def run_detailed_backtest(poi_tolerance, take_profit, stop_loss, output_path="tearsheet.html", max_ticks=100000):
    """Run a detailed backtest with full engine and generate tearsheet.

    Note: Default max_ticks=100000 to keep runtime reasonable (~2-3 minutes).
    The event-driven engine is much slower than vectorized, so we use a smaller sample.
    """

    print("=" * 80)
    print("NautilusTrader Detailed Backtest with Tearsheet Generation")
    print("=" * 80)
    print()

    # Load CSV data
    print(f"📊 Loading trade ticks from CSV: {CSV_FILE}")
    print(f"  Max ticks parameter: {max_ticks}")
    if not CSV_FILE.exists():
        print(f"❌ CSV file not found: {CSV_FILE}")
        print("   Make sure the CSV file exists")
        return

    df = pd.read_csv(CSV_FILE, nrows=max_ticks)
    print(f"✓ Loaded {len(df):,} rows from CSV")

    if max_ticks and len(df) >= max_ticks:
        print(f"  (Limited to {max_ticks:,} ticks for reasonable runtime)")

    # Convert to TradeTick objects
    ticks = csv_to_trade_ticks(df, INSTRUMENT_ID)

    if not ticks:
        print("❌ No trade ticks found!")
        return

    # Get instrument from test kit
    instrument = TestInstrumentProvider.ethusdt_perp_binance()
    print(f"✓ Created instrument: {instrument.id}")
    
    # Configure backtest engine
    config = BacktestEngineConfig(
        logging=LoggingConfig(log_level="ERROR"),  # Reduce logging spam
    )
    engine = BacktestEngine(config=config)
    
    # Add venue for Binance USDT-M Futures
    engine.add_venue(
        venue=Venue("BINANCE"),
        oms_type=OmsType.NETTING,
        account_type=AccountType.MARGIN,
        base_currency=USDT,
        starting_balances=[Money(100_000, USDT)],
        default_leverage=Decimal("20"),
        trade_execution=True,
    )
    print("✓ Added BINANCE venue (Futures: Netting, Margin, 20x leverage)")
    
    # Add instrument and data
    engine.add_instrument(instrument)
    engine.add_data(ticks)
    print("✓ Added instrument and trade tick data")
    
    # Get tick size
    tick_size = float(instrument.price_increment)
    
    # Configure strategy with provided parameters
    strategy_config = OrderFlowStrategyConfig(
        instrument_id=instrument.id,
        tick_size=tick_size,
        trade_size=Decimal("10.0"),
        poi_tolerance=poi_tolerance,
        warmup_ticks=1000,
        tp_pct=take_profit,
        sl_pct=stop_loss,
        trailing_activation_pct=0.25,
        trailing_offset_pct=0.10,
        use_emulated_orders=True,
    )
    
    strategy = OrderFlowStrategy(config=strategy_config)
    engine.add_strategy(strategy)
    print(f"✓ Added OrderFlow strategy (POI={poi_tolerance}, TP={take_profit}, SL={stop_loss})")

    # Run backtest
    print("\n🚀 Running detailed backtest...")
    print("-" * 80)
    engine.run()
    print("-" * 80)

    # Print results
    print("\n📈 Backtest Results:")
    print(f"  Total ticks processed: {strategy._tick_count:,}")
    print(f"  Total trades executed: {strategy._trade_count:,}")

    # Get backtest result
    result = engine.get_result()
    print(f"\n  Run ID: {result.run_id}")
    print(f"  Elapsed time: {result.elapsed_time:.2f}s")
    print(f"  Total events: {result.total_events:,}")
    print(f"  Total orders: {result.total_orders:,}")
    print(f"  Total positions: {result.total_positions:,}")

    # Generate tearsheet
    print("\n📊 Generating interactive tearsheet...")
    try:
        from nautilus_trader.analysis import TearsheetConfig
        from nautilus_trader.analysis.tearsheet import create_tearsheet

        tearsheet_config = TearsheetConfig(theme="plotly_white")

        create_tearsheet(
            engine=engine,
            output_path=output_path,
            config=tearsheet_config,
        )
        print(f"✓ Tearsheet saved to: {output_path}")
    except ImportError:
        print("\n⚠️ Plotly not installed. Install with: pip install plotly>=6.3.1")
    except Exception as e:
        print(f"\n⚠️ Error generating tearsheet: {e}")

    # Graceful shutdown
    print("\n🔄 Performing graceful shutdown...")

    # Stop all engines if running
    if engine.kernel.trader.is_running:
        engine.kernel.trader.stop()
        print("  ✓ Stopped trader")

    if engine.kernel.data_engine.is_running:
        engine.kernel.data_engine.stop()
        print("  ✓ Stopped data engine")

    if engine.kernel.risk_engine.is_running:
        engine.kernel.risk_engine.stop()
        print("  ✓ Stopped risk engine")

    if engine.kernel.exec_engine.is_running:
        engine.kernel.exec_engine.stop()
        print("  ✓ Stopped execution engine")

    # Reset and dispose
    engine.reset()
    print("  ✓ Reset engine")

    engine.dispose()
    print("  ✓ Disposed engine")

    print("\n✅ Detailed backtest complete with graceful shutdown!")
    print(f"📊 Open {output_path} in your browser to view the tearsheet")


def main():
    parser = argparse.ArgumentParser(description="Run detailed backtest with tearsheet generation")
    parser.add_argument("--from-json", type=str, help="Load best config from vectorized results JSON")
    parser.add_argument("--poi-tolerance", type=float, default=3.0, help="POI tolerance")
    parser.add_argument("--take-profit", type=float, default=0.30, help="Take profit percentage")
    parser.add_argument("--stop-loss", type=float, default=0.35, help="Stop loss percentage")
    parser.add_argument("--output", type=str, default="tearsheet.html", help="Output path for tearsheet")
    parser.add_argument("--max-ticks", type=int, help="Maximum number of ticks to process (for testing)")

    args = parser.parse_args()

    # Load config from JSON if provided
    if args.from_json:
        config = load_best_config_from_json(args.from_json)
        poi_tolerance = config['poi_tolerance']
        take_profit = config['take_profit']
        stop_loss = config['stop_loss']
    else:
        poi_tolerance = args.poi_tolerance
        take_profit = args.take_profit
        stop_loss = args.stop_loss

    # Run detailed backtest
    # Use default of 100K ticks if not specified (event-driven is slow!)
    max_ticks = args.max_ticks if args.max_ticks is not None else 100000

    run_detailed_backtest(
        poi_tolerance=poi_tolerance,
        take_profit=take_profit,
        stop_loss=stop_loss,
        output_path=args.output,
        max_ticks=max_ticks,
    )


if __name__ == "__main__":
    main()

