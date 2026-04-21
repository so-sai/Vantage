import sqlite3
conn = sqlite3.connect('.kit/local_brain.db')

# Check all symbols and their counts
print("=== Symbol distribution ===")
rows = conn.execute("""
    SELECT 
        CASE WHEN symbol IS NULL OR symbol = '' THEN '(empty)' ELSE symbol END as sym,
        COUNT(*) as cnt
    FROM observations 
    WHERE is_active = 1 
    GROUP BY sym
    ORDER BY cnt DESC
""").fetchall()

for row in rows:
    print(f"  {row[0]}: {row[1]}")

# The issue: recall matches by symbol/namespace, not by content
# Let's see what happens when we provide entities=[]
print("\n=== Testing without entities (empty list) ===")
from kit.api import recall
r = recall(entities=[], limit=5)
print(f"Results with empty entities: {len(r)}")

# Test with query only (no entities)
r2 = recall(entities=[], limit=5, query="invariant")
print(f"Results with query 'invariant': {len(r2)}")

conn.close()