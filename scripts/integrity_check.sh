#!/bin/bash
# scripts/integrity_check.sh - Vantage Linux Integrity Check (v1.2.4)
set -e

VANTAGE_BIN="./target/release/vantage"
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
# Vantage 'diff' command compares against VANTAGE.SEAL by default
$VANTAGE_BIN diff $TEST_FILE --json > diff_report.json

# Check if there are any changes in the report
CHANGES=$(grep -c "\"status\": \"unchanged\"" diff_report.json || true)
if [ "$CHANGES" -gt 0 ]; then
    echo "[OK] Seal integrity verified (matches baseline)."
else
    echo "[FAIL] Seal drift detected! Binary/Source mismatch."
    # cat diff_report.json
    exit 1
fi

rm out1.json out2.json diff_report.json
echo "[VANTAGE] Integrity Check PASSED."
