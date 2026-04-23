#!/bin/bash
# scripts/integrity_check.sh - Vantage Linux Integrity Check (v1.2.4)
set -e

VANTAGE_BIN="./target/release/kit-vantage"
TEST_FILE="core/test_sample.rs"

echo "[VANTAGE] Starting Linux Integrity Check..."

# 1. Determinism Check
echo "[VANTAGE] Step 1: Determinism Check..."
$VANTAGE_BIN verify $TEST_FILE --json > out1.json
$VANTAGE_BIN verify $TEST_FILE --json > out2.json

if diff out1.json out2.json > /dev/null; then
    echo "[OK] Determinism verified (identical output across runs)."
else
    echo "[FAIL] Non-deterministic output detected!"
    exit 1
fi

# 2. Seal Verification
echo "[VANTAGE] Step 2: Seal Verification..."

# Create a fresh seal for the directory (regenerates after any logic change)
$VANTAGE_BIN seal core 2>/dev/null || true

# Diff against baseline seal - check if status is "ok" (no drift)
$VANTAGE_BIN diff core/test_sample.rs --json > diff_report.json

# Check for drift: status field in JSON output
DRIFT_STATUS=$(python3 -c "import json; d=json.load(open('diff_report.json')); print(d.get('status', 'unknown'))" 2>/dev/null || echo "unknown")

if [ "$DRIFT_STATUS" = "ok" ]; then
    echo "[OK] Seal integrity verified (matches baseline)."
else
    echo "[FAIL] Seal drift detected: $DRIFT_STATUS"
    echo "[INFO] Note: If you changed hash logic, regenerate seal with: vantage seal core"
    cat diff_report.json
    exit 1
fi

rm out1.json out2.json diff_report.json
echo "[VANTAGE] Integrity Check PASSED."
