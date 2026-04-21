import sqlite3
import os

for db_name in ['brain.db', 'local_brain.db', 'memory_snapshot.db']:
    if os.path.exists(f'.kit/{db_name}'):
        conn = sqlite3.connect(f'.kit/{db_name}')
        cnt = conn.execute('SELECT COUNT(*) FROM observations WHERE is_active = 1').fetchone()[0]
        print(f"{db_name}: {cnt}")
        conn.close()