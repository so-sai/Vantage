import sqlite3

conn = sqlite3.connect('.kit/local_brain.db')
conn.row_factory = sqlite3.Row

print("=== 1. Orphan nodes check ===")
query1 = """
SELECT COUNT(*)
FROM observations o
LEFT JOIN nodes n ON o.node_id = n.id
WHERE o.node_id IS NOT NULL AND n.id IS NULL
"""
orphan_count = conn.execute(query1).fetchone()[0]
print(f"Orphan nodes: {orphan_count}")

print("\n=== 2. Duplicate structural_hash ===")
query2 = """
SELECT structural_hash, COUNT(*) as cnt
FROM observations
WHERE structural_hash IS NOT NULL AND structural_hash != ''
GROUP BY structural_hash
HAVING COUNT(*) > 1
"""
dupes = conn.execute(query2).fetchall()
print(f"Duplicate hashes: {len(dupes)}")
if dupes:
    for d in dupes[:5]:
        print(f"  {d[0][:16]}... x{d[1]}")

print("\n=== 3. Baked view consistency ===")
total_obs = conn.execute("SELECT COUNT(*) FROM observations WHERE is_active = 1").fetchone()[0]
total_baked = conn.execute("SELECT COUNT(*) FROM observations WHERE is_active = 1 AND is_baked = 1").fetchone()[0]
print(f"Total observations: {total_obs}")
print(f"Baked observations: {total_baked}")
print(f"Match: {total_obs == total_baked}")

print("\n=== 4. Total records ===")
print(f"Grand total: {total_obs}")

conn.close()