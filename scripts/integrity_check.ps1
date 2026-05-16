# scripts/integrity_check.ps1 - Vantage Semantic Gatekeeper (v1.2.5)
$ErrorActionPreference = "Stop"

$VANTAGE_BIN = ".\target\release\kit-vantage.exe"
$RUST_SAMPLE = "tests\golden\rust_sample.rs"
$PY_SAMPLE = "tests\golden\python_sample.py"

Write-Host "[VANTAGE] 🛡️ Starting Semantic Integrity Enforcement (v1.2.5)..." -ForegroundColor Cyan

# 1. Determinism Guard (Zero-Side-Effect Check)
Write-Host "[VANTAGE] Gate 1: Determinism Guard..."
$Utf8NoBom = [System.Text.UTF8Encoding]::new($false)
$out1 = & $VANTAGE_BIN graph $RUST_SAMPLE
[System.IO.File]::WriteAllText("out1.json", ($out1 -join "`n") + "`n", $Utf8NoBom)
$out2 = & $VANTAGE_BIN graph $RUST_SAMPLE
[System.IO.File]::WriteAllText("out2.json", ($out2 -join "`n") + "`n", $Utf8NoBom)

$File1 = Get-Content out1.json -Raw
$File2 = Get-Content out2.json -Raw

if ($File1 -eq $File2) {
    Write-Host "[OK] Byte-identical output verified across runs." -ForegroundColor Green
} else {
    Write-Host "[FAIL] Non-deterministic serialization detected!" -ForegroundColor Red
    # Show diff for debugging
    & git diff --no-index out1.json out2.json
    exit 1
}

# 2. Golden Schema Guard
Write-Host "[VANTAGE] Gate 2: Golden Schema Guard (Rust/Python)..."

function Verify-Golden($file, $golden) {
    $currentJson = & $VANTAGE_BIN graph $file | ConvertFrom-Json
    $expectedJson = Get-Content $golden -Raw | ConvertFrom-Json
    
    $currentStr = $currentJson | ConvertTo-Json -Depth 100
    $expectedStr = $expectedJson | ConvertTo-Json -Depth 100

    if ($currentStr.Trim() -eq $expectedStr.Trim()) {
        Write-Host "[OK] Golden match: $file" -ForegroundColor Green
    } else {
        Write-Host "[FAIL] Schema drift detected in $file!" -ForegroundColor Red
        [System.IO.File]::WriteAllText("current_drift.json", $currentStr, $Utf8NoBom)
        & git diff --no-index $golden current_drift.json
        Remove-Item current_drift.json
        exit 1
    }
}

Verify-Golden $RUST_SAMPLE "tests\golden\rust_sample.golden.json"
Verify-Golden $PY_SAMPLE "tests\golden\python_sample.golden.json"

# 3. Seal & Drift Enforcement (Hard Lockdown)
Write-Host "[VANTAGE] Gate 3: Seal & Drift Enforcement..."
# Compare workspace against committed VANTAGE.SEAL
$Report = & $VANTAGE_BIN verify . --ci | ConvertFrom-Json

if ($Report.status -eq "ok") {
    Write-Host "[OK] Seal integrity verified (0% drift)." -ForegroundColor Green
} else {
    Write-Host "[FAIL] Semantic drift detected against VANTAGE.SEAL!" -ForegroundColor Red
    Write-Host "If this change is intentional, run: kit-vantage run ." -ForegroundColor Yellow
    echo $Report | ConvertTo-Json
    exit 1
}

# 4. Workspace Cleanliness (No hidden mutations)
Write-Host "[VANTAGE] Gate 4: Workspace Cleanliness..."
$GitDiff = git diff --exit-code
if ($LASTEXITCODE -eq 0) {
    Write-Host "[OK] Workspace is clean after audit." -ForegroundColor Green
} else {
    Write-Host "[FAIL] Audit caused hidden mutations in the workspace!" -ForegroundColor Red
    exit 1
}

Remove-Item out1.json, out2.json
Write-Host "[VANTAGE] ✅ SEMANTIC ENFORCEMENT PASSED." -ForegroundColor Green
