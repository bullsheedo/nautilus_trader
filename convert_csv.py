import pandas as pd
from pathlib import Path
from nautilus_trader.persistence.catalog import ParquetDataCatalog
from nautilus_trader.test_kit.providers import TestInstrumentProvider
from nautilus_trader.persistence.wranglers import TradeTickDataWrangler

# Load CSV
print("Loading CSV...")
df = pd.read_csv('/root/ethusdt_sample.csv')
print(f"Loaded {len(df):,} rows")

# Set timestamp as index with mixed format
df['timestamp'] = pd.to_datetime(df['timestamp'], format='mixed')
df.set_index('timestamp', inplace=True)

# Create instrument
instrument = TestInstrumentProvider.ethusdt_perp_binance()

# Create catalog
catalog = ParquetDataCatalog('/root/nautilus_trader/catalog_new')
catalog.write_data([instrument])

# Create wrangler
wrangler = TradeTickDataWrangler(instrument=instrument)

# Process and write
print("Converting to TradeTicks...")
ticks = wrangler.process(df)
print(f"Writing {len(ticks):,} ticks to catalog...")
catalog.write_data(ticks)

print("✓ Done!")
