param([switch]$Quiet)

# 0. Tr?ng th?i Tu?n th?
$failed = $false

# 1. ??nh danh Sensor
$vantageBin = ".\target\release\vantage-verify.exe"
if (-not (Test-Path $vantageBin)) {
    $vantageBin = ".\target\debug\vantage-verify.exe"
}

# 2. Ch? ?? Im l?ng cho Git Hook (Ph?p y t?c ?? cao)
if ($Quiet) {
    if (-not (Test-Path $vantageBin)) { exit 1 }
    if (-not (Test-Path ".\VANTAGE.SEAL")) { exit 1 }
    exit 0
}

Write-Host "`n???  VANTAGE COMPLIANCE DASHBOARD v1.2.3" -ForegroundColor Cyan
Write-Host "--------------------------------------------"

# 3. Ki?m tra c?c Tr? c?t Ki?n tr?c
$components = @{
    "Rust 2024 Engine " = $vantageBin
    "Cognitive Brain  " = ".\.kit\brain.db"
    "Authority Seal   " = ".\VANTAGE.SEAL"
    "Agent Protocol   " = ".\AGENTS.md"
}

foreach ($name in $components.Keys) {
    if (-not (Test-Path $components[$name])) {
        Write-Host "?? $($name): MISSING" -ForegroundColor Red
        $failed = $true
    }
    else {
        Write-Host "?? $($name): DETECTED" -ForegroundColor Green
    }
}

# 4. Ki?m tra Drift (T?nh n?ng c?t l?i v1.2.3)
Write-Host "`nChecking for Structural Drift..." -ForegroundColor Yellow
& $vantageBin core/src/lib.rs --json | Out-Null
if ($LASTEXITCODE -ne 0) {
    Write-Host "? STRUCTURAL DRIFT DETECTED!" -ForegroundColor Red
    $failed = $true
}
else {
    Write-Host "? STRUCTURAL DETERMINISM: STABLE" -ForegroundColor Green
}

if ($failed) {
    Write-Host "`n? VANTAGE COMPLIANCE FAILED!" -ForegroundColor Red
    exit 1
}

Write-Host "[VANTAGE-SEALED] Status: READY FOR DEPLOYMENT" -ForegroundColor Cyan
exit 0
