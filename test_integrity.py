import sqlite3

conn = sqlite3.connect('.kit/local_brain.db')
result = conn.execute("PRAGMA integrity_check").fetchall()
print(f"PRAGMA integrity_check: {result}")
conn.close()