param([switch]$Quiet)

# 0. Trạng thái Tuân thủ
$failed = $false

# 1. Định danh Sensor
$vantageBin = ".\target\release\vantage-verify.exe"
if (-not (Test-Path $vantageBin)) {
    $vantageBin = ".\target\debug\vantage-verify.exe"
}

# 2. Chế độ Im lặng cho Git Hook (Pháp y tốc độ cao)
if ($Quiet) {
    if (-not (Test-Path $vantageBin)) { exit 1 }
    if (-not (Test-Path ".\VANTAGE.SEAL")) { exit 1 }
    exit 0
}

Write-Host "`n[VANTAGE] COMPLIANCE DASHBOARD v1.2.3" -ForegroundColor Cyan
Write-Host "--------------------------------------------"

# 3. Kiểm tra các Trụ cột Kiến trúc
$components = @{
    "Rust 2024 Engine " = $vantageBin
    "Cognitive Brain  " = ".\.kit\brain.db"
    "Authority Seal   " = ".\VANTAGE.SEAL"
    "Agent Protocol   " = ".\AGENTS.md"
}

foreach ($name in $components.Keys) {
    if (-not (Test-Path $components[$name])) {
        Write-Host "[X] $($name): MISSING" -ForegroundColor Red
        $failed = $true
    }
    else {
        Write-Host "[OK] $($name): DETECTED" -ForegroundColor Green
    }
}

# 4. Kiểm tra Drift (Tính năng cốt lõi v1.2.3)
Write-Host "`nChecking for Structural Drift..." -ForegroundColor Yellow
& $vantageBin core/src/lib.rs --json | Out-Null
if ($LASTEXITCODE -ne 0) {
    Write-Host "[FAIL] STRUCTURAL DRIFT DETECTED!" -ForegroundColor Red
    $failed = $true
}
else {
    Write-Host "[PASS] STRUCTURAL DETERMINISM: STABLE" -ForegroundColor Green
}

if ($failed) {
    Write-Host "`n[FAIL] VANTAGE COMPLIANCE FAILED!" -ForegroundColor Red
    exit 1
}

Write-Host "[VANTAGE-SEALED] Status: READY FOR DEPLOYMENT" -ForegroundColor Cyan
exit 0
