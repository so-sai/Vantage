#!/bin/bash
# scripts/integrity_check.sh - Vantage Semantic Gatekeeper (v1.2.5)
set -e

VANTAGE_BIN="./target/release/kit-vantage"
RUST_SAMPLE="tests/golden/rust_sample.rs"
PY_SAMPLE="tests/golden/python_sample.py"

echo "[VANTAGE] 🛡️ Starting Semantic Integrity Enforcement (v1.2.5)..."

# 1. Determinism Guard
echo "[VANTAGE] Gate 1: Determinism Guard..."
$VANTAGE_BIN graph $RUST_SAMPLE > out1.json
$VANTAGE_BIN graph $RUST_SAMPLE > out2.json

if diff out1.json out2.json > /dev/null; then
    echo "[OK] Byte-identical output verified across runs."
else
    echo "[FAIL] Non-deterministic serialization detected!"
    diff out1.json out2.json
    exit 1
fi

# 2. Golden Schema Guard
echo "[VANTAGE] Gate 2: Golden Schema Guard (Rust/Python)..."

verify_golden() {
    file=$1
    golden=$2
    $VANTAGE_BIN graph $file > current.json
    if diff -w $golden current.json > /dev/null; then
        echo "[OK] Golden match: $file"
    else
        echo "[FAIL] Schema drift detected in $file!"
        diff -u $golden current.json
        rm current.json
        exit 1
    fi
    rm current.json
}

verify_golden $RUST_SAMPLE "tests/golden/rust_sample.golden.json"
verify_golden $PY_SAMPLE "tests/golden/python_sample.golden.json"

# 3. Seal & Drift Enforcement
echo "[VANTAGE] Gate 3: Seal & Drift Enforcement..."
$VANTAGE_BIN verify . --json > drift_report.json
STATUS=$(python3 -c "import json; d=json.load(open('drift_report.json')); print(d.get('status', 'unknown'))" 2>/dev/null || echo "unknown")

if [ "$STATUS" = "ok" ]; then
    echo "[OK] Seal integrity verified (0% drift)."
else
    echo "[FAIL] Semantic drift detected against VANTAGE.SEAL!"
    echo "If this change is intentional, run: kit-vantage run ."
    cat drift_report.json
    exit 1
fi

# 4. Workspace Cleanliness
echo "[VANTAGE] Gate 4: Workspace Cleanliness..."
if git diff --exit-code > /dev/null; then
    echo "[OK] Workspace is clean after audit."
else
    echo "[FAIL] Audit caused hidden mutations in the workspace!"
    exit 1
fi

rm out1.json out2.json drift_report.json
echo "[VANTAGE] ✅ SEMANTIC ENFORCEMENT PASSED."
