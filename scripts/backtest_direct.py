import pyarrow.parquet as pq
from pathlib import Path

# Read one parquet file directly
file_path = Path('catalog/data/trade_tick/ETHUSDT-PERP.BINANCE/2025-06-26T03-42-33-866000000Z_2025-06-26T13-54-03-202000000Z.parquet')
table = pq.read_table(file_path)
print(f"Rows: {len(table)}")
print(f"Columns: {table.column_names}")
print(f"First 5 rows:\n{table.to_pandas().head()}")
