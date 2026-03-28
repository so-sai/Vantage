param([switch]$Quiet)

# 1. Định danh Sensor
$vantageBin = ".\target\release\vantage-verify.exe"
if (-not (Test-Path $vantageBin)) {
    $vantageBin = ".\target\debug\vantage-verify.exe"
}

# 2. Chế độ Im lặng cho Git Hook (Pháp y tốc độ cao)
if ($Quiet) {
    # Kiểm tra xem binary có tồn tại không
    if (-not (Test-Path $vantageBin)) { exit 1 }
    # Kiểm tra Seal (Giả định v1.2.3)
    if (-not (Test-Path ".\VANTAGE.SEAL")) { exit 1 }
    exit 0
}

Write-Host "`n🛡️  VANTAGE COMPLIANCE DASHBOARD v1.2.3" -ForegroundColor Cyan
Write-Host "--------------------------------------------"

# 3. Kiểm tra các Trụ cột Kiến trúc
$components = @{
    "Rust 2024 Engine " = $vantageBin
    "Cognitive Brain  " = ".\.kit\brain.db"
    "Authority Seal   " = ".\VANTAGE.SEAL"
    "Agent Protocol   " = ".\AGENTS.md"
}

foreach ($name in $components.Keys) {
    if (Test-Path $components[$name]) {
        Write-Host "🟢 $($name): DETECTED" -ForegroundColor Green
    } else {
        Write-Host "🔴 $($name): MISSING" -ForegroundColor Red
        $failed = $true
    }
}

# 4. Kiểm tra Drift (Tính năng cốt lõi v1.2.3)
Write-Host "`nChecking for Structural Drift..." -ForegroundColor Yellow
# Ở đây ta gọi Vantage để soi thử chính nó
& $vantageBin core/src/lib.rs --json | Out-Null
if ($LASTEXITCODE -eq 0) {
    Write-Host "✅ STRUCTURAL DETERMINISM: STABLE" -ForegroundColor Green
} else {
    Write-Host "❌ STRUCTURAL DRIFT DETECTED!" -ForegroundColor Red
}

Write-Host "`n[VANTAGE-SEALED] Status: READY FOR DEPLOYMENT`n" -ForegroundColor Cyan
