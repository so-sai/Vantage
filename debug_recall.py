import sqlite3

conn = sqlite3.connect('.kit/local_brain.db')

# Check total
total = conn.execute("SELECT COUNT(*) FROM observations WHERE is_active = 1").fetchone()[0]
print(f"Total observations: {total}")

# Check recent
rows = conn.execute("SELECT created_at, tag, substr(content, 1, 40) FROM observations WHERE is_active = 1 ORDER BY id DESC LIMIT 10").fetchall()

print("\nRecent observations:")
for row in rows:
    print(f"  {row[0]} | {row[1]} | {row[2]}...")

conn.close()