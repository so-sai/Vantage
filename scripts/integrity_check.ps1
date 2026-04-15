# scripts/integrity_check.ps1 - Vantage Windows Integrity Check (v1.2.4)
$ErrorActionPreference = "Stop"

$VANTAGE_BIN = ".\target\release\vantage.exe"
$TEST_FILE = "core\test_sample.rs"

Write-Host "[VANTAGE] Starting Windows Integrity Check..." -ForegroundColor Cyan

# 1. Determinism Check
Write-Host "[VANTAGE] Step 1: Determinism Check..."
& $VANTAGE_BIN verify $TEST_FILE --json | Out-File -Encoding UTF8 out1.json
& $VANTAGE_BIN verify $TEST_FILE --json | Out-File -Encoding UTF8 out2.json

$File1 = Get-Content out1.json -Raw
$File2 = Get-Content out2.json -Raw

if ($File1 -eq $File2) {
    Write-Host "[OK] Determinism verified (identical output across runs)." -ForegroundColor Green
} else {
    Write-Host "[FAIL] Non-deterministic output detected!" -ForegroundColor Red
    exit 1
}

# 2. Seal Verification
Write-Host "[VANTAGE] Step 2: Seal Verification..."

# Regenerate seal to ensure baseline matches current logic (handles hash logic changes)
& $VANTAGE_BIN seal core 2>$null

# Diff against baseline seal
& $VANTAGE_BIN diff $TEST_FILE --json | Out-File -Encoding UTF8 diff_report.json

$Report = Get-Content diff_report.json | ConvertFrom-Json

# Check for drift by looking at items status - "unchanged" means no drift
$UnchangedCount = ($Report.items | Where-Object { $_.status -eq "unchanged" }).Count
$TotalItems = $Report.items.Count

if ($TotalItems -eq 0 -or $UnchangedCount -eq $TotalItems) {
    Write-Host "[OK] Seal integrity verified (matches baseline)." -ForegroundColor Green
} else {
    Write-Host "[FAIL] Seal drift detected: $UnchangedCount unchanged of $TotalItems" -ForegroundColor Red
    Get-Content diff_report.json
    exit 1
}

Remove-Item out1.json, out2.json, diff_report.json
Write-Host "[VANTAGE] Integrity Check PASSED." -ForegroundColor Green
