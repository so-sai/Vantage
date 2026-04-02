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
& $VANTAGE_BIN diff $TEST_FILE --json | Out-File -Encoding UTF8 diff_report.json

$Report = Get-Content diff_report.json | ConvertFrom-Json
$UnchangedCount = ($Report.items | Where-Object { $_.status -eq "unchanged" }).Count

if ($UnchangedCount -gt 0) {
    Write-Host "[OK] Seal integrity verified (matches baseline)." -ForegroundColor Green
} else {
    Write-Host "[FAIL] Seal drift detected! Binary/Source mismatch." -ForegroundColor Red
    # Write-Host (Get-Content diff_report.json -Raw)
    exit 1
}

Remove-Item out1.json, out2.json, diff_report.json
Write-Host "[VANTAGE] Integrity Check PASSED." -ForegroundColor Green
