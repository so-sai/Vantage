@echo off
:: Vantage Agent Shim (v1.2.3)
:: Purpose: Minimize Token Detox via Structural Signal only.
:: Usage: vantage-shim <file_path>

if "%~1"=="" (
    echo [vantage] Error: No file provided.
    echo Usage: vantage-shim ^<file_path^>
    exit /b 1
)

:: Ensure we are running from target/debug if local, or from PATH
set VANTAGE_BIN=vantage-verify
where %VANTAGE_BIN% >nul 2>&1
if errorlevel 1 (
    if exist "target\debug\vantage-verify.exe" (
        set VANTAGE_BIN=target\debug\vantage-verify.exe
    ) else if exist "target\release\vantage-verify.exe" (
        set VANTAGE_BIN=target\release\vantage-verify.exe
    ) else (
        echo [vantage] Error: vantage-verify binary not found. Please build with cargo.
        exit /b 1
    )
)

%VANTAGE_BIN% %1 --json
